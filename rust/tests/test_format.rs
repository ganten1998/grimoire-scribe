// test_format.c: carrying what a newer version writes (docs/FORMAT.md). Pages this version
// writes are checked against a frozen copy of the 1.0.0-beta reader and writer (beta_format.h):
// 1.0.0-beta must open every one of them, and its re-save must lose only what FORMAT.md says.
// This version must carry all of it through a save, open newer pages read-only instead of setting
// them aside, bring back pages 1.0.0-beta set aside, and keep ini lines it does not know.
//
// Index loops are written as the C test writes them.
#![allow(clippy::needless_range_loop, clippy::manual_range_contains)]
mod beta_format;
mod testlib;
use beta_format::*;
use grimoire::prelude::*;
use std::sync::atomic::Ordering;
use std::sync::OnceLock;
use testlib::*;

static ROOT: OnceLock<String> = OnceLock::new();
fn root() -> &'static str {
    ROOT.get().map_or("", |s| s.as_str())
}
const DPI: f32 = 300.0;

fn exists(path: &str) -> bool {
    std::path::Path::new(path).exists()
}
fn unlink(path: &str) -> bool {
    std::fs::remove_file(path).is_ok()
}

// rand() and srand() are the C library's, so the fuzz is the C test's
fn srand(seed: u32) {
    // SAFETY: srand only sets the C library's generator state.
    unsafe { libc::srand(seed) }
}
fn rand() -> usize {
    // SAFETY: rand has no preconditions.
    unsafe { libc::rand() as usize }
}

// ---- building pages ----

fn wave(t: ToolKind, y: f32, seed: u32, ext: Option<&[u8; 3]>) -> Box<Stroke> {
    let mut p = ToolParams::default();
    tool_params_default(t, &mut p);
    let mut s = stroke_new(t, &p, seed);
    if let Some(e) = ext {
        s.ext = *e;
    }
    for i in 0..24 {
        let f = i as f32;
        let q = PenSample {
            x: fmla(f, 50.0, 200.0),
            y: fmla((i % 5) as f32, 9.0, y),
            pressure: 0.1 + f / 30.0,
            tilt_x: f - 12.0,
            tilt_y: fmla(-f, 0.5, 5.0),
            t_ms: i as u32 * 6,
        };
        stroke_append(&mut s, &q, DPI);
    }
    s
}

/// A chunk: tag, u32 length, payload. Appended to out; returns its size.
fn chunk(out: &mut Vec<u8>, tag: &[u8; 4], payload: &[u8]) -> usize {
    let n = payload.len();
    out.extend_from_slice(tag);
    out.extend_from_slice(&[n as u8, (n >> 8) as u8, (n >> 16) as u8, (n >> 24) as u8]);
    out.extend_from_slice(payload);
    8 + n
}

fn set_tail(p: &mut Page, d: &[u8]) {
    p.tail = d.to_vec();
}

const K_PLAIN: usize = 0;
const K_EMPTY: usize = 1;
const K_CUSTOM_MUSIC: usize = 2;
const K_PADS: usize = 3;
const K_ERASE_TEXTURE: usize = 4;
const K_SAFE_CHUNKS: usize = 5;
const K_CRITICAL_CHUNK: usize = 6;
const K_IMAGE_PAPER: usize = 7;
const K_MUSIC_NEWER: usize = 8;
const K_EMPTY_NEWER_PAPER: usize = 9;
const K_EVERYTHING: usize = 10;
const K_COUNT: usize = 11;
const K_KIND_NAME: [&str; K_COUNT] =
    ["plain", "empty", "custom music", "pad bytes", "eraser seeds", "safe chunks", "critical chunk", "image paper", "newer music part", "empty, newer paper", "everything"];

// An image paper (namespace 0x5) with the colour bits set: a later version's, which this one does
// not know (well-formed image papers are this version's own: tests/test_templates.c).
const IMAGE_PAPER: u32 = 0x5C000001;
const MUSIC_NEWER: u32 = 0x4100000C; // custom music with a part this version lacks

fn custom_layout() -> MusicLayout {
    MusicLayout { systems: 4, n: 3, part: [PART_TREBLE, PART_BASS, PART_LYRICS, PART_NONE, PART_NONE, PART_NONE] }
}

// Whether the kind's page carries data 1.0.0-beta does not keep.
fn has_new_data(k: usize) -> bool {
    k != K_PLAIN && k != K_EMPTY && k != K_CUSTOM_MUSIC
}

fn kind_paper(kind: usize) -> PaperKind {
    let m = custom_layout();
    match kind {
        K_CUSTOM_MUSIC => music_encode(&m),
        K_IMAGE_PAPER | K_EMPTY_NEWER_PAPER | K_EVERYTHING => PaperKind(IMAGE_PAPER),
        K_MUSIC_NEWER => PaperKind(MUSIC_NEWER),
        _ => PAPER_MEDIUM,
    }
}

fn make_page(kind: usize) -> Box<Page> {
    const PADS: [[u8; 3]; 3] = [[3, 1, 0x80], [0, 2, 0], [255, 255, 255]];
    let mut p = page_new(1860, 2480, kind_paper(kind));
    if kind == K_EMPTY || kind == K_EMPTY_NEWER_PAPER {
        return p;
    }
    let pad = kind == K_PADS || kind == K_EVERYTHING;
    page_add(&mut p, wave(TOOL_PENCIL, 300.0, 11, if pad { Some(&PADS[0]) } else { None }));
    page_add(&mut p, wave(TOOL_BALLPOINT, 600.0, 22, if pad { Some(&PADS[1]) } else { None }));
    page_add(&mut p, wave(TOOL_HIGHLIGHTER, 900.0, 33, if pad { Some(&PADS[2]) } else { None }));
    if kind == K_ERASE_TEXTURE || kind == K_EVERYTHING {
        // Erase strokes: a white-ink texture in seed bits 24-31 (tool + 1), and a round tip (0).
        page_add(&mut p, wave(TOOL_ERASER, 1200.0, ((TOOL_CHARCOAL.0 + 1) as u32) << 24 | 0x123456, None));
        page_add(&mut p, wave(TOOL_ERASER, 1500.0, 0x00ABCDEF, None));
    }
    let mut tail: Vec<u8> = Vec::new();
    if kind == K_SAFE_CHUNKS || kind == K_EVERYTHING {
        chunk(&mut tail, b"layr", b"\x01\x00\x02\x00base");
        chunk(&mut tail, b"pal0", b"");
        chunk(&mut tail, b"zz99", b"0123456789abcdefXYZ");
    }
    if kind == K_CRITICAL_CHUNK {
        chunk(&mut tail, b"note", b"safe first");
        chunk(&mut tail, b"LAYR", b"\x02\x00");
    }
    if !tail.is_empty() {
        set_tail(&mut p, &tail);
    }
    p
}

// ---- reading the bytes back by hand ----

fn rd32(d: &[u8]) -> u32 {
    d[0] as u32 | (d[1] as u32) << 8 | (d[2] as u32) << 16 | (d[3] as u32) << 24
}
fn rd16(d: &[u8]) -> u16 {
    (d[0] as u16) | (d[1] as u16) << 8
}

