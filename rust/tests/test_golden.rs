// test_golden.c: renders that must never change. Every paper 1.0.0-beta knows and a stroke of
// every tool, rendered at the Scribe's 1860x2480 and 300 dpi, checked against CRC32s recorded
// from the 1.0.0-beta sources (this table was made at the base of 1.0.0-beta.2, whose src/ was
// the released 1.0.0-beta's). A page someone already wrote must look exactly the same in every
// later version: if one of these fails, old pages changed. Never re-record the table to make a
// change pass; fix the change. (1.2.0's vertical highlighter tip drew old strokes with the new tip
// until 2.0.0-beta.5 recorded the tip on each stroke, issue #5: old strokes match this table
// again, and the vertical tip's own renders are listed separately below.)
//
// GRIMOIRE_GOLDEN_PRINT=1 prints the table instead of checking it (that is how it was made).
// The values hold for the host build this suite runs on (clang on arm64 macOS): another
// compiler or CPU may round floats differently, so there the comparison is skipped.
//
// The Rust build is checked against the C's recorded values: these are the arbiter of the render
// path (PORTING.md, Tests). Rust's arm64 macOS build contracts a * b + c as clang does (fmla).
#![allow(clippy::needless_range_loop)]
mod testlib;
use grimoire::prelude::*;
use std::sync::OnceLock;
use testlib::*;

const DPI: f32 = 300.0;
const W: i32 = 1860;
const H: i32 = 2480;
/// the fixed kinds 1.0.0-beta knows: PAPER_NARROW..PAPER_BASS7
const BETA_PAPER_COUNT: i32 = 17;
/// the tool ids 1.0.0-beta knows: TOOL_PENCIL..TOOL_CALLIGRAPHY
const BETA_TOOL_COUNT: usize = 10;

const GOLDEN_HOST: bool = cfg!(all(target_arch = "aarch64", target_os = "macos"));

// A stroke stores its settings as 13 floats (docs/FORMAT.md): ToolParams never changes size.
const _: () = assert!(std::mem::size_of::<ToolParams>() == 13 * std::mem::size_of::<f32>(), "ToolParams is frozen at 13 floats");

// ToolParams from its 13 floats, in field order.
fn tp(v: [f32; 13]) -> ToolParams {
    let mut p = ToolParams::default();
    for (i, x) in v.iter().enumerate() {
        *p.get_mut(i) = *x;
    }
    p
}

// 1.0.0-beta's default settings for each tool (src/brush.c, tool_params_default), frozen here:
// defaults may change in a later version, but a stroke keeps the snapshot it was drawn with.
fn k_beta_params() -> [ToolParams; BETA_TOOL_COUNT] {
    [
    tp([0.20, 0.45, 0.30, 0.95, 0.03, 0.85, 1.25, 15.0, 60.0, 1.6, 0.45, 0.65, 0.10]),  // pencil
    tp([0.26, 0.36, 0.38, 0.85, 0.03, 0.80, 1.10, 25.0, 70.0, 0.35, 0.10, 0.50, 0.15]),  // mechanical
    tp([0.30, 0.48, 0.85, 1.00, 0.02, 0.55, 0.60, 30.0, 75.0, 0.10, 0.00, 0.06, 0.35]),  // ballpoint
    tp([2.5, 7.0, 1.0, 1.0, 0.0, 0.60, 1.00, 30.0, 75.0, 0.0, 0.0, 0.0, 0.0]),  // eraser
    tp([4.0, 5.0, 0.55, 0.65, 0.02, 0.50, 0.80, 30.0, 70.0, 0.2, 0.0, 0.0, 0.40]),  // highlighter
    tp([0.35, 1.10, 0.90, 1.00, 0.02, 0.60, 0.80, 30.0, 75.0, 0.0, 0.0, 0.0, 0.30]),  // fountain
    tp([0.28, 0.34, 1.00, 1.00, 0.01, 0.30, 1.00, 30.0, 75.0, 0.0, 0.0, 0.0, 0.30]),  // fineliner
    tp([1.50, 2.00, 0.92, 1.00, 0.01, 0.40, 0.70, 30.0, 75.0, 0.3, 0.0, 0.04, 0.30]),  // marker
    tp([0.80, 2.40, 0.30, 1.00, 0.03, 0.85, 1.00, 45.0, 75.0, 2.0, 0.35, 0.85, 0.15]),  // charcoal
    tp([0.70, 1.40, 0.92, 1.00, 0.02, 0.60, 0.80, 10.0, 25.0, 0.0, 0.0, 0.0, 0.30]),  // calligraphy
    ]
}

