// fontfix: the fonts the tests and the scenarios use, written here byte by byte (no font tool, no file from
// anywhere else): a small pixel face of our own, "Grimoire Test", as TrueType, as a variable font with a weight
// axis, as CFF, and the broken ones the check must answer without crashing (the malformed-font corpus:
// truncated files, a cmap that points past the end, composite glyphs that include each other, CFF subroutines
// nested too deep, a symbol cmap, a web font, letters that are there and draw nothing). tests/fixtures/fonts
// holds the ones the scenarios copy (`make font-fixtures` writes them; test_font_fixtures.rs checks the
// committed bytes are what this makes now).
//
// The face: every letter on a grid three pixels wide and five high (digits and capitals drawn, small letters
// the capitals at a lower height, the rest of ASCII a checker). 1000 units an em, 800 above the baseline and
// 200 below.
#![allow(dead_code)]

pub const UPEM: u16 = 1000;
pub const ASCENT: i16 = 800;
pub const DESCENT: i16 = -200;
/// A pixel's width, and a capital's and a small letter's pixel height.
const PW: i16 = 140;
const CAP_H: i16 = 140;
const LOW_H: i16 = 100;
const LSB: i16 = 60;
pub const ADVANCE: u16 = 580;

/// One glyph: its contours (points, all on the curve), or the glyphs it is made of (index, dx, dy); how far
/// each point moves right at the heaviest weight (a variable font's deltas; empty: none); its advance.
#[derive(Clone, Default)]
pub struct Glyph {
    pub contours: Vec<Vec<(i16, i16)>>,
    pub components: Vec<(u16, i16, i16)>,
    pub bolder: Vec<i16>,
    pub advance: u16,
}

/// What a font file is made of.
#[derive(Clone)]
pub struct Font {
    pub family: String,
    pub style: String,
    pub weight: u16,
    pub italic: bool,
    pub glyphs: Vec<Glyph>,
    /// code point to glyph, in code point order (the Basic Multilingual Plane)
    pub map: Vec<(u16, u16)>,
    /// a weight axis: least, default, most (the glyphs' `bolder` are its deltas)
    pub wght: Option<(u16, u16, u16)>,
    /// the cmap is a symbol one (platform 3, encoding 0) and nothing else
    pub symbol: bool,
    pub ascent: i16,
    pub descent: i16,
}

fn rows(c: char) -> Option<[&'static str; 5]> {
    Some(match c.to_ascii_uppercase() {
        '0' | 'O' => ["###", "#.#", "#.#", "#.#", "###"],
        '1' => [".#.", "##.", ".#.", ".#.", "###"],
        '2' => ["###", "..#", "###", "#..", "###"],
        '3' => ["###", "..#", "###", "..#", "###"],
        '4' => ["#.#", "#.#", "###", "..#", "..#"],
        '5' | 'S' => ["###", "#..", "###", "..#", "###"],
        '6' => ["###", "#..", "###", "#.#", "###"],
        '7' => ["###", "..#", "..#", "..#", "..#"],
        '8' => ["###", "#.#", "###", "#.#", "###"],
        '9' => ["###", "#.#", "###", "..#", "###"],
        'A' => ["###", "#.#", "###", "#.#", "#.#"],
        'B' => ["##.", "#.#", "##.", "#.#", "##."],
        'C' => ["###", "#..", "#..", "#..", "###"],
        'D' => ["##.", "#.#", "#.#", "#.#", "##."],
        'E' => ["###", "#..", "###", "#..", "###"],
        'F' => ["###", "#..", "###", "#..", "#.."],
        'G' => ["###", "#..", "#.#", "#.#", "###"],
        'H' => ["#.#", "#.#", "###", "#.#", "#.#"],
        'I' => ["###", ".#.", ".#.", ".#.", "###"],
        'J' => ["..#", "..#", "..#", "#.#", "###"],
        'K' => ["#.#", "#.#", "##.", "#.#", "#.#"],
        'L' => ["#..", "#..", "#..", "#..", "###"],
        'M' => ["#.#", "###", "###", "#.#", "#.#"],
        'N' => ["###", "#.#", "#.#", "#.#", "#.#"],
        'P' => ["###", "#.#", "###", "#..", "#.."],
        'Q' => ["###", "#.#", "#.#", "###", "..#"],
        'R' => ["###", "#.#", "##.", "#.#", "#.#"],
        'T' => ["###", ".#.", ".#.", ".#.", ".#."],
        'U' => ["#.#", "#.#", "#.#", "#.#", "###"],
        'V' => ["#.#", "#.#", "#.#", "#.#", ".#."],
        'W' => ["#.#", "#.#", "###", "###", "#.#"],
        'X' => ["#.#", "#.#", ".#.", "#.#", "#.#"],
        'Y' => ["#.#", "#.#", ".#.", ".#.", ".#."],
        'Z' => ["###", "..#", ".#.", "#..", "###"],
        '.' => ["...", "...", "...", "...", ".#."],
        ',' => ["...", "...", "...", ".#.", "#.."],
        '-' => ["...", "...", "###", "...", "..."],
        '!' => [".#.", ".#.", ".#.", "...", ".#."],
        '?' => ["###", "..#", ".#.", "...", ".#."],
        ':' => ["...", ".#.", "...", ".#.", "..."],
        ' ' => return None,
        _ => ["#.#", ".#.", "#.#", ".#.", "#.#"],
    })
}