// Offsets of each stroke record, and where the strokes end. Returns the stroke count.
fn walk(d: &[u8], rec: &mut [usize], end: &mut usize) -> u32 {
    let len = d.len();
    let mut pos = if rd16(&d[4..]) == 2 { 24 } else { 20 };
    let ns = rd32(&d[pos - 4..]);
    for i in 0..ns as usize {
        if i < rec.len() {
            rec[i] = pos;
        }
        let np = rd32(&d[pos + 60..]);
        pos += 64 + np as usize * 18;
        if pos > len {
            break;
        }
    }
    *end = pos;
    ns
}

fn count_files(dir: &str, prefix: &str) -> i32 {
    let mut n = 0;
    if let Ok(d) = std::fs::read_dir(dir) {
        for e in d.flatten() {
            if e.file_name().to_string_lossy().starts_with(prefix) {
                n += 1;
            }
        }
    }
    n
}

fn slurp(path: &str) -> Option<Vec<u8>> {
    read_file(path)
}

fn same_file(path: &str, d: &[u8]) -> bool {
    slurp(path).is_some_and(|f| f == d)
}

// ---- tests ----

fn papers_newer_than_this_version() {
    check!(paper_is_known(PAPER_NARROW) && paper_is_known(PAPER_BASS7));
    let m = custom_layout();
    check!(paper_is_known(music_encode(&m)));
    check!(!paper_is_known(PaperKind(PAPER_COUNT as u32)) && !paper_is_known(PaperKind(IMAGE_PAPER)));
    check!(!paper_is_known(PaperKind(MUSIC_NEWER)) && !paper_is_known(PaperKind(PAPER_CUSTOM_FLAG)));
    check!(!paper_is_known(PaperKind(0x60000123)) && !paper_is_known(PaperKind(0xFFFFFFFF)));
    check!(paper_name(PaperKind(IMAGE_PAPER)) == "Newer paper");
    check!(paper_name(PaperKind(MUSIC_NEWER)) == "Newer paper");
    check!(paper_name(music_encode(&m)) == "Custom music");
    // Picker choices and new notebooks still need a known kind.
    check!(paper_sanitize(IMAGE_PAPER, PAPER_MEDIUM) == PAPER_MEDIUM);
    check!(paper_sanitize(PAPER_GRID.0, PAPER_MEDIUM) == PAPER_GRID);
    // A page keeps whatever paper it is given, and draws an unknown one as blank paper.
    let p = page_new(186, 248, PaperKind(IMAGE_PAPER));
    check!(p.paper.0 == IMAGE_PAPER && !p.readonly && p.tail.is_empty());
    let mut a = Canvas::default();
    let mut b = Canvas::default();
    canvas_init(&mut a, 186, 248);
    canvas_init(&mut b, 186, 248);
    canvas_fill(&mut a, Rect { x: 0, y: 0, w: 186, h: 248 }, 0);
    page_render(&mut a, &p, Rect { x: 0, y: 0, w: 186, h: 248 }, 30.0);
    paper_render(&mut b, PAPER_BLANK, Rect { x: 0, y: 0, w: 186, h: 248 }, 30.0);
    check!(a.px[..186 * 248] == b.px[..186 * 248]);
    for k in PAPER_COUNT as u32..PAPER_COUNT as u32 + 40 {
        // every unknown kind: blank
        paper_render(&mut a, PaperKind(k.wrapping_mul(0x0F0F0F1)), Rect { x: 0, y: 0, w: 186, h: 248 }, 30.0);
        if !paper_is_known(PaperKind(k.wrapping_mul(0x0F0F0F1))) {
            check!(a.px[..186 * 248] == b.px[..186 * 248]);
        }
    }
    canvas_free(&mut a);
    canvas_free(&mut b);
    drop(p);
    let s = stroke_new(TOOL_PENCIL, &ToolParams::default(), 5);
    check!(s.ext[0] == 0 && s.ext[1] == 0 && s.ext[2] == 0);
    stroke_free(s);
}

// (a) 1.0.0-beta opens every page this version writes, with every stroke; (b) its re-save keeps
// the strokes and seeds, zeroes the pad bytes, drops the chunks and turns an unknown paper into
// version 1 blank; (c) this version reads it all back and writes the same bytes again.
fn beta_opens_what_we_write() {
    for k in 0..K_COUNT {
        let p = make_page(k);
        let mut d = Vec::new();
        check!(page_serialize(&p, &mut d) == 0);
        if d.is_empty() {
            continue;
        }
        let len = d.len();
        let ver = rd16(&d[4..]);
        check!(ver == 1 || ver == 2); // never a version 1.0.0-beta sets aside
        let known = paper_is_known(p.paper);
        check!(ver as u32 == if p.paper.0 >= PAPER_COUNT as u32 { 2 } else { 1 });
        if ver == 2 {
            check!(rd16(&d[6..]) as u32 == if paper_is_custom(p.paper) { PAPER_MUSIC.0 } else { PAPER_BLANK.0 });
        }
        // (a)
        let mut bp = BetaPage::default();
        let rc = beta_page_deserialize(&d, &mut bp);
        check!(rc == 0);
        if rc == 0 {
            check!(bp.n as usize == p.strokes.len());
            check!(bp.paper == if known { p.paper.0 } else { BETA_PAPER_BLANK });
            for i in 0..(bp.n as usize).min(p.strokes.len()) {
                check!(bp.strokes[i].tool == p.strokes[i].tool.0 as u8);
                check!(bp.strokes[i].seed == p.strokes[i].seed);
                check!(bp.strokes[i].n as usize == p.strokes[i].pts.len());
            }
            beta_page_free(&mut bp);
        }
        // (b)
        let mut b2 = Vec::new();
        check!(beta_resave(&d, &mut b2) == 0);
        if !b2.is_empty() {
            let l2 = b2.len();
            check!(rd16(&b2[4..]) == if known && p.paper.0 >= PAPER_COUNT as u32 { 2 } else { 1 });
            if !known {
                check!(rd16(&b2[6..]) as u32 == BETA_PAPER_BLANK);
            }
            let mut rec = [0usize; 16];
            let mut rec0 = [0usize; 16];
            let (mut end, mut end0) = (0, 0);
            let n2 = walk(&b2, &mut rec, &mut end);
            let n0 = walk(&d, &mut rec0, &mut end0);
            check!(n2 as usize == p.strokes.len() && n0 == n2 && end == l2); // nothing after the strokes
            check!(end0 + p.tail.len() == len);
            for i in 0..(n2 as usize).min(16) {
                check!(b2[rec[i] + 1] == 0 && b2[rec[i] + 2] == 0 && b2[rec[i] + 3] == 0);
                check!(rd32(&b2[rec[i] + 4..]) == p.strokes[i].seed);
                // The rest of each record (params, points) is what we wrote.
                let rl = (if i + 1 < n2 as usize { rec[i + 1] } else { end }) - rec[i];
                check!(b2[rec[i] + 4..rec[i] + rl] == d[rec0[i] + 4..rec0[i] + rl]);
            }
            // Without new data, 1.0.0-beta writes exactly what we wrote.
            if !has_new_data(k) {
                check!(l2 == len && b2 == d);
            }
        }
        // (c)
        let mut q = None;
        check!(page_deserialize(&d, &mut q) == 0 && q.is_some());
        if let Some(q) = q {
            check!(q.readonly == (k == K_CRITICAL_CHUNK));
            check!(q.strokes.len() == p.strokes.len() && q.paper == p.paper && q.tail.len() == p.tail.len());
            for i in 0..q.strokes.len().min(p.strokes.len()) {
                check!(q.strokes[i].ext == p.strokes[i].ext);
                check!(q.strokes[i].seed == p.strokes[i].seed);
            }
            let mut d2 = Vec::new();
            if !q.readonly {
                check!(page_serialize(&q, &mut d2) == 0);
                check!(d2.len() == len && d2 == d);
                if d2.len() != len || d2 != d {
                    eprintln!("  {}: not the same bytes", K_KIND_NAME[k]);
                }
            } else {
                check!(page_serialize(&q, &mut d2) != 0); // a read-only page is never written
            }
        }
    }
}