// Three custom music layouts, as stored on disk (music_encode must still produce these).
fn k_music() -> [(MusicLayout, u32); 3] {
    [
        (MusicLayout { systems: 3, n: 5, part: [PART_TREBLE, PART_BASS, PART_TAB6, PART_LYRICS, PART_TAB4, PART_NONE] }, 0x4307b921),
        (MusicLayout { systems: 2, n: 4, part: [PART_ALTO, PART_TENOR, PART_PERC, PART_TAB7, PART_NONE, PART_NONE] }, 0x4200a543),
        (MusicLayout { systems: 6, n: 3, part: [PART_LYRICS, PART_TREBLE, PART_BASS, PART_NONE, PART_NONE, PART_NONE] }, 0x4600021b),
    ]
}

// Recorded from the 1.0.0-beta sources on 2026-10-19 (GRIMOIRE_GOLDEN_PRINT=1).
const K_GOLDEN: [(&str, u32); 90] = [
    ("paper 0", 0x57638e39),
    ("paper 1", 0x98cd1286),
    ("paper 2", 0x1b0f66e6),
    ("paper 3", 0x68b3566c),
    ("paper 4", 0xe08d3ace),
    ("paper 5", 0x5fa6d50c),
    ("paper 6", 0xbda74f6d),
    ("paper 7", 0x6ecc544c),
    ("paper 8", 0x0531cc67),
    ("paper 9", 0x91a5d1f7),
    ("paper 10", 0x2144d948),
    ("paper 11", 0xca11ef8b),
    ("paper 12", 0x1dd6d7a4),
    ("paper 13", 0xa631f68b),
    ("paper 14", 0xb57c5a8b),
    ("paper 15", 0xfd5d47f4),
    ("paper 16", 0x13fa2364),
    ("custom music 0", 0xa34002e2),
    ("custom music 1", 0x26ce7de9),
    ("custom music 2", 0xb9a665aa),
    ("medium rule, pencil", 0x62d3bb3a),
    ("medium rule, pencil, ink", 0x0cd4f241),
    ("medium rule, pencil, paper", 0x98cd1286),
    ("medium rule, mechanical pencil", 0x897d012f),
    ("medium rule, mechanical pencil, ink", 0x8f2475bf),
    ("medium rule, mechanical pencil, paper", 0x98cd1286),
    ("medium rule, ballpoint", 0x92c42d6d),
    ("medium rule, ballpoint, ink", 0xee1629ce),
    ("medium rule, ballpoint, paper", 0x98cd1286),
    ("medium rule, eraser", 0x98cd1286),
    ("medium rule, eraser, ink", 0xe08d3ace),
    ("medium rule, eraser, paper", 0x98cd1286),
    ("medium rule, highlighter", 0x6eda0d5b),
    ("medium rule, highlighter, ink", 0x4c159364),
    ("medium rule, highlighter, paper", 0x98cd1286),
    ("medium rule, fountain pen", 0x04ef983b),
    ("medium rule, fountain pen, ink", 0x772a3f9b),
    ("medium rule, fountain pen, paper", 0x98cd1286),
    ("medium rule, fineliner", 0x431d1354),
    ("medium rule, fineliner, ink", 0xd177bf82),
    ("medium rule, fineliner, paper", 0x98cd1286),
    ("medium rule, marker", 0xefcaee3e),
    ("medium rule, marker, ink", 0xb203d91f),
    ("medium rule, marker, paper", 0x98cd1286),
    ("medium rule, charcoal", 0xb65bb58a),
    ("medium rule, charcoal, ink", 0xddce7929),
    ("medium rule, charcoal, paper", 0x98cd1286),
    ("medium rule, calligraphy pen", 0x3143fdc1),
    ("medium rule, calligraphy pen, ink", 0xa42da566),
    ("medium rule, calligraphy pen, paper", 0x98cd1286),
    ("grand staff, pencil", 0xe78a9cd7),
    ("grand staff, pencil, ink", 0x0cd4f241),
    ("grand staff, pencil, paper", 0x0531cc67),
    ("grand staff, mechanical pencil", 0xb96ee1d4),
    ("grand staff, mechanical pencil, ink", 0x8f2475bf),
    ("grand staff, mechanical pencil, paper", 0x0531cc67),
    ("grand staff, ballpoint", 0x3679b20a),
    ("grand staff, ballpoint, ink", 0xee1629ce),
    ("grand staff, ballpoint, paper", 0x0531cc67),
    ("grand staff, eraser", 0x0531cc67),
    ("grand staff, eraser, ink", 0xe08d3ace),
    ("grand staff, eraser, paper", 0x0531cc67),
    ("grand staff, highlighter", 0x665532fe),
    ("grand staff, highlighter, ink", 0x4c159364),
    ("grand staff, highlighter, paper", 0x0531cc67),
    ("grand staff, fountain pen", 0xc6e3a133),
    ("grand staff, fountain pen, ink", 0x772a3f9b),
    ("grand staff, fountain pen, paper", 0x0531cc67),
    ("grand staff, fineliner", 0xc24dc4ed),
    ("grand staff, fineliner, ink", 0xd177bf82),
    ("grand staff, fineliner, paper", 0x0531cc67),
    ("grand staff, marker", 0x3d9d8de1),
    ("grand staff, marker, ink", 0xb203d91f),
    ("grand staff, marker, paper", 0x0531cc67),
    ("grand staff, charcoal", 0xa5643825),
    ("grand staff, charcoal, ink", 0xddce7929),
    ("grand staff, charcoal, paper", 0x0531cc67),
    ("grand staff, calligraphy pen", 0x433efb0d),
    ("grand staff, calligraphy pen, ink", 0xa42da566),
    ("grand staff, calligraphy pen, paper", 0x0531cc67),
    ("medium rule, every tool", 0x1d589bf4),
    ("medium rule, every tool, ink", 0xe653463a),
    ("medium rule, every tool, paper", 0x98cd1286),
    ("medium rule, every tool, file of 26678 bytes", 0x1ab5b75f),
    ("medium rule, every tool, read back", 0xb91c7fea),
    ("grand staff, every tool", 0x23997dea),
    ("grand staff, every tool, ink", 0xe653463a),
    ("grand staff, every tool, paper", 0x0531cc67),
    ("grand staff, every tool, file of 26678 bytes", 0x4bc35d72),
    ("grand staff, every tool, read back", 0x09bc38eb),
];