/// The glyph of a character: a rectangle for each run of pixels in a row. `wider`: every rectangle that much
/// wider (a Bold file). A variable font's glyph gets the same widening as its deltas.
pub fn glyph_of(c: char, wider: i16, deltas: i16) -> Glyph {
    let mut g = Glyph { advance: ADVANCE, ..Default::default() };
    let Some(r) = rows(c) else { return g };
    let h = if c.is_ascii_lowercase() { LOW_H } else { CAP_H };
    for (row, line) in r.iter().enumerate() {
        let b = line.as_bytes();
        let mut x = 0;
        while x < 3 {
            if b[x] != b'#' {
                x += 1;
                continue;
            }
            let mut e = x;
            while e < 3 && b[e] == b'#' {
                e += 1;
            }
            let (x0, x1) = (LSB + x as i16 * PW, LSB + e as i16 * PW + wider);
            let (y0, y1) = ((4 - row as i16) * h, (5 - row as i16) * h);
            g.contours.push(vec![(x0, y0), (x0, y1), (x1, y1), (x1, y0)]);
            if deltas != 0 {
                g.bolder.extend([0, 0, deltas, deltas]);
            }
            x = e;
        }
    }
    g
}

/// "Grimoire Test": printable ASCII (glyph 1 is the space, 2 the exclamation mark and so on; glyph 0 a box).
pub fn test_font(style: &str, weight: u16) -> Font {
    let wider = if weight >= 600 { 50 } else { 0 };
    let mut glyphs = vec![Glyph { contours: vec![vec![(60, 0), (60, 700), (480, 700), (480, 0)]], advance: ADVANCE, ..Default::default() }];
    let mut map = Vec::new();
    for cp in 0x20u16..0x7F {
        map.push((cp, glyphs.len() as u16));
        glyphs.push(glyph_of(cp as u8 as char, wider, 0));
    }
    Font { family: "Grimoire Test".into(), style: style.into(), weight, italic: false, glyphs, map, wght: None, symbol: false, ascent: ASCENT, descent: DESCENT }
}

/// The same with Latin-1's upper half as well (192 glyphs), each drawn as its place in the block's letter.
pub fn test_font_latin1(style: &str, weight: u16) -> Font {
    let mut f = test_font(style, weight);
    for cp in 0xA0u16..0x100 {
        f.map.push((cp, f.glyphs.len() as u16));
        let base = (b'A' + ((cp - 0xA0) % 26) as u8) as char;
        f.glyphs.push(glyph_of(if cp >= 0xE0 { base.to_ascii_lowercase() } else { base }, 0, 0));
    }
    f
}

/// The variable one: a weight axis from 400 to 700, every rectangle 50 units wider at 700.
pub fn test_font_variable() -> Font {
    let mut f = test_font("Regular", 400);
    f.family = "Grimoire Test Variable".into();
    for cp in 0x20u16..0x7F {
        f.glyphs[(cp - 0x20 + 1) as usize] = glyph_of(cp as u8 as char, 0, 50);
    }
    f.wght = Some((400, 400, 700));
    f
}

/// A font of many glyphs (the check's time): ASCII, then `n` more mapped from U+4E00 on, each a few rectangles.
pub fn test_font_many(n: usize) -> Font {
    let mut f = test_font("Regular", 400);
    f.family = "Grimoire Test Many".into();
    let letters: Vec<char> = ('A'..='Z').chain('0'..='9').collect();
    for i in 0..n {
        f.map.push((0x4E00 + i as u16, f.glyphs.len() as u16));
        f.glyphs.push(glyph_of(letters[i % letters.len()], (i % 7) as i16 * 5, 0));
    }
    f
}