// (h) Pages with no new data come out byte for byte as 1.0.0-beta's own writer makes them.
fn plain_pages_match_beta_writer() {
    for k in 0..K_COUNT {
        if has_new_data(k) {
            continue;
        }
        let p = make_page(k);
        // The same page built for the frozen writer, field by field.
        let mut bp = BetaPage { paper: p.paper.0, w: p.w as u32, h: p.h as u32, n: p.strokes.len() as u32, strokes: Vec::new() };
        for s in p.strokes.iter() {
            let mut b = BetaStroke { tool: s.tool.0 as u8, seed: s.seed, params: [0.0; 13], n: s.pts.len() as u32, pts: Vec::new() };
            for j in 0..13 {
                b.params[j] = s.params.get(j);
            }
            for q in s.pts.iter() {
                b.pts.push(BetaPoint { x: q.x, y: q.y, pressure: q.pressure, tilt_x: q.tilt_x, tilt_y: q.tilt_y, t_ms: q.t_ms });
            }
            bp.strokes.push(b);
        }
        let mut mine = Vec::new();
        check!(page_serialize(&p, &mut mine) == 0);
        let mut theirs = Vec::new();
        check!(beta_page_serialize(&bp, &mut theirs) == 0);
        check!(!mine.is_empty() && mine == theirs);
        beta_page_free(&mut bp);
    }
}

// Bytes for the read-only cases, made from a plain page.
const RO_VERSION3: usize = 0;
const RO_VERSION3_ODD_SIZE: usize = 1;
const RO_TOOL10: usize = 2;
const RO_CRITICAL: usize = 3;
const RO_CUT_TAIL: usize = 4;
const RO_NOT_A_CHUNK: usize = 5;
const RO_LONG_CHUNK: usize = 6;
const RO_DIGIT_TAG: usize = 7;
const RO_COUNT: usize = 8;
const K_RO_NAME: [&str; RO_COUNT] = ["version 3", "version 3, odd size", "tool 10", "critical chunk", "cut-off tail", "not a chunk", "chunk past the end", "tag starting with a digit"];

fn readonly_bytes(which: usize) -> Vec<u8> {
    let p = make_page(if which == RO_CRITICAL { K_CRITICAL_CHUNK } else { K_PLAIN });
    let mut d = Vec::new();
    page_serialize(&p, &mut d);
    drop(p);
    let mut rec = [0usize; 4];
    let mut end = 0;
    walk(&d, &mut rec, &mut end);
    match which {
        RO_VERSION3 => d[4] = 3,
        RO_VERSION3_ODD_SIZE => {
            d[4] = 3;
            d[8..12].fill(0); // w = 0: the device's page size is used
        }
        RO_TOOL10 => d[rec[1]] = 10,
        RO_CUT_TAIL => d.extend_from_slice(b"lay"),
        RO_NOT_A_CHUNK => d.extend_from_slice(b"\x00\x01\x02\x03\x00\x00\x00\x00"),
        RO_LONG_CHUNK => {
            // says 9, has 4
            chunk(&mut d, b"layr", b"abcd");
            let at = d.len() - 8;
            d[at] = 9;
        }
        RO_DIGIT_TAG => {
            chunk(&mut d, b"1lay", b"abcd");
        }
        _ => {}
    }
    d
}

fn make_notebook(name: &str, nb: &mut NotebookInfo, pages: i32) {
    check!(store_create(name, "", PAPER_MEDIUM, nb) == 0);
    for _ in 1..pages {
        let at = nb.npages();
        check!(store_add_page(nb, at, PAPER_MEDIUM) == 0);
    }
}

fn page_file(nb: &NotebookInfo, pid: u32) -> String {
    format!("{}/notebooks/{}/p{}.qpg", root(), nb.id, pid)
}

// (d) A newer page opens read-only: never set aside, never counted as damage, never rewritten.
fn newer_pages_open_readonly() {
    let mut nb = NotebookInfo::default();
    make_notebook("Newer pages", &mut nb, 2);
    let dir = format!("{}/notebooks/{}", root(), nb.id);
    let path = page_file(&nb, nb.page_ids[1]);
    for w in 0..RO_COUNT {
        let d = readonly_bytes(w);
        let len = d.len();
        if w == RO_VERSION3_ODD_SIZE {
            store_set_device(300.0, 1404, 1872); // a screen of another size
        }
        let mut p = None;
        check!(page_deserialize(&d, &mut p) == 0 && p.as_ref().is_some_and(|p| p.readonly));
        if w == RO_VERSION3_ODD_SIZE {
            store_set_device(300.0, 1860, 2480);
        }
        let Some(pg) = p else { continue };
        if !pg.readonly {
            eprintln!("  {}: not read-only", K_RO_NAME[w]);
        }
        if w == RO_VERSION3 || w == RO_VERSION3_ODD_SIZE {
            check!(pg.strokes.is_empty() && pg.paper == PAPER_BLANK); // shown blank
            // Its own size from the frozen prefix, or the screen's when that makes no sense.
            check!(if w == RO_VERSION3 { pg.w == 1860 && pg.h == 2480 } else { pg.w == 1404 && pg.h == 1872 });
        }
        if w == RO_TOOL10 {
            check!(pg.strokes.len() == 2 && pg.strokes[0].tool == TOOL_PENCIL && pg.strokes[1].tool == TOOL_HIGHLIGHTER);
        }
        if w == RO_CRITICAL || w == RO_CUT_TAIL || w == RO_NOT_A_CHUNK || w == RO_LONG_CHUNK || w == RO_DIGIT_TAG {
            check!(pg.strokes.len() == 3); // every stroke is still shown
        }
        let mut out = Vec::new();
        check!(page_serialize(&pg, &mut out) != 0 && out.is_empty());
        drop(pg);
        // Through the store, as the editor opens it.
        check!(write_file_atomic(&path, &d) == 0);
        let mut p = None;
        check!(store_load_page(&nb, 1, 1860, 2480, &mut p) == 0 && p.as_ref().is_some_and(|p| p.readonly));
        check!(store_take_corrupt_count() == 0);
        check!(same_file(&path, &d) && count_files(&dir, "p2.qpg.damaged-") == 0);
        if let Some(mut pg) = p {
            // An edit that slipped through would still never reach the file.
            page_add(&mut pg, wave(TOOL_PENCIL, 1700.0, 1, None));
            check!(store_save_page(&nb, 1, &pg) == 0);
            check!(same_file(&path, &d));
        }
        // Scaled to another screen size, still read-only, still untouched on disk.
        let mut p = None;
        check!(store_load_page(&nb, 1, 930, 1240, &mut p) == 0 && p.as_ref().is_some_and(|p| p.readonly && p.w == 930));
        drop(p);
        check!(same_file(&path, &d) && store_take_corrupt_count() == 0);
        let _ = len;
    }
    // Damage is still damage: versions no Grimoire writes, impossible sizes, cut-off strokes.
    let d = readonly_bytes(RO_COUNT); // a plain page
    let len = d.len();
    struct Bad {
        name: &'static str,
        at: i32,
        v: u8,
        cut: usize,
    }
    let bad = [
        Bad { name: "version 0", at: 4, v: 0, cut: 0 },
        Bad { name: "version 256", at: 5, v: 1, cut: 0 },
        Bad { name: "bad magic", at: 0, v: b'X', cut: 0 },
        Bad { name: "zero width", at: 8, v: 0, cut: 0 },
        Bad { name: "cut in the strokes", at: -1, v: 0, cut: 100 },
        Bad { name: "only a prefix", at: -1, v: 0, cut: 16 },
    ];
    for i in 0..bad.len() {
        let mut m = d.clone();
        let mut ml = len;
        if bad[i].at >= 0 {
            m[bad[i].at as usize] = bad[i].v;
        }
        if bad[i].name == "version 256" {
            m[4] = 0;
        }
        if bad[i].name == "zero width" {
            m[8..12].fill(0);
        }
        if bad[i].cut != 0 {
            ml = bad[i].cut;
        }
        let mut p = None;
        check!(page_deserialize(&m[..ml], &mut p) != 0 && p.is_none());
        check!(write_file_atomic(&path, &m[..ml]) == 0);
        check!(store_load_page(&nb, 1, 1860, 2480, &mut p) == 0 && p.as_ref().is_some_and(|p| p.strokes.is_empty() && !p.readonly));
        drop(p);
        check!(store_take_corrupt_count() == 1);
        check!(std::fs::metadata(&path).is_err()); // set aside, as before
        if std::fs::metadata(&path).is_ok() {
            eprintln!("  {}: not set aside", bad[i].name);
        }
    }
    check!(count_files(&dir, "p2.qpg.damaged-") >= 1);
}