const K_VERTICAL_TIP: [(&str, u32); 6] = [
    ("medium rule, highlighter, vertical tip", 0xe1df8dfc),
    ("medium rule, highlighter, vertical tip, ink", 0xe83cb220),
    ("medium rule, highlighter, vertical tip, paper", 0x98cd1286),
    ("grand staff, highlighter, vertical tip", 0xb2bc9b09),
    ("grand staff, highlighter, vertical tip, ink", 0xe83cb220),
    ("grand staff, highlighter, vertical tip, paper", 0x0531cc67),
];

fn crc32_bytes(crc: u32, p: &[u8]) -> u32 {
    static CRC_TABLE: OnceLock<[u32; 256]> = OnceLock::new();
    let table = CRC_TABLE.get_or_init(|| {
        let mut t = [0u32; 256];
        for i in 0..256u32 {
            let mut c = i;
            for _ in 0..8 {
                c = if c & 1 != 0 { 0xEDB88320 ^ (c >> 1) } else { c >> 1 };
            }
            t[i as usize] = c;
        }
        t
    });
    let mut crc = !crc;
    for &b in p {
        crc = table[((crc ^ b as u32) & 0xFF) as usize] ^ (crc >> 8);
    }
    !crc
}

fn crc_canvas(c: &Canvas) -> u32 {
    let mut crc = 0;
    for y in 0..c.h {
        let at = (y * c.stride) as usize;
        crc = crc32_bytes(crc, &c.px[at..at + c.w as usize]);
    }
    crc
}