fn gid(f: &Font, c: char) -> usize {
    f.map.iter().find(|m| m.0 == c as u16).unwrap().1 as usize
}

// ---- the broken ones ----

/// Composite glyphs that include each other: A is made of B, and B of A. With `letters` the letters themselves
/// (the font is unreadable); without, the currency sign of a Latin-1 font (one glyph of 191 comes from
/// Terminess).
pub fn cyclic_composite(letters: bool) -> Font {
    let mut f = test_font_latin1("Regular", 400);
    f.family = "Grimoire Test Cyclic".into();
    let (a, b) = if letters { (gid(&f, 'A'), gid(&f, 'B')) } else { (gid(&f, '\u{a4}'), f.glyphs.len()) };
    if !letters {
        f.glyphs.push(Glyph::default()); // (a glyph no character maps to: the other half of the ring)
    }
    f.glyphs[a] = Glyph { components: vec![(b as u16, 0, 0)], advance: ADVANCE, ..Default::default() };
    f.glyphs[b] = Glyph { components: vec![(a as u16, 0, 0)], advance: ADVANCE, ..Default::default() };
    f
}

/// Letters that are there and draw nothing: every letter and digit a glyph without a contour.
pub fn empty_letters() -> Font {
    let mut f = test_font("Regular", 400);
    f.family = "Grimoire Test Blank".into();
    for g in f.glyphs.iter_mut().skip(1) {
        g.contours.clear();
    }
    f
}

/// A symbol font: the same glyphs under a symbol cmap alone (platform 3, encoding 0, at U+F020 and on).
pub fn symbol_cmap() -> Font {
    let mut f = test_font("Regular", 400);
    f.family = "Grimoire Test Symbol".into();
    for m in f.map.iter_mut() {
        m.0 += 0xF000;
    }
    f.symbol = true;
    f
}

/// Which glyphs of a Latin-1 font are wild.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Wild {
    /// the section sign, far outside any line (the box cap)
    Far,
    /// the pilcrow, of far too many points (the point cap)
    Dense,
    /// five glyphs far outside: more than one in a hundred
    TooMany,
}

/// A Latin-1 font (191 glyphs mapped) with a glyph or more that no cap lets through.
pub fn wild_glyphs(which: Wild) -> Font {
    let mut f = test_font_latin1("Regular", 400);
    f.family = "Grimoire Test Wild".into();
    let far = vec![vec![(0, 0), (0, 30000), (30000, 30000), (30000, 0)]];
    match which {
        Wild::Far => {
            let g = gid(&f, '\u{a7}');
            f.glyphs[g].contours = far;
        }
        Wild::Dense => {
            let g = gid(&f, '\u{b6}');
            f.glyphs[g].contours = (0..1300)
                .map(|i| {
                    let (x, y) = ((i % 100) as i16 * 4, (i / 100) as i16 * 40);
                    vec![(x, y), (x, y + 30), (x + 3, y + 30), (x + 3, y)]
                })
                .collect();
        }
        Wild::TooMany => {
            for c in ['\u{a1}', '\u{a2}', '\u{a3}', '\u{a5}', '\u{a7}'] {
                let g = gid(&f, c);
                f.glyphs[g].contours = far.clone();
            }
        }
    }
    f
}

/// The glyph a character of a font maps to.
pub fn glyph_index(f: &Font, c: char) -> u32 {
    gid(f, c) as u32
}

/// A font whose tables claim a line far from its letters: ten ems above the baseline and nine below.
pub fn tall_line() -> Font {
    let mut f = test_font("Regular", 400);
    f.family = "Grimoire Test Tall".into();
    f.ascent = 7000;
    f.descent = -900;
    f
}

/// The bytes of a web font: the WOFF2 signature and nothing a font check should read on.
pub fn woff2() -> Vec<u8> {
    let mut v = b"wOF2".to_vec();
    v.extend_from_slice(&0x00010000u32.to_be_bytes());
    v.extend(std::iter::repeat_n(0xA5u8, 120));
    v
}

/// A TrueType file whose cmap's one subtable is said to start past the end of the file.
pub fn cmap_past_end() -> Vec<u8> {
    let mut v = ttf(&test_font("Regular", 400));
    let at = table(&v, b"cmap").0;
    // (the encoding record's offset: version, count, platform, encoding, then the offset)
    v[at + 8..at + 12].copy_from_slice(&0x7FFF_FFF0u32.to_be_bytes());
    v
}

