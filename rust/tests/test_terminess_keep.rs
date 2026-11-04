// test_terminess_keep: the Terminess diet (assets/terminess-keep.txt; assets/README.md). The
// embedded Terminess fonts hold exactly the code points the list names; the list keeps no Nerd Fonts
// glyph the diet leaves out; and every icon the code draws is on the list, in both fonts, and draws.
use grimoire::prelude::*;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

const TOP: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/..");

fn is_pua(cp: u32) -> bool {
    (0xE000..=0xF8FF).contains(&cp) || cp >= 0xF0000
}

fn hex(cp: &u32) -> String {
    format!("U+{cp:04X}")
}

fn fonts() -> [(&'static str, &'static [u8]); 2] {
    [("TerminessNerdFontMono-Regular.ttf", grimoire::assets::FONT_TERMINESS_REGULAR),
     ("TerminessNerdFontMono-Bold.ttf", grimoire::assets::FONT_TERMINESS_BOLD)]
}

/// assets/terminess-keep.txt: a code point (U+XXXX) or a range (U+XXXX-U+YYYY) a line; # comments.
fn keep_list() -> BTreeSet<u32> {
    let path = format!("{TOP}/assets/terminess-keep.txt");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"));
    let mut keep = BTreeSet::new();
    for (n, line) in text.lines().enumerate() {
        let line = line.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        let ends: Vec<u32> = line
            .split('-')
            .map(|p| {
                let p = p.trim();
                assert!(p.starts_with("U+"), "{path}:{}: not a code point or a range: {line:?}", n + 1);
                u32::from_str_radix(&p[2..], 16).unwrap_or_else(|_| panic!("{path}:{}: {line:?}", n + 1))
            })
            .collect();
        assert!(ends.len() <= 2 && ends[0] <= ends[ends.len() - 1], "{path}:{}: {line:?}", n + 1);
        for cp in ends[0]..=ends[ends.len() - 1] {
            assert!(keep.insert(cp), "{path}:{}: U+{cp:04X} is listed twice", n + 1);
        }
    }
    keep
}

/// Nerd Fonts' own glyphs at ordinary code points, which the diet leaves out (its box drawing and
/// block elements, the Octicons heart and lightning, the power symbols, the menu sign, the heavy
/// angle brackets: the list's head says why).
fn left_out() -> BTreeSet<u32> {
    let mut s: BTreeSet<u32> = (0x2500..=0x259F).collect();
    s.extend(0x23FB..=0x23FE);
    s.extend(0x276C..=0x2771);
    s.extend([0x2630, 0x2665, 0x26A1, 0x2B58]);
    s
}

// ---- the fonts' cmaps, read from their bytes ----

fn be16(d: &[u8], o: usize) -> u32 {
    u16::from_be_bytes([d[o], d[o + 1]]) as u32
}

fn be32(d: &[u8], o: usize) -> u32 {
    u32::from_be_bytes([d[o], d[o + 1], d[o + 2], d[o + 3]])
}

/// Where the font's cmap subtable for (platform, encoding) starts, if it has one.
fn subtable(font: &[u8], platform: u32, encoding: u32) -> Option<usize> {
    let cmap = (0..be16(font, 4) as usize)
        .map(|i| 12 + 16 * i)
        .find(|&r| &font[r..r + 4] == b"cmap")
        .map(|r| be32(font, r + 8) as usize)?;
    (0..be16(font, cmap + 2) as usize)
        .map(|i| cmap + 4 + 8 * i)
        .find(|&e| be16(font, e) == platform && be16(font, e + 2) == encoding)
        .map(|e| cmap + be32(font, e + 4) as usize)
}

/// The code points a format 4 or format 12 subtable maps to a glyph.
fn mapped(font: &[u8], at: usize) -> BTreeSet<u32> {
    let mut s = BTreeSet::new();
    match be16(font, at) {
        12 => {
            for g in 0..be32(font, at + 12) as usize {
                let o = at + 16 + 12 * g;
                let (first, last, glyph) = (be32(font, o), be32(font, o + 4), be32(font, o + 8));
                s.extend((first..=last).filter(|&cp| glyph + (cp - first) != 0));
            }
        }
        4 => {
            let segs = be16(font, at + 6) as usize / 2;
            let (ends, starts) = (at + 14, at + 16 + 2 * segs);
            let (deltas, ranges) = (starts + 2 * segs, starts + 4 * segs);
            for i in 0..segs {
                let (last, first) = (be16(font, ends + 2 * i), be16(font, starts + 2 * i));
                let (delta, range) = (be16(font, deltas + 2 * i), be16(font, ranges + 2 * i) as usize);
                for cp in first..=last {
                    let glyph = if range == 0 {
                        (cp + delta) & 0xFFFF
                    } else {
                        match be16(font, ranges + 2 * i + range + 2 * (cp - first) as usize) {
                            0 => 0,
                            g => (g + delta) & 0xFFFF,
                        }
                    };
                    if cp != 0xFFFF && glyph != 0 {
                        s.insert(cp);
                    }
                }
            }
        }
        f => panic!("cmap format {f}: this test reads 4 and 12"),
    }
    s
}