// Unknown papers and the pad bytes survive the store's own paths: a save, a scaled load.
fn store_carries_new_data() {
    let mut nb = NotebookInfo::default();
    make_notebook("Carried", &mut nb, 1);
    let p = make_page(K_EVERYTHING);
    check!(store_save_page(&nb, 0, &p) == 0);
    let mut want = Vec::new();
    page_serialize(&p, &mut want);
    let mut path = page_file(&nb, nb.page_ids[0]);
    check!(same_file(&path, &want));
    // Loaded at another size (points rescaled), the extras are all still there.
    let mut q = None;
    check!(store_load_page(&nb, 0, 930, 1240, &mut q) == 0 && q.as_ref().is_some_and(|q| !q.readonly));
    if let Some(q) = q.take() {
        check!(q.paper.0 == IMAGE_PAPER && q.strokes.len() == p.strokes.len() && q.tail.len() == p.tail.len());
        check!(q.tail == p.tail);
        for i in 0..q.strokes.len().min(p.strokes.len()) {
            check!(q.strokes[i].ext == p.strokes[i].ext);
        }
    }
    // Loaded at its own size and saved after an edit: the new stroke joins, nothing is lost.
    check!(store_load_page(&nb, 0, 1860, 2480, &mut q) == 0 && q.is_some());
    if let Some(mut q) = q.take() {
        page_add(&mut q, wave(TOOL_MARKER, 2000.0, 77, None));
        check!(store_save_page(&nb, 0, &q) == 0);
    }
    check!(store_load_page(&nb, 0, 1860, 2480, &mut q) == 0 && q.is_some());
    if let Some(q) = q.take() {
        check!(q.paper.0 == IMAGE_PAPER && q.strokes.len() == p.strokes.len() + 1 && q.tail.len() == p.tail.len());
        check!(q.strokes[0].ext[0] == 3 && q.strokes[2].ext[2] == 255 && q.strokes[3].seed >> 24 == (TOOL_CHARCOAL.0 + 1) as u32);
    }
    // The notebook's default paper may be a newer one too: new pages get it, version 2.
    nb.paper = PaperKind(IMAGE_PAPER);
    check!(store_save_info(&mut nb) == 0);
    let mut back = NotebookInfo::default();
    check!(store_load_info(nb.id.as_str(), &mut back) == 0 && back.paper.0 == IMAGE_PAPER);
    let at = back.npages();
    let bp = back.paper;
    check!(store_add_page(&mut back, at, bp) == 0);
    path = page_file(&back, back.page_ids[1]);
    let d = slurp(&path);
    check!(d.as_ref().is_some_and(|d| d.len() >= 24 && rd16(&d[4..]) == 2 && rd16(&d[6..]) as u32 == PAPER_BLANK.0 && rd32(&d[16..]) == IMAGE_PAPER));
    // A missing page file shows the notebook's paper.
    check!(unlink(&path));
    check!(store_load_page(&back, 1, 1860, 2480, &mut q) == 0 && q.as_ref().is_some_and(|q| q.paper.0 == IMAGE_PAPER));
    drop(q);
    // But a new notebook only ever starts on a paper this version offers.
    let mut fresh = NotebookInfo::default();
    check!(store_create("Fresh", "", PaperKind(IMAGE_PAPER), &mut fresh) == 0 && fresh.paper == PAPER_MEDIUM);
}

// (f) Pages 1.0.0-beta set aside come back when the notebook is opened.
fn set_aside_pages_come_back() {
    let mut nb = NotebookInfo::default();
    make_notebook("Set aside", &mut nb, 4);
    let dir = format!("{}/notebooks/{}", root(), nb.id);
    // Page 1: a version 3 page; 1.0.0-beta opens it and sets it aside.
    let v3 = readonly_bytes(RO_VERSION3);
    let t10 = readonly_bytes(RO_TOOL10);
    let mut path = page_file(&nb, nb.page_ids[0]);
    check!(write_file_atomic(&path, &v3) == 0);
    check!(beta_load_page_file(&path, 1792000100) != 0);
    check!(!exists(&path) && count_files(&dir, "p1.qpg.damaged-") == 1);
    // Page 2: set aside twice (two tool-10 versions); the newer copy is the one that comes back.
    path = page_file(&nb, nb.page_ids[1]);
    check!(write_file_atomic(&path, &v3) == 0);
    check!(beta_load_page_file(&path, 1792000100) != 0);
    check!(write_file_atomic(&path, &t10) == 0);
    check!(beta_load_page_file(&path, 1792000200) != 0);
    // Page 3: its newest copy is real damage, an older one is a newer version's page.
    path = page_file(&nb, nb.page_ids[2]);
    check!(write_file_atomic(&path, &t10) == 0);
    check!(beta_load_page_file(&path, 1792000100) != 0);
    check!(write_file_atomic(&path, b"QPG1 garbage") == 0);
    check!(beta_load_page_file(&path, 1792000300) != 0);
    // Page 4: set aside, then edited in 1.0.0-beta, which wrote a fresh page: both stay.
    path = page_file(&nb, nb.page_ids[3]);
    check!(write_file_atomic(&path, &v3) == 0);
    check!(beta_load_page_file(&path, 1792000100) != 0);
    let fresh = make_page(K_PLAIN);
    let mut fd = Vec::new();
    page_serialize(&fresh, &mut fd);
    check!(write_file_atomic(&path, &fd) == 0);
    // A page that is no longer in the notebook's list stays aside, and so does real damage.
    let aside9 = format!("{dir}/p9.qpg.damaged-1792000100");
    check!(write_file_atomic(&aside9, &v3) == 0);
    let aside = format!("{dir}/p8.qpg.damaged-1792000100");
    let garbage = b"QPG1 not a page at all";
    check!(write_file_atomic(&aside, garbage) == 0);

    check!(store_recover_pages(&nb) == 3); // pages 1, 2 and 3
    path = page_file(&nb, nb.page_ids[0]);
    check!(same_file(&path, &v3) && count_files(&dir, "p1.qpg.damaged-") == 0);
    path = page_file(&nb, nb.page_ids[1]);
    check!(same_file(&path, &t10) && count_files(&dir, "p2.qpg.damaged-") == 1); // the older copy stays
    path = page_file(&nb, nb.page_ids[2]);
    check!(same_file(&path, &t10) && count_files(&dir, "p3.qpg.damaged-") == 1); // the garbage stays
    path = page_file(&nb, nb.page_ids[3]);
    check!(same_file(&path, &fd) && count_files(&dir, "p4.qpg.damaged-") == 1); // both kept
    path = format!("{dir}/p9.qpg");
    check!(!exists(&path) && same_file(&aside9, &v3));
    check!(count_files(&dir, "p8.qpg.damaged-") == 1 && exists(&aside));
    // They open read-only, as pages from a newer version do; nothing is counted as damage.
    let mut p = None;
    check!(store_load_page(&nb, 0, 1860, 2480, &mut p) == 0 && p.as_ref().is_some_and(|p| p.readonly));
    drop(p.take());
    check!(store_load_page(&nb, 1, 1860, 2480, &mut p) == 0 && p.as_ref().is_some_and(|p| p.readonly && p.strokes.len() == 2));
    drop(p);
    check!(store_take_corrupt_count() == 0);
    // Opening it again changes nothing.
    check!(store_recover_pages(&nb) == 0);
}