/// Where a table is in a font file, and how long.
pub fn table(v: &[u8], tag: &[u8; 4]) -> (usize, usize) {
    let n = u16::from_be_bytes([v[4], v[5]]) as usize;
    for i in 0..n {
        let r = &v[12 + i * 16..12 + i * 16 + 16];
        if &r[..4] == tag {
            return (u32::from_be_bytes([r[8], r[9], r[10], r[11]]) as usize, u32::from_be_bytes([r[12], r[13], r[14], r[15]]) as usize);
        }
    }
    panic!("no such table");
}

// ---- writing: the tables ----

fn w16(v: &mut Vec<u8>, x: u16) {
    v.extend_from_slice(&x.to_be_bytes());
}
fn wi16(v: &mut Vec<u8>, x: i16) {
    v.extend_from_slice(&x.to_be_bytes());
}
fn w32(v: &mut Vec<u8>, x: u32) {
    v.extend_from_slice(&x.to_be_bytes());
}

fn checksum(t: &[u8]) -> u32 {
    let mut s = 0u32;
    for c in t.chunks(4) {
        let mut w = [0u8; 4];
        w[..c.len()].copy_from_slice(c);
        s = s.wrapping_add(u32::from_be_bytes(w));
    }
    s
}

/// The file: the table directory (tables in tag order), each table padded to four bytes.
fn sfnt(kind: u32, mut tables: Vec<([u8; 4], Vec<u8>)>) -> Vec<u8> {
    tables.sort_by(|a, b| a.0.cmp(&b.0));
    let n = tables.len() as u16;
    let mut v = Vec::new();
    w32(&mut v, kind);
    w16(&mut v, n);
    let pow = 1u16 << (15 - n.leading_zeros() as u16);
    w16(&mut v, pow * 16);
    w16(&mut v, 15 - n.leading_zeros() as u16);
    w16(&mut v, n * 16 - pow * 16);
    let mut at = 12 + tables.len() * 16;
    for (tag, t) in &tables {
        v.extend_from_slice(tag);
        w32(&mut v, checksum(t));
        w32(&mut v, at as u32);
        w32(&mut v, t.len() as u32);
        at += t.len().div_ceil(4) * 4;
    }
    for (_, t) in &tables {
        v.extend_from_slice(t);
        v.resize(v.len().div_ceil(4) * 4, 0);
    }
    v
}

fn bounds(g: &Glyph) -> (i16, i16, i16, i16) {
    let pts: Vec<(i16, i16)> = g.contours.iter().flatten().copied().collect();
    if pts.is_empty() {
        return (0, 0, 0, 0);
    }
    (pts.iter().map(|p| p.0).min().unwrap(), pts.iter().map(|p| p.1).min().unwrap(), pts.iter().map(|p| p.0).max().unwrap(), pts.iter().map(|p| p.1).max().unwrap())
}

fn glyf_of(g: &Glyph) -> Vec<u8> {
    let mut v = Vec::new();
    if !g.components.is_empty() {
        wi16(&mut v, -1);
        for _ in 0..4 {
            wi16(&mut v, 0);
        }
        for (i, (glyph, dx, dy)) in g.components.iter().enumerate() {
            // ARG_1_AND_2_ARE_WORDS | ARGS_ARE_XY_VALUES, and MORE_COMPONENTS but on the last
            w16(&mut v, 0x0003 | if i + 1 < g.components.len() { 0x0020 } else { 0 });
            w16(&mut v, *glyph);
            wi16(&mut v, *dx);
            wi16(&mut v, *dy);
        }
        return v;
    }
    if g.contours.is_empty() {
        return v; // (no ink: no bytes)
    }
    let (x0, y0, x1, y1) = bounds(g);
    wi16(&mut v, g.contours.len() as i16);
    for b in [x0, y0, x1, y1] {
        wi16(&mut v, b);
    }
    let mut end = 0usize;
    for c in &g.contours {
        end += c.len();
        w16(&mut v, (end - 1) as u16);
    }
    w16(&mut v, 0); // no instructions
    let pts: Vec<(i16, i16)> = g.contours.iter().flatten().copied().collect();
    v.extend(std::iter::repeat_n(1u8, pts.len())); // every point on the curve, coordinates as words
    let mut last = 0i16;
    for p in &pts {
        wi16(&mut v, p.0.wrapping_sub(last));
        last = p.0;
    }
    last = 0;
    for p in &pts {
        wi16(&mut v, p.1.wrapping_sub(last));
        last = p.1;
    }
    v
}