fn g_print() -> bool {
    std::env::var_os("GRIMOIRE_GOLDEN_PRINT").is_some()
}

fn golden(name: &str, crc: u32) {
    if g_print() {
        println!("    (\"{}\", 0x{:08x}),", name, crc);
        return;
    }
    if !GOLDEN_HOST {
        return;
    }
    for &(n, expected) in K_GOLDEN.iter().chain(K_VERTICAL_TIP.iter()) {
        if n != name {
            continue;
        }
        check!(expected == crc);
        if expected != crc {
            eprintln!("  {}: 0x{:08x}, recorded 0x{:08x}", name, crc, expected);
        }
        return;
    }
    eprintln!("  {}: not in the table", name);
    check!(false); // every render has a recorded value
}

// One stroke across the page: a zigzag whose pressure starts below every deadzone, swells to
// full and fades again, with the pen leaning from left to right and nodding, so each tool shows
// its widths, darkness, grain, tilt shading and (calligraphy) its turning nib. Plain arithmetic
// only; seeds stay below 2^24.
fn golden_stroke(tool: usize, y0: f32, seed: u32) -> Box<Stroke> {
    let mut s = stroke_new(ToolKind(tool as i32), &k_beta_params()[tool], seed);
    const N: i32 = 161;
    for i in 0..N {
        let f = i as f32 / (N - 1) as f32;
        let tri = ((f * 6.0) % 2.0 - 1.0).abs();
        let bell = fmla(-fmla(2.0, f, -1.0), fmla(2.0, f, -1.0), 1.0);
        let q = PenSample {
            x: fmla(1420.0, f, 220.0),
            y: fmla(60.0 * f, f, fmla(170.0, tri, y0) - 85.0),
            pressure: if i < 3 { 0.0 } else { fmla(0.96, bell, 0.04) },
            tilt_x: fmla(120.0, f, -60.0),
            tilt_y: fmla(50.0, tri, -15.0),
            t_ms: i as u32 * 7,
        };
        stroke_append(&mut s, &q, DPI);
    }
    s
}

fn seed_for(tool: usize) -> u32 {
    0x00A5F00Du32 + tool as u32 * 40503
}

// Names for the table, fixed here (paper_name and tool_name may be translated one day).
const K_TOOL_KEY: [&str; BETA_TOOL_COUNT] = [
    "pencil", "mechanical pencil", "ballpoint", "eraser", "highlighter", "fountain pen", "fineliner", "marker", "charcoal", "calligraphy pen",
];
fn paper_key(k: PaperKind) -> &'static str {
    if k == PAPER_MEDIUM {
        "medium rule"
    } else {
        "grand staff"
    }
}

// Renders `pg` three ways: whole, ink only (on white) and paper only.
fn golden_page(c: &mut Canvas, pg: &Page, name: &str) {
    let full = Rect { x: 0, y: 0, w: W, h: H };
    page_render(c, pg, full, DPI);
    golden(name, crc_canvas(c));
    canvas_fill(c, full, 255);
    page_render_layers(c, pg, full, DPI, PAGE_LAYER_INK);
    golden(&cformat!("%s, ink", name), crc_canvas(c));
    canvas_fill(c, full, 0);
    page_render_layers(c, pg, full, DPI, PAGE_LAYER_PAPER);
    golden(&cformat!("%s, paper", name), crc_canvas(c));
}

fn papers() {
    let mut c = Canvas::default();
    check!(canvas_init(&mut c, W, H) == 0);
    let full = Rect { x: 0, y: 0, w: W, h: H };
    for gothic in 0..2 {
        // Paper labels and clefs come from Terminess in both themes: the theme never shows on a page.
        font_set_gothic(gothic != 0);
        for k in 0..BETA_PAPER_COUNT {
            canvas_fill(&mut c, full, 0);
            paper_render(&mut c, PaperKind(k as u32), full, DPI);
            let key = cformat!("paper %d", k);
            if gothic == 0 || !g_print() {
                golden(&key, crc_canvas(&c));
            }
        }
        for (i, (m, stored)) in k_music().iter().enumerate() {
            let k = music_encode(m);
            check!(k.0 == *stored);
            canvas_fill(&mut c, full, 0);
            paper_render(&mut c, PaperKind(*stored), full, DPI);
            let key = cformat!("custom music %d", i as i32);
            if gothic == 0 || !g_print() {
                golden(&key, crc_canvas(&c));
            }
        }
    }
    font_set_gothic(false);
    canvas_free(&mut c);
}