/// stderr goes to a file between redirect_stderr and restore_stderr (qlog writes to fd 2).
fn redirect_stderr(path: &str) -> i32 {
    let cpath = std::ffi::CString::new(path).unwrap_or_default();
    // SAFETY: dup, open, dup2 and close on descriptors this function opens or duplicates itself.
    unsafe {
        let saved = libc::dup(2);
        let fd = libc::open(cpath.as_ptr(), libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC, 0o644 as libc::c_uint);
        if fd >= 0 {
            libc::dup2(fd, 2);
            libc::close(fd);
        }
        saved
    }
}
fn restore_stderr(saved: i32) {
    if saved >= 0 {
        // SAFETY: dup2 puts back the descriptor redirect_stderr saved, and close drops that copy.
        unsafe {
            libc::dup2(saved, 2);
            libc::close(saved);
        }
    }
}

// Damaged and hostile bytes never crash the reader, and whatever it accepts as an editable page
// settles after one save: writing what it read back gives the same bytes every time after.
fn fuzz_new_paths() {
    let p = make_page(K_EVERYTHING);
    let mut d = Vec::new();
    check!(page_serialize(&p, &mut d) == 0);
    drop(p);
    if d.is_empty() {
        return;
    }
    let len = d.len();
    let mut m = vec![0u8; len + 64];
    srand(7);
    let (mut editable, mut readonly) = (0, 0);
    let failed0 = FAILURES.load(Ordering::Relaxed);
    // The reader logs every read-only page it meets, over a thousand here: that log goes to a
    // file (kept if anything fails, an ASan report included).
    let logp = format!("{}/fuzz-{}.log", tl_tmp(), std::process::id());
    eprintln!("  (the reader's log for this test: {logp})");
    let saved = redirect_stderr(&logp);
    for it in 0..4000 {
        m[..len].copy_from_slice(&d);
        let mut ml = len;
        let mode = it % 4;
        if mode == 0 {
            ml = rand() % (len + 1); // cut anywhere
        } else if mode == 1 {
            m[len - 1 - rand() % 48] = rand() as u8; // in the chunks
        } else if mode == 2 {
            m[rand() % 24] = rand() as u8; // in the header
        } else {
            for _ in 0..4 {
                m[rand() % len] = rand() as u8;
            }
            let mut k = 0;
            while k < rand() % 40 {
                m[ml] = rand() as u8; // and a tail of noise
                ml += 1;
                k += 1;
            }
        }
        let mut q = None;
        if page_deserialize(&m[..ml], &mut q) != 0 {
            continue;
        }
        let Some(qp) = q.take() else { continue };
        if qp.readonly {
            readonly += 1;
            let mut o = Vec::new();
            check!(page_serialize(&qp, &mut o) != 0);
            continue;
        }
        editable += 1;
        let mut b1 = Vec::new();
        let mut b2 = Vec::new();
        check!(page_serialize(&qp, &mut b1) == 0);
        drop(qp);
        check!(!b1.is_empty() && page_deserialize(&b1, &mut q) == 0 && q.as_ref().is_some_and(|q| !q.readonly));
        if let Some(q) = &q {
            check!(page_serialize(q, &mut b2) == 0 && b1 == b2);
        }
    }
    restore_stderr(saved);
    check!(editable > 100 && readonly > 100);
    if FAILURES.load(Ordering::Relaxed) != failed0 {
        eprintln!("  fuzz: {} checks failed, see {}", FAILURES.load(Ordering::Relaxed) - failed0, logp);
    } else {
        unlink(&logp);
    }
}

fn file_text(path: &str) -> Option<String> {
    slurp(path).map(|b| String::from_utf8_lossy(&b).into_owned())
}

// The number of lines in `text` exactly equal to `line`.
fn has_line(text: &str, line: &str) -> i32 {
    text.split('\n').filter(|l| *l == line).count() as i32
}