fn cmap_of(f: &Font) -> Vec<u8> {
    // format 4: a segment for each run of code points whose glyphs run with them
    let mut segs: Vec<(u16, u16, u16)> = Vec::new(); // start, end, first glyph
    for &(cp, g) in &f.map {
        match segs.last_mut() {
            Some(s) if s.1 + 1 == cp && s.2 + (cp - s.0) == g => s.1 = cp,
            _ => segs.push((cp, cp, g)),
        }
    }
    let n = segs.len() as u16 + 1;
    let mut sub = Vec::new();
    w16(&mut sub, 4);
    w16(&mut sub, 16 + n * 8);
    w16(&mut sub, 0);
    w16(&mut sub, n * 2);
    let pow = 1u16 << (15 - n.leading_zeros() as u16);
    w16(&mut sub, pow * 2);
    w16(&mut sub, 15 - n.leading_zeros() as u16);
    w16(&mut sub, n * 2 - pow * 2);
    for s in &segs {
        w16(&mut sub, s.1);
    }
    w16(&mut sub, 0xFFFF);
    w16(&mut sub, 0);
    for s in &segs {
        w16(&mut sub, s.0);
    }
    w16(&mut sub, 0xFFFF);
    for s in &segs {
        w16(&mut sub, s.2.wrapping_sub(s.0));
    }
    w16(&mut sub, 1);
    for _ in 0..n {
        w16(&mut sub, 0);
    }
    let mut v = Vec::new();
    w16(&mut v, 0);
    w16(&mut v, 1);
    w16(&mut v, 3);
    w16(&mut v, if f.symbol { 0 } else { 1 });
    w32(&mut v, 12);
    v.extend_from_slice(&sub);
    v
}

fn name_of(f: &Font) -> Vec<u8> {
    let full = format!("{} {}", f.family, f.style);
    let items: [(u16, &str); 4] = [(1, &f.family), (2, &f.style), (4, &full), (6, "GrimoireTest")];
    let mut strings = Vec::new();
    let mut v = Vec::new();
    w16(&mut v, 0);
    w16(&mut v, items.len() as u16);
    w16(&mut v, 6 + items.len() as u16 * 12);
    for (id, s) in items {
        let utf16: Vec<u8> = s.encode_utf16().flat_map(|u| u.to_be_bytes()).collect();
        for x in [3, 1, 0x409, id, utf16.len() as u16, strings.len() as u16] {
            w16(&mut v, x);
        }
        strings.extend(utf16);
    }
    v.extend(strings);
    v
}

fn os2_of(f: &Font) -> Vec<u8> {
    let mut v = vec![0u8; 96];
    v[0..2].copy_from_slice(&4u16.to_be_bytes());
    v[2..4].copy_from_slice(&(ADVANCE as i16).to_be_bytes());
    v[4..6].copy_from_slice(&f.weight.to_be_bytes());
    v[6..8].copy_from_slice(&5u16.to_be_bytes());
    let sel: u16 = if f.italic { 1 } else if f.weight >= 600 { 0x20 } else { 0x40 };
    v[62..64].copy_from_slice(&sel.to_be_bytes());
    v[64..66].copy_from_slice(&0x20u16.to_be_bytes());
    v[66..68].copy_from_slice(&0x7Eu16.to_be_bytes());
    v[68..70].copy_from_slice(&f.ascent.to_be_bytes());
    v[70..72].copy_from_slice(&f.descent.to_be_bytes());
    v[74..76].copy_from_slice(&(f.ascent as u16).to_be_bytes());
    v[76..78].copy_from_slice(&((-f.descent) as u16).to_be_bytes());
    v[86..88].copy_from_slice(&500i16.to_be_bytes());
    v[88..90].copy_from_slice(&700i16.to_be_bytes());
    v
}

fn head_of(loca_long: bool) -> Vec<u8> {
    let mut v = Vec::new();
    w32(&mut v, 0x0001_0000);
    w32(&mut v, 0x0001_0000);
    w32(&mut v, 0);
    w32(&mut v, 0x5F0F_3CF5);
    w16(&mut v, 0x000B);
    w16(&mut v, UPEM);
    v.extend_from_slice(&[0; 16]); // created, modified: the same bytes on any day
    for b in [0i16, DESCENT, ADVANCE as i16, ASCENT] {
        wi16(&mut v, b);
    }
    w16(&mut v, 0);
    w16(&mut v, 8);
    wi16(&mut v, 2);
    wi16(&mut v, loca_long as i16);
    wi16(&mut v, 0);
    v
}

