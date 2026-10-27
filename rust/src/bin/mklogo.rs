//! tools/mklogo.c: the Grimoire logo: "Grimoire" in the gothic title face (Jacquard 24, through the
//! app's own pixel renderer, so each design pixel is a k x k block exactly as on the Kindle) and
//! a pixel bat on the same grid. `make logo` writes docs/logo.png (black) and docs/logo-dark.png
//! (white), both on transparent; the PNGs are committed, so nothing else needs this tool.
//!   usage: mklogo <layout: above | crest | right> <k> <out.png> [white]
use grimoire::prelude::*;
use std::ffi::CString;
use std::io::Write;

/// stbi_write_png(filename, w, h, comp, data, stride): nonzero on success.
fn stbi_write_png(filename: &str, w: i32, h: i32, comp: i32, data: &[u8], stride: i32) -> i32 {
    let Ok(path) = CString::new(filename) else { return 0 };
    assert!(data.len() >= (h.max(0) as usize) * (stride.max(0) as usize), "stbi_write_png: short image");
    // SAFETY: path is NUL-terminated; data holds h rows of stride bytes (checked above), of which
    // stb reads w * comp per row and writes nothing.
    unsafe { grimoire::ffi_stb::stbi_write_png(path.as_ptr(), w, h, comp, data.as_ptr() as *const libc::c_void, stride) }
}

/// atoi, as musl's computes it: white space and a sign first, then the digits, in int, wrapping.
fn atoi(s: &str) -> i32 {
    let b = s.as_bytes();
    let mut i = 0;
    while i < b.len() && (b[i] == b' ' || (9..=13).contains(&b[i])) {
        i += 1;
    }
    let mut neg = false;
    if i < b.len() && (b[i] == b'+' || b[i] == b'-') {
        neg = b[i] == b'-';
        i += 1;
    }
    let mut n: i32 = 0;
    while i < b.len() && b[i].is_ascii_digit() {
        n = n.wrapping_mul(10).wrapping_sub((b[i] - b'0') as i32);
        i += 1;
    }
    if neg {
        n
    } else {
        n.wrapping_neg()
    }
}

fn main() {
    let args = std::env::args_os().map(|a| a.to_string_lossy().into_owned()).collect();
    std::process::exit(run(args));
}

fn run(args: Vec<String>) -> i32 {
    if args.len() < 4 {
        let _ = std::io::stderr().write_all(b"usage: mklogo <above|crest|right> <k> <out.png> [white]\n");
        return 2;
    }
    let (layout, out) = (args[1].as_str(), args[3].as_str());
    let k = atoi(&args[2]);
    let white = args.len() > 4 && args[4] == "white";
    if !(1..=16).contains(&k) || font_init() != 0 {
        return 1;
    }
    font_set_gothic(true);
    // The title face is Jacquard at 1.1x the asked size; 43 design pixels make one line.
    let px = 43.0f32 * k as f32 / 1.1f32;
    let word = "Grimoire";
    let tw = font_measure(FontFace::Title, px, word);
    let asc = font_ascent(FontFace::Title, px);
    let desc = font_descent(FontFace::Title, px);
    let (m, bw, bh) = (6 * k, BAT_W * k, BAT_H * k);
    let (w, h, tx, base, bx, by);
    let crest = layout == "crest";
    if crest || layout == "above" {
        // the bat over the middle of the word (crest: and a rule)
        w = (if tw > bw { tw } else { bw }) + 2 * m;
        h = m + bh + 4 * k + asc + desc + (if crest { 8 * k } else { 0 }) + m;
        tx = (w - tw) / 2;
        bx = (w - bw) / 2;
        by = m;
        base = by + bh + 4 * k + asc;
    } else {
        // flying off the end of the word, above the "re"
        let rx = font_measure(FontFace::Title, px, "Grimoi");
        tx = m;
        bx = tx + rx + bw / 4;
        by = m;
        base = m + bh / 2 + asc;
        w = (if bx + bw > tx + tw { bx + bw } else { tx + tw }) + m;
        h = base + desc + m;
    }
    let mut c = Canvas::default();
    if canvas_init(&mut c, w, h) != 0 {
        return 1;
    }
    font_draw(&mut c, FontFace::Title, px, tx, base, word, 0);
    draw_bat(&mut c, bx, by, k);
    if crest {
        // the Library cover's rule: two hairlines and a lozenge
        let (y, cx) = (base + desc + 2 * k, w / 2);
        canvas_hline(&mut c, tx + 4 * k, cx - 6 * k, y, k, 0);
        canvas_hline(&mut c, cx + 6 * k, tx + tw - 4 * k, y, k, 0);
        for r in 0..=3 {
            // a pixel diamond, 7 design pixels across
            let mut row = Rect { x: cx - (3 - r) * k - k / 2, y: y - r * k, w: (2 * (3 - r) + 1) * k, h: k };
            canvas_fill(&mut c, row, 0);
            row.y = y + r * k;
            canvas_fill(&mut c, row, 0);
        }
    }
    // One color on transparent: alpha is the ink's darkness.
    let n = w as usize * h as usize * 2;
    let mut ga: Vec<u8> = Vec::new();
    if ga.try_reserve_exact(n).is_err() {
        return 1;
    }
    ga.resize(n, 0);
    for y in 0..h {
        for x in 0..w {
            let at = ((y * w + x) * 2) as usize;
            ga[at] = if white { 255 } else { 0 };
            ga[at + 1] = 255 - c.px[(y * c.stride + x) as usize];
        }
    }
    let ok = stbi_write_png(out, w, h, 2, &ga, w * 2);
    drop(ga);
    canvas_free(&mut c);
    if ok == 0 {
        return 1;
    }
    print!("{}", cformat!("wrote %s (%dx%d)\n", out, w, h));
    0
}