// (g) settings.ini: lines this version does not know survive a save, and so do values it cannot
// use as written, unless the setting is changed.
fn settings_carry_unknown_lines() {
    let path = format!("{}/settings.ini", store_root());
    let mut longv = vec![b'x'; 20000];
    longv[10] = b'=';
    let longv = String::from_utf8(longv).unwrap_or_default();
    let f = format!(
        "# Grimoire settings\r\n\
         current_tool=2\r\n\
         ink_palette=sepia\r\n\
         side_button=4\n\
         theme=dark\n\
         sleep_minutes=500\n\
         pencil.size_max_mm=20\n\
         ballpoint.pressure_gamma=2.5\n\
         pencil.color=3\n\
         white.size_min_mm=0.5\n\
         pen.rotation=1\n\
         averyveryverylongheadthatisover32chars.k=v\n\
         url=http://host/?a=b&c=d==\n\
         empty=\n\
         music_layout={}\n\
         settings_version=7\n\
         http_port=80\n\
         long={}\n",
        MUSIC_NEWER, longv
    );
    // (ink_palette: a newer version's key, in a CRLF file; side_button=4: a value this version does
    // not have; pencil.size_max_mm=20: past this version's range; pencil.color: a tool setting this
    // version lacks; white: a tool this version lacks; pen.rotation: a calibration field it lacks;
    // url: '=' inside the value; music_layout: custom music with a part it lacks; http_port=80:
    // below the range, saved as used because the launcher reads it)
    check!(write_file_atomic(&path, f.as_bytes()) == 0);
    let mut s = Settings::default();
    check!(settings_load(&mut s) == 0);
    check!(s.current_tool == TOOL_BALLPOINT && s.side_button == 3 && s.theme == 0 && s.sleep_minutes == 240);
    check_near!(s.tools[TOOL_PENCIL.0 as usize].size_max_mm, 12.0f32, 1e-4);
    check_near!(s.tools[TOOL_BALLPOINT.0 as usize].pressure_gamma, 2.5f32, 1e-4);
    check!(s.music_layout == music_default() && s.version == 7 && s.http_port == 1024);
    check!(settings_save(&s) == 0);
    let t = file_text(&path);
    check!(t.is_some());
    if let Some(t) = &t {
        check!(has_line(t, "ink_palette=sepia") == 1 && has_line(t, "pencil.color=3") == 1);
        check!(has_line(t, "white.size_min_mm=0.5") == 1 && has_line(t, "pen.rotation=1") == 1);
        check!(has_line(t, "averyveryverylongheadthatisover32chars.k=v") == 1);
        check!(has_line(t, "url=http://host/?a=b&c=d==") == 1 && has_line(t, "empty=") == 1);
        let ll = format!("long={longv}");
        check!(has_line(t, &ll) == 1);
        // Known keys this version could not use as written go back as they were.
        check!(has_line(t, "side_button=4") == 1 && has_line(t, "theme=dark") == 1);
        check!(has_line(t, "sleep_minutes=500") == 1 && has_line(t, "pencil.size_max_mm=20") == 1);
        let ml = format!("music_layout={MUSIC_NEWER}");
        check!(has_line(t, &ml) == 1);
        check!(has_line(t, "settings_version=7") == 1 && has_line(t, "current_tool=2") == 1);
        check!(has_line(t, "http_port=1024") == 1 && !t.contains("http_port=80\n"));
        check!(has_line(t, "ballpoint.pressure_gamma=2.5000") == 1);
        check!(!t.contains('\r'));
        // Known keys come first, the unknown lines after them.
        check!(t.find("ink_palette=sepia") > t.find("calligraphy.smoothing="));
    }
    // A second round trip writes the very same file.
    let mut s2 = Settings::default();
    check!(settings_load(&mut s2) == 0 && settings_save(&s2) == 0);
    let t2 = file_text(&path);
    check!(t.is_some() && t2.is_some() && t == t2);
    // A setting changed here is written as changed; the others keep their file values.
    s2.side_button = 0;
    s2.theme = 1;
    s2.tools[TOOL_PENCIL.0 as usize].size_max_mm = 0.5;
    check!(settings_save(&s2) == 0);
    let t = file_text(&path);
    check!(t.as_ref().is_some_and(|t| has_line(t, "side_button=0") == 1 && has_line(t, "theme=classic") == 1));
    check!(t.as_ref().is_some_and(|t| has_line(t, "pencil.size_max_mm=0.5000") == 1 && has_line(t, "sleep_minutes=500") == 1));
    check!(t.as_ref().is_some_and(|t| has_line(t, "ink_palette=sepia") == 1));
    // The last line for a key counts: a later value this version has forgets the kept one.
    let dup = "side_button=4\nside_button=2\ntheme=dark\ntheme=gothic\n";
    check!(write_file_atomic(&path, dup.as_bytes()) == 0);
    check!(settings_load(&mut s) == 0 && s.side_button == 2 && settings_save(&s) == 0);
    let t = file_text(&path);
    check!(t.as_ref().is_some_and(|t| has_line(t, "side_button=2") == 1 && has_line(t, "theme=gothic") == 1 && !t.contains("dark")));
    // Far more unknown text than any real file: the known keys are all written (no fixed buffer),
    // and unknown lines are kept up to a bound.
    let big = 200000;
    let mut g = String::new();
    while g.len() + 40 < big {
        g.push_str(&format!("newer.key{:06}={}\n", g.len(), "some value or other"));
    }
    g.push_str("side_button=1\n");
    check!(write_file_atomic(&path, g.as_bytes()) == 0);
    check!(settings_load(&mut s) == 0 && s.side_button == 1);
    check!(settings_save(&s) == 0);
    let t = file_text(&path);
    check!(t.as_ref().is_some_and(|t| has_line(t, "side_button=1") == 1 && has_line(t, "newer.key000000=some value or other") == 1));
    check!(t.as_ref().is_some_and(|t| t.contains("calligraphy.smoothing=") && t.len() > 65536 && t.len() < 65536 + 8192));
    // A file with no settings at all, then defaults: nothing kept from before.
    unlink(&path);
    check!(settings_load(&mut s) != 0 && settings_save(&s) == 0);
    let t = file_text(&path);
    check!(t.as_ref().is_some_and(|t| !t.contains("newer.key") && has_line(t, "side_button=2") == 1));
}

// (g) notebook.ini: lines this version does not know survive every save of the notebook.
fn notebook_ini_carries_unknown_lines() {
    let mut nb = NotebookInfo::default();
    make_notebook("Keys", &mut nb, 2);
    let path = format!("{}/notebooks/{}/notebook.ini", root(), nb.id);
    let t = file_text(&path);
    check!(t.is_some());
    let longv = "v".repeat(1000);
    let huge = "h".repeat(3000);
    let f = format!("{}page_w=1986\r\npage_h=2648\nlang=es\ntmpl=a=b=c\nlong={}\nhuge={}\nlast=1\npaper={}\n", t.as_deref().unwrap_or(""), longv, huge, IMAGE_PAPER);
    check!(write_file_atomic(&path, f.as_bytes()) == 0);
    let mut back = NotebookInfo::default();
    check!(store_load_info(nb.id.as_str(), &mut back) == 0 && back.npages() == 2 && back.paper.0 == IMAGE_PAPER);
    check!(store_save_info(&mut back) == 0);
    // Each of these writes the file from what it just read: every one must carry the lines on.
    check!(store_rename(nb.id.as_str(), "Keys, renamed") == 0); // load + save
    let t = file_text(&path);
    check!(t.as_ref().is_some_and(|t| has_line(t, "lang=es") == 1 && has_line(t, "tmpl=a=b=c") == 1 && has_line(t, "name=Keys, renamed") == 1));
    check!(store_set_folder(nb.id.as_str(), "Somewhere") == 0); // load + save
    let t = file_text(&path);
    check!(t.as_ref().is_some_and(|t| has_line(t, "lang=es") == 1 && has_line(t, "tmpl=a=b=c") == 1 && has_line(t, "folder=Somewhere") == 1));
    check!(t.as_ref().is_some_and(|t| has_line(t, "name=Keys, renamed") == 1));
    // The in-memory copy the editor would hold now: read again after those, a page added, saved.
    check!(store_load_info(nb.id.as_str(), &mut back) == 0);
    let at = back.npages();
    check!(store_add_page(&mut back, at, PAPER_GRID) == 0);
    let t = file_text(&path);
    check!(t.is_some());
    if let Some(t) = &t {
        check!(has_line(t, "page_w=1986") == 1 && has_line(t, "page_h=2648") == 1 && has_line(t, "lang=es") == 1);
        check!(has_line(t, "tmpl=a=b=c") == 1 && has_line(t, "last=1") == 1);
        let ll = format!("long={longv}");
        check!(has_line(t, &ll) == 1);
        check!(!t.contains("huge=")); // too long to keep: left out, the rest is intact
        let pl = format!("paper={IMAGE_PAPER}");
        check!(has_line(t, &pl) == 1);
        check!(!t.contains('\r'));
    }
    let mut again = NotebookInfo::default();
    check!(store_load_info(nb.id.as_str(), &mut again) == 0 && again.npages() == 3 && again.name == "Keys, renamed" && again.folder == "Somewhere");
    // And deleting a page saves them on as well.
    check!(store_delete_page(&mut again, 2) == 0 && again.npages() == 2);
    let t = file_text(&path);
    check!(t.as_ref().is_some_and(|t| has_line(t, "lang=es") == 1 && has_line(t, "tmpl=a=b=c") == 1 && has_line(t, "last=1") == 1));
}