fn hhea_of(f: &Font) -> Vec<u8> {
    let mut v = Vec::new();
    w32(&mut v, 0x0001_0000);
    wi16(&mut v, f.ascent);
    wi16(&mut v, f.descent);
    wi16(&mut v, 0);
    w16(&mut v, ADVANCE);
    wi16(&mut v, 0);
    wi16(&mut v, 0);
    wi16(&mut v, ADVANCE as i16);
    wi16(&mut v, 1);
    for _ in 0..7 {
        wi16(&mut v, 0);
    }
    w16(&mut v, f.glyphs.len() as u16);
    v
}

fn hmtx_of(f: &Font) -> Vec<u8> {
    let mut v = Vec::new();
    for g in &f.glyphs {
        w16(&mut v, g.advance);
        wi16(&mut v, bounds(g).0);
    }
    v
}

fn post_of() -> Vec<u8> {
    let mut v = Vec::new();
    w32(&mut v, 0x0003_0000);
    v.extend_from_slice(&[0; 28]);
    v
}

fn fvar_of(axis: (u16, u16, u16)) -> Vec<u8> {
    let mut v = Vec::new();
    for x in [1u16, 0, 16, 2, 1, 20, 0, 8] {
        w16(&mut v, x);
    }
    v.extend_from_slice(b"wght");
    for x in [axis.0, axis.1, axis.2] {
        w32(&mut v, (x as u32) << 16);
    }
    w16(&mut v, 0);
    w16(&mut v, 256);
    v
}

// Deltas as gvar packs them: runs of up to 64, zeros as a count alone, the others a byte each.
fn pack_deltas(d: &[i16]) -> Vec<u8> {
    let mut v = Vec::new();
    let mut i = 0;
    while i < d.len() {
        let zero = d[i] == 0;
        let mut n = 1;
        while i + n < d.len() && n < 64 && (d[i + n] == 0) == zero {
            n += 1;
        }
        if zero {
            v.push(0x80 | (n as u8 - 1));
        } else {
            v.push(n as u8 - 1);
            v.extend(d[i..i + n].iter().map(|&x| x as i8 as u8));
        }
        i += n;
    }
    v
}

fn gvar_of(f: &Font) -> Vec<u8> {
    let mut data: Vec<Vec<u8>> = Vec::new();
    for g in &f.glyphs {
        if g.bolder.is_empty() {
            data.push(Vec::new());
            continue;
        }
        // one tuple, its peak at the axis's most, its own point numbers (all of them), the x deltas (the four
        // phantom points keep still), then the y deltas (none)
        let mut x = g.bolder.clone();
        x.extend([0, 0, 0, 0]);
        let mut ser = vec![0u8];
        ser.extend(pack_deltas(&x));
        ser.extend(pack_deltas(&vec![0; x.len()]));
        let mut v = Vec::new();
        w16(&mut v, 1);
        w16(&mut v, 4 + 4 + 2);
        w16(&mut v, ser.len() as u16);
        w16(&mut v, 0x8000 | 0x2000);
        w16(&mut v, 0x4000);
        v.extend(ser);
        v.resize(v.len().div_ceil(2) * 2, 0);
        data.push(v);
    }
    let n = f.glyphs.len();
    let head = 20 + (n + 1) * 4;
    let mut v = Vec::new();
    for x in [1u16, 0, 1, 0] {
        w16(&mut v, x);
    }
    w32(&mut v, head as u32);
    w16(&mut v, n as u16);
    w16(&mut v, 1);
    w32(&mut v, head as u32);
    let mut at = 0u32;
    for d in &data {
        w32(&mut v, at);
        at += d.len() as u32;
    }
    w32(&mut v, at);
    for d in data {
        v.extend(d);
    }
    v
}

/// A font whose every stroke is `w` units wide and `h` high at most (a hairline, or dust).
pub fn thin(w: i16, h: i16) -> Font {
    let mut f = test_font("Regular", 400);
    f.family = "Grimoire Test Thin".into();
    for g in f.glyphs.iter_mut().skip(1) {
        for c in g.contours.iter_mut() {
            let (x0, y0, y1) = (c[0].0, c[0].1, c[1].1);
            let top = y1.min(y0 + h);
            *c = vec![(x0, y0), (x0, top), (x0 + w, top), (x0 + w, y0)];
        }
    }
    f
}

/// The TrueType file of a font, made `pad` bytes bigger by a table no reader looks at (a big file that is
/// still a font).
pub fn ttf_padded(f: &Font, pad: usize) -> Vec<u8> {
    ttf_with(f, pad)
}