fn one_stroke_of_every_tool() {
    const PAPERS: [PaperKind; 2] = [PAPER_MEDIUM, PAPER_GRAND];
    let mut c = Canvas::default();
    check!(canvas_init(&mut c, W, H) == 0);
    for pi in 0..2 {
        for t in 0..BETA_TOOL_COUNT {
            let mut pg = page_new(W, H, PAPERS[pi]);
            page_add(&mut pg, golden_stroke(t, 1240.0, seed_for(t)));
            let key = cformat!("%s, %s", paper_key(PAPERS[pi]), K_TOOL_KEY[t]);
            golden_page(&mut c, &pg, &key);
            if ToolKind(t as i32) == TOOL_HIGHLIGHTER {
                // the same stroke as drawn since 2.0.0-beta.5: vertical tip
                stroke_mark_tip(&mut pg.strokes[0]);
                check!(stroke_tip(&pg.strokes[0]) == TIP_HIGHLIGHTER_VERTICAL);
                let key = cformat!("%s, highlighter, vertical tip", paper_key(PAPERS[pi]));
                golden_page(&mut c, &pg, &key);
            }
            page_free(pg);
        }
    }
    canvas_free(&mut c);
}

// Every drawing tool on one page, overlapping its neighbours (no eraser: no released build
// ever wrote one). The same pages also go through the file format: the bytes 1.0.0-beta wrote
// for them, and the page read back from those bytes, must not change either.
fn all_tools_on_one_page() {
    const PAPERS: [PaperKind; 2] = [PAPER_MEDIUM, PAPER_GRAND];
    let mut c = Canvas::default();
    check!(canvas_init(&mut c, W, H) == 0);
    for pi in 0..2 {
        let mut pg = page_new(W, H, PAPERS[pi]);
        let mut row = 0;
        for t in 0..BETA_TOOL_COUNT {
            if ToolKind(t as i32) == TOOL_ERASER {
                continue;
            }
            page_add(&mut pg, golden_stroke(t, fmla(245.0, row as f32, 260.0), seed_for(t)));
            row += 1;
        }
        let key = cformat!("%s, every tool", paper_key(PAPERS[pi]));
        golden_page(&mut c, &pg, &key);
        let mut bytes: Vec<u8> = Vec::new();
        check!(page_serialize(&pg, &mut bytes) == 0);
        let key = cformat!("%s, every tool, file of %zu bytes", paper_key(PAPERS[pi]), bytes.len());
        golden(&key, crc32_bytes(0, &bytes));
        let mut back: Option<Box<Page>> = None;
        check!(page_deserialize(&bytes, &mut back) == 0 && back.is_some());
        if let Some(back) = back {
            check!(back.strokes.len() == pg.strokes.len() && back.paper == pg.paper);
            page_render(&mut c, &back, Rect { x: 0, y: 0, w: W, h: H }, DPI);
            let key = cformat!("%s, every tool, read back", paper_key(PAPERS[pi]));
            golden(&key, crc_canvas(&c));
            page_free(back);
        }
        page_free(pg);
    }
    canvas_free(&mut c);
}

#[test]
fn test_golden() {
    check!(font_init() == 0); // papers draw labels and clefs
    check!(PAPER_COUNT >= BETA_PAPER_COUNT && TOOL_COUNT >= BETA_TOOL_COUNT);
    if !GOLDEN_HOST && !g_print() {
        eprintln!("  golden: not the recording host (clang, arm64 macOS): renders not compared");
    }
    if g_print() {
        println!("const K_GOLDEN: [(&str, u32); N] = [");
    }
    run!(papers);
    run!(one_stroke_of_every_tool);
    run!(all_tools_on_one_page);
    if g_print() {
        println!("];");
    }
    done();
}