// Names longer than the 95 bytes notebook.ini keeps (1.0.0-beta's limit too, docs/FORMAT.md) are
// cut at the start of a character, never inside one: the cut name is written back, and becomes
// the mirror PDF's file name and the share pages' text.
fn valid_utf8(s: &[u8]) -> bool {
    std::str::from_utf8(s).is_ok()
}

fn long_names_cut_at_a_character() {
    let mut nb = NotebookInfo::default();
    let mut back = NotebookInfo::default();
    make_notebook("Short", &mut nb, 1);
    // 93 ASCII bytes, then n with a tilde (2 bytes) across the 95-byte limit, then more.
    let name = format!("{}ññ tail", "a".repeat(93));
    check!(store_rename(nb.id.as_str(), &name) == 0);
    check!(store_load_info(nb.id.as_str(), &mut back) == 0);
    check!(back.name.len() == 93 + 2 && valid_utf8(back.name.as_bytes()) && &back.name.as_bytes()[93..95] == "ñ".as_bytes());
    // The limit inside the second byte of a 2-byte letter: that letter goes.
    let name = format!("{}ñ tail", "b".repeat(94));
    check!(store_rename(nb.id.as_str(), &name) == 0);
    check!(store_load_info(nb.id.as_str(), &mut back) == 0);
    check!(back.name.len() == 94 && valid_utf8(back.name.as_bytes()) && back.name.as_bytes()[93] == b'b');
    // A 4-byte character (an emoji) across the limit, in a folder name.
    let name = format!("{}\u{1F3B5} tail", "c".repeat(92));
    check!(store_set_folder(nb.id.as_str(), &name) == 0);
    check!(store_load_info(nb.id.as_str(), &mut back) == 0);
    check!(back.folder.len() == 92 && valid_utf8(back.folder.as_bytes()));
    // folders.ini's lines the same.
    check!(store_add_folder(&name) == 0);
    let mut fs: Vec<String> = Vec::new();
    check!(store_list_folders(&mut fs) == 0);
    let mut all_valid = !fs.is_empty();
    for f in &fs {
        all_valid &= valid_utf8(f.as_bytes()) && f.len() <= 95;
    }
    check!(all_valid);
    // A name that fits is kept byte for byte, accents and all.
    let fits = "Música española €";
    check!(store_rename(nb.id.as_str(), fits) == 0 && store_load_info(nb.id.as_str(), &mut back) == 0 && back.name == fits);
    check!(store_set_folder(nb.id.as_str(), "") == 0);
}

// A value kept from settings.ini goes back only while the setting is untouched: once a save finds
// it changed, it is saved as set from then on, also when it is set back to the value used.
fn changed_settings_stay_changed() {
    let path = format!("{}/settings.ini", store_root());
    let f = format!("side_button=4\ntheme=dark\npencil.size_max_mm=20\nmusic_layout={MUSIC_NEWER}\nsleep_minutes=500\n");
    check!(write_file_atomic(&path, f.as_bytes()) == 0);
    let mut s = Settings::default();
    check!(settings_load(&mut s) == 0 && s.side_button == 3 && s.theme == 0 && s.music_layout == music_default());
    // Changed (every settings change is saved as it is made), then set back to what was used.
    let m = custom_layout();
    s.side_button = 0;
    s.theme = 1;
    s.tools[TOOL_PENCIL.0 as usize].size_max_mm = 5.0;
    s.music_layout = music_encode(&m);
    check!(settings_save(&s) == 0);
    s.side_button = 3;
    s.theme = 0;
    s.tools[TOOL_PENCIL.0 as usize].size_max_mm = 12.0;
    s.music_layout = music_default();
    check!(settings_save(&s) == 0);
    let ml = format!("music_layout={}", music_default().0);
    for _round in 0..2 {
        // and the same again on the next save
        let t = file_text(&path);
        check!(t.as_ref().is_some_and(|t| has_line(t, "side_button=3") == 1 && has_line(t, "theme=gothic") == 1));
        check!(t.as_ref().is_some_and(|t| has_line(t, "pencil.size_max_mm=12.0000") == 1 && has_line(t, &ml) == 1));
        check!(t.as_ref().is_some_and(|t| has_line(t, "sleep_minutes=500") == 1)); // the setting nobody changed
        check!(settings_save(&s) == 0);
    }
}

// A 0/1 setting holding another value (a newer version's third state) is used as on or off and
// written back as it was until the setting changes; calibration's valid and swap the same.
fn bool_settings_carry_other_values() {
    let path = format!("{}/settings.ini", store_root());
    let f = "invert=2\nmirror_pdf=2\nfollow_rotation=2\npinch_zoom=auto\npanel_collapsed=2\n\
             passcode_told=2\npen.valid=2\npen.swap=1\ntouch.valid=0\ntouch.swap=2\n";
    check!(write_file_atomic(&path, f.as_bytes()) == 0);
    let mut s = Settings::default();
    check!(settings_load(&mut s) == 0);
    check!(s.invert && s.mirror_pdf && s.follow_rotation && !s.pinch_zoom && s.panel_collapsed && s.passcode_told);
    check!(s.pen_calib.valid && s.pen_calib.swap_xy && !s.touch_calib.valid && s.touch_calib.swap_xy);
    for _round in 0..2 {
        // kept on every save while unchanged
        check!(settings_save(&s) == 0);
        let t = file_text(&path);
        check!(t.as_ref().is_some_and(|t| has_line(t, "invert=2") == 1 && has_line(t, "mirror_pdf=2") == 1));
        check!(t.as_ref().is_some_and(|t| has_line(t, "follow_rotation=2") == 1 && has_line(t, "pinch_zoom=auto") == 1));
        check!(t.as_ref().is_some_and(|t| has_line(t, "panel_collapsed=2") == 1 && has_line(t, "passcode_told=2") == 1));
        check!(t.as_ref().is_some_and(|t| has_line(t, "pen.valid=2") == 1 && has_line(t, "pen.swap=1") == 1));
        check!(t.as_ref().is_some_and(|t| has_line(t, "touch.valid=0") == 1 && has_line(t, "touch.swap=2") == 1));
    }
    // Changed here: saved as set from then on, also when set back.
    s.invert = false;
    s.pen_calib.valid = false;
    check!(settings_save(&s) == 0);
    s.invert = true;
    s.pen_calib.valid = true;
    check!(settings_save(&s) == 0);
    let t = file_text(&path);
    check!(t.as_ref().is_some_and(|t| has_line(t, "invert=1") == 1 && has_line(t, "pen.valid=1") == 1 && has_line(t, "mirror_pdf=2") == 1));
    unlink(&path);
}

fn put_page_list_aside(dir: &str) {
    let ini = format!("{dir}/notebook.ini");
    let bare = "# Grimoire notebook\nname=Rebuilt\npaper=1\n"; // no page list, no next id
    check!(write_file_atomic(&ini, bare.as_bytes()) == 0);
}