/// The TrueType file of a font.
pub fn ttf(f: &Font) -> Vec<u8> {
    ttf_with(f, 0)
}

fn ttf_with(f: &Font, pad: usize) -> Vec<u8> {
    let (mut glyf, mut loca) = (Vec::new(), Vec::new());
    for g in &f.glyphs {
        w32(&mut loca, glyf.len() as u32);
        glyf.extend(glyf_of(g));
        glyf.resize(glyf.len().div_ceil(4) * 4, 0);
    }
    w32(&mut loca, glyf.len() as u32);
    let mut maxp = Vec::new();
    w32(&mut maxp, 0x0001_0000);
    w16(&mut maxp, f.glyphs.len() as u16);
    for x in [6000u16, 1500, 6000, 1500, 2, 0, 0, 0, 0, 0, 0, 4, 4] {
        w16(&mut maxp, x);
    }
    let mut tables = vec![
        (*b"head", head_of(true)),
        (*b"hhea", hhea_of(f)),
        (*b"maxp", maxp),
        (*b"OS/2", os2_of(f)),
        (*b"hmtx", hmtx_of(f)),
        (*b"cmap", cmap_of(f)),
        (*b"loca", loca),
        (*b"glyf", glyf),
        (*b"name", name_of(f)),
        (*b"post", post_of()),
    ];
    if let Some(axis) = f.wght {
        tables.push((*b"fvar", fvar_of(axis)));
        tables.push((*b"gvar", gvar_of(f)));
    }
    if pad > 0 {
        tables.push((*b"zpad", vec![0x5A; pad]));
    }
    sfnt(0x0001_0000, tables)
}

// ---- writing: CFF ----

fn cs_num(v: &mut Vec<u8>, n: i16) {
    v.push(28);
    v.extend_from_slice(&n.to_be_bytes());
}

fn cff_index(items: &[Vec<u8>]) -> Vec<u8> {
    let mut v = Vec::new();
    w16(&mut v, items.len() as u16);
    if items.is_empty() {
        return v;
    }
    v.push(4);
    let mut at = 1u32;
    w32(&mut v, at);
    for i in items {
        at += i.len() as u32;
        w32(&mut v, at);
    }
    for i in items {
        v.extend_from_slice(i);
    }
    v
}

fn dict_int(v: &mut Vec<u8>, n: i32) {
    v.push(29);
    v.extend_from_slice(&n.to_be_bytes());
}

/// A glyph's Type 2 charstring: a move and three lines for each rectangle (the fourth side closes itself);
/// the capital O two discs of four curves each instead, so the file has cubic curves in it.
fn charstring(g: &Glyph, round: bool) -> Vec<u8> {
    let mut v = Vec::new();
    let (mut cx, mut cy) = (0i16, 0i16);
    if round {
        // a circle of radius 300 about (290, 350), and a hole of radius 160, each four curves
        for (r, k) in [(300i16, 166i16), (160, 88)] {
            let (sx, sy) = (290 + r, 350);
            cs_num(&mut v, sx - cx);
            cs_num(&mut v, sy - cy);
            v.push(21);
            for (a, b, c) in [((0, k), (-(r - k), r - k), (-k, 0)), ((-k, 0), (-(r - k), -(r - k)), (0, -k)), ((0, -k), (r - k, -(r - k)), (k, 0)), ((k, 0), (r - k, r - k), (0, k))] {
                for n in [a.0, a.1, b.0, b.1, c.0, c.1] {
                    cs_num(&mut v, n);
                }
                v.push(8);
            }
            (cx, cy) = (sx, sy);
        }
    } else {
        for c in &g.contours {
            cs_num(&mut v, c[0].0 - cx);
            cs_num(&mut v, c[0].1 - cy);
            v.push(21);
            for w in c.windows(2) {
                cs_num(&mut v, w[1].0 - w[0].0);
                cs_num(&mut v, w[1].1 - w[0].1);
            }
            v.push(5);
            (cx, cy) = *c.last().unwrap();
        }
    }
    v.push(14);
    v
}