// ---- what the code draws ----

/// The text files of the places Grimoire draws Terminess from: the app (rust/src), KOReader's plugin
/// and patches (kindle/), the share pages (web/), the translations (po/) and What's new.
fn sources() -> Vec<PathBuf> {
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else { return };
        let mut entries: Vec<PathBuf> = entries.filter_map(|e| e.ok()).map(|e| e.path()).collect();
        entries.sort();
        for p in entries {
            if p.is_dir() {
                walk(&p, out);
            } else if p.extension().and_then(|e| e.to_str()).is_some_and(|e| {
                ["rs", "lua", "sh", "txt", "ld", "js", "css", "html", "po", "md"].contains(&e)
            }) {
                out.push(p);
            }
        }
    }
    let mut out = Vec::new();
    for dir in ["rust/src", "kindle", "web", "po"] {
        walk(&Path::new(TOP).join(dir), &mut out);
    }
    out.push(Path::new(TOP).join("docs/WHATSNEW.md"));
    out
}

/// Hex numbers that are the private use areas' own bounds, in range checks rather than drawn
/// (json_min.rs's surrogate test ends at 0xE000; the largest code point is 0x10FFFF).
const BOUNDS: [u32; 8] = [0xE000, 0xF8FF, 0xF0000, 0xFFFFD, 0xFFFFF, 0x100000, 0x10FFFD, 0x10FFFF];

/// The private use code points in a source text: characters, escapes (\u{...}, \uXXXX, a CSS
/// \XXXXX, &#x...;, &#...;, runs of \xHH or Lua's \ddd bytes) and hex numbers (0x..., with a Rust
/// or C suffix).
fn scan(text: &str, out: &mut BTreeSet<u32>) {
    out.extend(text.chars().map(|c| c as u32).filter(|&c| is_pua(c)));
    let b = text.as_bytes();
    let hexrun = |from: usize, max: usize| -> (u32, usize) {
        let mut v = 0u32;
        let mut n = 0;
        while from + n < b.len() && n < max && (b[from + n].is_ascii_hexdigit() || b[from + n] == b'_') {
            if b[from + n] != b'_' {
                v = v.saturating_mul(16).saturating_add((b[from + n] as char).to_digit(16).unwrap_or(0));
            }
            n += 1;
        }
        (v, n)
    };
    let mut add = |cp: u32| {
        if is_pua(cp) && cp <= 0x10FFFF {
            out.insert(cp);
        }
    };
    let mut bytes = Vec::new();
    let mut i = 0;
    while i < b.len() {
        let word_start = i == 0 || !(b[i - 1].is_ascii_alphanumeric() || b[i - 1] == b'_');
        if word_start && b[i] == b'0' && i + 1 < b.len() && (b[i + 1] == b'x' || b[i + 1] == b'X') {
            let (v, n) = hexrun(i + 2, 10);
            if n > 0 && !BOUNDS.contains(&v) {
                add(v);
            }
            i += 2 + n;
            continue;
        }
        if b[i] == b'&' && i + 2 < b.len() && b[i + 1] == b'#' {
            if b[i + 2] == b'x' || b[i + 2] == b'X' {
                let (v, n) = hexrun(i + 3, 6);
                if n > 0 && b.get(i + 3 + n) == Some(&b';') {
                    add(v);
                }
            } else {
                let digits = b[i + 2..].iter().take_while(|c| c.is_ascii_digit()).count();
                if (1..=7).contains(&digits) && b.get(i + 2 + digits) == Some(&b';') {
                    add(text[i + 2..i + 2 + digits].parse().unwrap_or(0));
                }
            }
        }
        if b[i] == b'\\' && i + 1 < b.len() {
            let c = b[i + 1];
            if c == b'u' && b.get(i + 2) == Some(&b'{') {
                let (v, n) = hexrun(i + 3, 6);
                if b.get(i + 3 + n) == Some(&b'}') {
                    add(v);
                }
            } else if c == b'u' {
                let (v, n) = hexrun(i + 2, 4);
                if n == 4 {
                    add(v);
                }
            } else if c == b'x' || c.is_ascii_digit() {
                // a run of byte escapes, read as UTF-8
                let mut j = i;
                bytes.clear();
                while j + 1 < b.len() && b[j] == b'\\' {
                    if b[j + 1] == b'x' {
                        let (v, n) = hexrun(j + 2, 2);
                        if n != 2 {
                            break;
                        }
                        bytes.push(v as u8);
                        j += 4;
                    } else if b[j + 1].is_ascii_digit() {
                        let n = b[j + 1..].iter().take(3).take_while(|c| c.is_ascii_digit()).count();
                        let Ok(v) = text[j + 1..j + 1 + n].parse::<u32>() else { break };
                        if v > 255 {
                            break;
                        }
                        bytes.push(v as u8);
                        j += 1 + n;
                    } else {
                        break;
                    }
                }
                if bytes.len() > 1 {
                    String::from_utf8_lossy(&bytes).chars().for_each(|ch| add(ch as u32));
                }
            }
            if c.is_ascii_hexdigit() {
                // a CSS escape, \f0150
                let (v, n) = hexrun(i + 1, 6);
                if n >= 4 {
                    add(v);
                }
            }
        }
        i += 1;
    }
}