// A set-aside copy of a page the notebook no longer lists stays aside, where no new page can take
// its number and write over it; and a new page never writes over a page file that is there.
fn recovery_never_loses_a_page() {
    let mut nb = NotebookInfo::default();
    make_notebook("Rebuilt", &mut nb, 3);
    let dir = format!("{}/notebooks/{}", root(), nb.id);
    let v3 = readonly_bytes(RO_VERSION3);
    // Page 4 was set aside by 1.0.0-beta, then notebook.ini lost its page list.
    let aside = format!("{dir}/p4.qpg.damaged-1792000000");
    check!(write_file_atomic(&aside, &v3) == 0);
    put_page_list_aside(&dir);
    let mut back = NotebookInfo::default();
    check!(store_load_info(nb.id.as_str(), &mut back) == 0 && back.npages() == 3 && back.next_page_id == 4); // rebuilt
    check!(store_recover_pages(&back) == 0);
    let mut path = format!("{dir}/p4.qpg");
    check!(!exists(&path) && same_file(&aside, &v3));
    // The next new page takes number 4; the copy is still there as it was, and then kept beside it.
    let at = back.npages();
    check!(store_add_page(&mut back, at, PAPER_GRID) == 0 && back.page_ids[3] == 4);
    check!(same_file(&aside, &v3));
    check!(store_recover_pages(&back) == 0 && same_file(&aside, &v3) && exists(&path));
    // A page file at the next number that the page list does not name is never written over.
    let taken = back.next_page_id;
    path = format!("{dir}/p{taken}.qpg");
    check!(write_file_atomic(&path, &v3) == 0);
    check!(store_add_page(&mut back, 0, PAPER_GRID) == 0);
    check!(back.page_ids[0] == taken + 1 && back.next_page_id == taken + 2 && same_file(&path, &v3));
    let mut again = NotebookInfo::default();
    check!(store_load_info(nb.id.as_str(), &mut again) == 0 && again.npages() == 5 && again.page_ids[0] == taken + 1);
}

// The log, for a while: stderr goes to a file between log_begin and log_end (which returns it).
fn log_path() -> String {
    format!("{}/log-{}.txt", tl_tmp(), std::process::id())
}

fn log_begin() -> i32 {
    redirect_stderr(&log_path())
}

fn log_end(saved: i32) -> Option<String> {
    restore_stderr(saved);
    let t = file_text(&log_path());
    unlink(&log_path());
    t
}

fn count_text(text: &str, what: &str) -> i32 {
    text.matches(what).count() as i32
}

// What stays as it is (a set-aside copy beside its page or of a page no longer listed, notebook.ini
// lines that do not fit) is logged once, not on every opening or listing; checking the copies
// logs no read-only reasons of its own.
fn lasting_states_logged_once() {
    let mut nb = NotebookInfo::default();
    make_notebook("Logged once", &mut nb, 2);
    let dir = format!("{}/notebooks/{}", root(), nb.id);
    let t10 = readonly_bytes(RO_TOOL10);
    let mut aside = format!("{dir}/p2.qpg.damaged-1792000000");
    check!(write_file_atomic(&aside, &t10) == 0); // p2.qpg is there too: both kept
    aside = format!("{dir}/p7.qpg.damaged-1792000000");
    check!(write_file_atomic(&aside, &t10) == 0); // not in the page list
    let ini = format!("{dir}/notebook.ini");
    let mut f = file_text(&ini).unwrap_or_default();
    for i in 0..40 {
        f.push_str(&format!("newer{:02}={:0100}\n", i, i)); // 4 KB
    }
    check!(write_file_atomic(&ini, f.as_bytes()) == 0);
    let mut ok = 0;
    let saved = log_begin();
    for _ in 0..3 {
        let mut back = NotebookInfo::default();
        ok += (store_load_info(nb.id.as_str(), &mut back) == 0 && back.npages() == 2) as i32;
        ok += (store_recover_pages(&back) == 0) as i32;
    }
    let log = log_end(saved);
    check!(ok == 6 && log.is_some());
    let l = log.as_deref().unwrap_or("");
    check!(count_text(l, "both kept") == 1 && count_text(l, "not in the page list") == 1);
    check!(count_text(l, "do not fit") == 1 && count_text(l, "read-only page") == 0);
    if log.is_some() && (count_text(l, "both kept") != 1 || count_text(l, "do not fit") != 1) {
        eprint!("{l}");
    }
}

// A stroke parameter past its range, as a later version that widened one would write it: this
// version and 1.0.0-beta both read it at the range's end, and both save the same narrowed bytes.
fn params_past_their_ranges() {
    let mut prm = ToolParams::default();
    tool_params_default(TOOL_PENCIL, &mut prm);
    prm.size_min_mm = 0.01;
    prm.size_max_mm = 50.0;
    prm.pressure_gamma = 0.0;
    prm.tilt_max_deg = -3.0;
    prm.smoothing = 7.0;
    let mut s = stroke_new(TOOL_PENCIL, &prm, 9);
    for i in 0..12 {
        let q = PenSample { x: fmla(i as f32, 40.0, 300.0), y: 500.0, pressure: 0.5, tilt_x: 0.0, tilt_y: 0.0, t_ms: i as u32 * 7 };
        stroke_append(&mut s, &q, DPI);
    }
    let mut p = page_new(1860, 2480, PAPER_MEDIUM);
    page_add(&mut p, s);
    let mut d = Vec::new();
    let mut mine = Vec::new();
    check!(page_serialize(&p, &mut d) == 0);
    let mut q = None;
    check!(page_deserialize(&d, &mut q) == 0 && q.as_ref().is_some_and(|q| !q.readonly && q.strokes.len() == 1));
    if let Some(q) = &q {
        if q.strokes.len() == 1 {
            let r = &q.strokes[0].params;
            check!(r.size_min_mm == 0.10 && r.size_max_mm == 12.0 && r.pressure_gamma == 0.25);
            check!(r.tilt_max_deg == 5.0 && r.smoothing == 1.0 && r.opacity_max == prm.opacity_max);
            check!(page_serialize(q, &mut mine) == 0);
        }
    }
    let mut theirs = Vec::new();
    check!(beta_resave(&d, &mut theirs) == 0);
    check!(!mine.is_empty() && mine == theirs);
    check!(!mine.is_empty() && mine.len() == d.len() && mine != d); // narrowed for good
}

#[test]
fn test_format() {
    let r = format!("{}/format-{}", tl_tmp(), std::process::id());
    let _ = ROOT.set(r.clone());
    let _ = std::fs::remove_dir_all(&r);
    check!(store_init(&r) == 0);
    run!(papers_newer_than_this_version);
    run!(beta_opens_what_we_write);
    run!(plain_pages_match_beta_writer);
    run!(newer_pages_open_readonly);
    run!(store_carries_new_data);
    run!(set_aside_pages_come_back);
    run!(fuzz_new_paths);
    run!(settings_carry_unknown_lines);
    run!(notebook_ini_carries_unknown_lines);
    run!(changed_settings_stay_changed);
    run!(bool_settings_carry_other_values);
    run!(long_names_cut_at_a_character);
    run!(recovery_never_loses_a_page);
    run!(lasting_states_logged_once);
    run!(params_past_their_ranges);
    let _ = std::fs::remove_dir_all(&r);
    done();
}