/// The OpenType (CFF) file of a font. `deep`: the letters' outlines are behind subroutines nested that many
/// deep (the format allows ten); `deep_one`: only the tilde's are.
pub fn otf(f: &Font, deep: usize, deep_one: bool) -> Vec<u8> {
    let mut subrs: Vec<Vec<u8>> = Vec::new();
    if deep > 0 {
        // subroutine i calls i + 1; the last draws a box
        for i in 0..deep {
            let mut s = Vec::new();
            if i + 1 < deep {
                cs_num(&mut s, i as i16 + 1 - 107);
                s.push(10);
            } else {
                for n in [60, 0] {
                    cs_num(&mut s, n);
                }
                s.push(21);
                for n in [400, 0, 0, 700, -400, 0] {
                    cs_num(&mut s, n);
                }
                s.push(5);
            }
            s.push(11);
            subrs.push(s);
        }
    }
    let through_subrs = || {
        let mut c = Vec::new();
        cs_num(&mut c, -107);
        c.extend([10, 14]);
        c
    };
    let o = gid(f, 'O');
    let tilde = gid(f, '~');
    let chars: Vec<Vec<u8>> = f
        .glyphs
        .iter()
        .enumerate()
        .map(|(i, g)| {
            let hit = deep > 0 && i != 0 && if deep_one { i == tilde } else { !g.contours.is_empty() };
            if hit {
                through_subrs()
            } else {
                charstring(g, i == o)
            }
        })
        .collect();
    let name = cff_index(&[b"GrimoireTest".to_vec()]);
    let strings = cff_index(&[]);
    let gsubrs = cff_index(&[]);
    let charstrings = cff_index(&chars);
    let lsubrs = cff_index(&subrs);
    let mut private = Vec::new();
    cs_num(&mut private, ADVANCE as i16);
    private.push(20); // defaultWidthX
    if !subrs.is_empty() {
        dict_int(&mut private, 4 + 6);
        private.push(19); // Subrs, from the Private DICT's start: right after it
    }
    let top_len = 17;
    let top_index_len = 2 + 1 + 8 + top_len;
    let chars_at = 4 + name.len() + top_index_len + strings.len() + gsubrs.len();
    let private_at = chars_at + charstrings.len();
    let mut top = Vec::new();
    dict_int(&mut top, chars_at as i32);
    top.push(17);
    dict_int(&mut top, private.len() as i32);
    dict_int(&mut top, private_at as i32);
    top.push(18);
    assert_eq!(top.len(), top_len);
    let mut cff = vec![1, 0, 4, 4];
    cff.extend(name);
    cff.extend(cff_index(&[top]));
    cff.extend(strings);
    cff.extend(gsubrs);
    cff.extend(charstrings);
    cff.extend(private);
    cff.extend(lsubrs);
    let mut maxp = Vec::new();
    w32(&mut maxp, 0x0000_5000);
    w16(&mut maxp, f.glyphs.len() as u16);
    sfnt(
        0x4F54_544F,
        vec![
            (*b"CFF ", cff),
            (*b"head", head_of(false)),
            (*b"hhea", hhea_of(f)),
            (*b"maxp", maxp),
            (*b"OS/2", os2_of(f)),
            (*b"hmtx", hmtx_of(f)),
            (*b"cmap", cmap_of(f)),
            (*b"name", name_of(f)),
            (*b"post", post_of()),
        ],
    )
}

/// The fonts tests/fixtures/fonts holds, for the scenarios: each folder is copied into a data root's fonts/.
pub fn scenario_files() -> Vec<(&'static str, Vec<u8>)> {
    let mut cff = test_font("Regular", 400);
    cff.family = "Grimoire Test CFF".into();
    let mut v = vec![
        // a family as someone unzips one: a Regular and its Bold, a variable font, an OpenType (CFF) one
        ("good/GrimoireTest-Regular.ttf", ttf(&test_font("Regular", 400))),
        ("good/GrimoireTest-Bold.ttf", ttf(&test_font("Bold", 700))),
        ("good/GrimoireTestVariable.ttf", ttf(&test_font_variable())),
        ("good/GrimoireTestCFF.otf", otf(&cff, 0, false)),
        // one of each kind the sheet refuses, beside one that passes with a glyph from Terminess
        ("refused/Blank.ttf", ttf(&empty_letters())),
        ("refused/Cut.ttf", ttf(&test_font("Regular", 400))[..700].to_vec()),
        ("refused/Symbols.ttf", ttf(&symbol_cmap())),
        ("refused/Web.woff2", woff2()),
        ("refused/WebInside.ttf", woff2()),
        ("refused/Borrowing.ttf", ttf(&cyclic_composite(false))),
    ];
    // enough fonts for two pages of the sheet
    for i in 1..=7 {
        let mut f = test_font("Regular", 400);
        f.family = format!("Grimoire Test {i}");
        let name: &'static str = Box::leak(format!("many/Test{i:02}.ttf").into_boxed_str());
        v.push((name, ttf(&f)));
    }
    v
}