fn drawn() -> BTreeSet<u32> {
    let mut out = BTreeSet::new();
    for f in sources() {
        let Ok(data) = std::fs::read(&f) else { continue };
        scan(&String::from_utf8_lossy(&data), &mut out);
    }
    out
}

#[test]
fn the_scan_reads_every_way_a_code_point_is_written() {
    let mut s = BTreeSet::new();
    scan("font_draw_icon(c, f, 0xF0F70, r, 0); [0xF0150u32]; 0xE000", &mut s);
    scan("\"\\u{F0831}\" '\\uE0A0' content:\"\\f0219\" &#xF0570; &#983644; \\xF3\\xB0\\x86\\x8F \\243\\176\\170\\182", &mut s);
    scan("\u{F09EE} 0xDC00..0xE000 c > 0x10FFFF 0x1234", &mut s);
    let want: BTreeSet<u32> =
        [0xF0F70, 0xF0150, 0xF0831, 0xE0A0, 0xF0219, 0xF0570, 0xF025C, 0xF018F, 0xF0AB6, 0xF09EE].into_iter().collect();
    assert_eq!(s, want);
}

#[test]
fn the_fonts_hold_exactly_the_list() {
    let keep = keep_list();
    for (name, font) in fonts() {
        // (3,10) is the cmap stb_truetype, FreeType and the browsers read; (3,1) older readers' BMP
        let full = mapped(font, subtable(font, 3, 10).expect("a (3,10) cmap"));
        let missing: Vec<String> = keep.difference(&full).map(hex).collect();
        let extra: Vec<String> = full.difference(&keep).map(hex).collect();
        assert!(
            missing.is_empty() && extra.is_empty(),
            "{name} is not assets/terminess-keep.txt (make fonts-subset): {} missing {:?}, {} extra {:?}",
            missing.len(),
            &missing[..missing.len().min(20)],
            extra.len(),
            &extra[..extra.len().min(20)]
        );
        let bmp: BTreeSet<u32> = full.iter().copied().filter(|&c| c < 0x10000).collect();
        assert_eq!(mapped(font, subtable(font, 3, 1).expect("a (3,1) cmap")), bmp, "{name}: (3,1) and (3,10)");
        for (enc, want) in [(3, &bmp), (4, &full)] {
            if let Some(at) = subtable(font, 0, enc) {
                assert_eq!(&mapped(font, at), want, "{name}: (0,{enc})");
            }
        }
    }
}

#[test]
fn the_list_keeps_terminus_text_and_material_design_icons_only() {
    let keep = keep_list();
    let back: Vec<String> = keep.intersection(&left_out()).map(hex).collect();
    assert!(back.is_empty(), "Nerd Fonts glyphs the diet leaves out are listed again: {back:?}");
    let not_mdi: Vec<String> = keep.iter().filter(|&&c| is_pua(c) && !(0xF0001..=0xF1AF0).contains(&c)).map(hex).collect();
    assert!(not_mdi.is_empty(), "private use code points outside Material Design Icons (U+F0001-U+F1AF0): {not_mdi:?}");
    // The text: ASCII whole, and Terminus's Latin, Greek, Cyrillic and symbols around it.
    assert!((0x20..=0x7E).all(|c| keep.contains(&c)) && keep.contains(&0xFFFD));
    assert_eq!(keep.iter().filter(|&&c| !is_pua(c)).count(), 1197);
}

#[test]
fn every_icon_the_code_draws_is_kept_and_draws() {
    let keep = keep_list();
    let drawn = drawn();
    // The scan finds the icon table, the shapes button and the clefs (65 at 2.0.0-beta.2).
    assert!(drawn.len() >= 65 && drawn.contains(&0xF064F) && drawn.contains(&0xF0831) && drawn.contains(&0xF0F6E),
        "the scan found {} icons", drawn.len());
    let missing: Vec<String> = drawn.difference(&keep).map(hex).collect();
    assert!(missing.is_empty(), "drawn but not in assets/terminess-keep.txt (add them, then make fonts-subset): {missing:?}");
    assert_eq!(font_init(), 0);
    for &cp in &drawn {
        for face in [FontFace::Regular, FontFace::Bold] {
            let ttf = if face == FontFace::Bold { FontFace::FixedBold } else { FontFace::Fixed };
            assert!(font_face_has_glyph(ttf, cp), "U+{cp:05X} is not in the {face:?} Terminess");
            let mut c = Canvas::default();
            assert_eq!(canvas_init(&mut c, 64, 64), 0);
            canvas_fill(&mut c, Rect { x: 0, y: 0, w: 64, h: 64 }, 255);
            font_draw_icon(&mut c, face, cp, Rect { x: 0, y: 0, w: 64, h: 64 }, 0);
            assert!(c.px.iter().any(|&p| p < 128), "U+{cp:05X} draws nothing ({face:?})");
            canvas_free(&mut c);
        }
    }
}
