// beta_format: the page reader and writer of the released Grimoire 1.0.0-beta, frozen, so tests
// can ask what 1.0.0-beta does with a file a later version writes. 1.0.0-rc.1 reads and writes
// pages the same way (its store.c, paper.c and brush.c are identical to 1.0.0-beta's).
//
// Copied from `git show v1.0.0-beta:src/store.c` (page_serialize, page_deserialize and what
// store_load_page does with a page it cannot read), src/paper.c (paper_sanitize and the custom
// music decoding it needs) and src/brush.c (the parameter ranges the reader clamps to), renamed
// beta_* and working on plain bytes and structs of their own, with no dependency on the current
// sources. Never update this file: 1.0.0-beta is in people's hands as it is.
// (tests/beta_format.h, in Rust: the reader and writer keep the C's names and its bytes.)
#![allow(dead_code)]

pub const BETA_PAPER_MUSIC: u32 = 3;
pub const BETA_PAPER_BLANK: u32 = 4;
pub const BETA_PAPER_COUNT: u32 = 17;
pub const BETA_PAPER_CUSTOM_FLAG: u32 = 0x40000000;
pub const BETA_PART_NONE: i32 = 0;
pub const BETA_PART_COUNT: i32 = 12;
pub const BETA_MUSIC_MAX_PARTS: i32 = 6;
pub const BETA_MUSIC_MAX_SYSTEMS: i32 = 12;
pub const BETA_TOOL_COUNT: u32 = 10;

#[derive(Clone, Copy, Debug, Default)]
pub struct BetaPoint {
    pub x: f32,
    pub y: f32,
    pub pressure: f32,
    pub tilt_x: f32,
    pub tilt_y: f32,
    pub t_ms: u32,
}

#[derive(Clone, Debug, Default)]
pub struct BetaStroke {
    pub tool: u8,
    pub seed: u32,
    /// ToolParams field order
    pub params: [f32; 13],
    /// the points (the C's n and pts)
    pub pts: Vec<BetaPoint>,
}

#[derive(Clone, Debug, Default)]
pub struct BetaPage {
    /// always a kind 1.0.0-beta knows (sanitized on read)
    pub paper: u32,
    pub w: u32,
    pub h: u32,
    /// the strokes (the C's n and strokes)
    pub strokes: Vec<BetaStroke>,
}

pub fn beta_clampf(v: f32, lo: f32, hi: f32) -> f32 {
    if v < lo {
        lo
    } else if v > hi {
        hi
    } else {
        v
    }
}

// src/brush.c k_params: {min, max} per ToolParams field, in field order.
pub const BETA_PARAM_RANGE: [[f32; 2]; 13] = [
    [0.10, 12.0],
    [0.10, 12.0],
    [0.0, 1.0],
    [0.0, 1.0],
    [0.0, 0.5],
    [0.05, 1.0],
    [0.25, 4.0],
    [0.0, 85.0],
    [5.0, 90.0],
    [0.0, 4.0],
    [0.0, 1.0],
    [0.0, 1.0],
    [0.0, 1.0],
];

// ---- src/paper.c ----

pub fn beta_paper_is_custom(k: u32) -> bool {
    (k & 0xF0000000) == BETA_PAPER_CUSTOM_FLAG
}

pub fn beta_music_decode(v: u32) -> bool {
    if !beta_paper_is_custom(v) {
        return false;
    }
    let systems = ((v >> 24) & 15) as i32;
    let mut n = 0;
    for i in 0..BETA_MUSIC_MAX_PARTS {
        let p = ((v >> (4 * i)) & 15) as i32;
        if p == BETA_PART_NONE {
            break;
        }
        if p >= BETA_PART_COUNT {
            return false;
        }
        n += 1;
    }
    !(n == 0 || systems < 1 || systems > BETA_MUSIC_MAX_SYSTEMS)
}

pub fn beta_paper_sanitize(v: u32, fallback: u32) -> u32 {
    if v < BETA_PAPER_COUNT {
        return v;
    }
    if beta_music_decode(v) {
        return v;
    }
    fallback
}

// ---- src/store.c: page format ----

fn beta_w_u16(w: &mut Vec<u8>, v: u16) {
    w.extend_from_slice(&[v as u8, (v >> 8) as u8]);
}
fn beta_w_u32(w: &mut Vec<u8>, v: u32) {
    w.extend_from_slice(&[v as u8, (v >> 8) as u8, (v >> 16) as u8, (v >> 24) as u8]);
}
fn beta_w_f32(w: &mut Vec<u8>, f: f32) {
    beta_w_u32(w, f.to_bits());
}

/// page_serialize: version 2 only for custom music; the pad bytes are always written as 0, and
/// nothing follows the last stroke.
pub fn beta_page_serialize(p: &BetaPage) -> Option<Vec<u8>> {
    let mut w: Vec<u8> = Vec::new();
    let custom = beta_paper_is_custom(p.paper);
    w.extend_from_slice(b"QPG1");
    beta_w_u16(&mut w, if custom { 2 } else { 1 });
    beta_w_u16(&mut w, if custom { BETA_PAPER_MUSIC as u16 } else { p.paper as u16 });
    beta_w_u32(&mut w, p.w);
    beta_w_u32(&mut w, p.h);
    if custom {
        beta_w_u32(&mut w, p.paper);
    }
    beta_w_u32(&mut w, p.strokes.len() as u32);
    for s in p.strokes.iter() {
        w.push(s.tool);
        w.push(0);
        w.push(0);
        w.push(0);
        beta_w_u32(&mut w, s.seed);
        for k in 0..13 {
            beta_w_f32(&mut w, s.params[k]);
        }
        beta_w_u32(&mut w, s.pts.len() as u32);
        for q in s.pts.iter() {
            beta_w_f32(&mut w, q.x);
            beta_w_f32(&mut w, q.y);
            beta_w_u16(&mut w, ((beta_clampf(q.pressure, 0.0, 1.0) * 65535.0).round_ties_even() as i64) as u16);
            beta_w_u16(&mut w, (((beta_clampf(q.tilt_x, -90.0, 90.0) * 100.0).round_ties_even() as i64) as i16) as u16);
            beta_w_u16(&mut w, (((beta_clampf(q.tilt_y, -90.0, 90.0) * 100.0).round_ties_even() as i64) as i16) as u16);
            beta_w_u32(&mut w, q.t_ms);
        }
    }
    Some(w)
}

struct BetaR<'a> {
    d: &'a [u8],
    n: usize,
    pos: usize,
    err: bool,
}

fn beta_r_take<'a>(r: &mut BetaR<'a>, n: usize) -> Option<&'a [u8]> {
    if r.err || r.n - r.pos < n {
        r.err = true;
        return None;
    }
    let p = &r.d[r.pos..r.pos + n];
    r.pos += n;
    Some(p)
}
fn beta_r_u8(r: &mut BetaR) -> u8 {
    beta_r_take(r, 1).map_or(0, |p| p[0])
}
fn beta_r_u16(r: &mut BetaR) -> u16 {
    beta_r_take(r, 2).map_or(0, |p| (p[0] as u16) | (p[1] as u16) << 8)
}
fn beta_r_u32(r: &mut BetaR) -> u32 {
    beta_r_take(r, 4).map_or(0, |p| (p[0] as u32) | (p[1] as u32) << 8 | (p[2] as u32) << 16 | (p[3] as u32) << 24)
}
fn beta_r_f32(r: &mut BetaR) -> f32 {
    let f = f32::from_bits(beta_r_u32(r));
    if f.is_finite() {
        f
    } else {
        0.0
    }
}

pub const BETA_STROKE_HEADER_BYTES: usize = 4 + 4 + 13 * 4 + 4;
pub const BETA_POINT_BYTES: usize = 4 + 4 + 2 + 2 + 2 + 4;

/// page_deserialize: the page, or None when it is rejected (bad magic, a version other than 1 or
/// 2, impossible sizes, a tool id of 10 or more, or a cut-off file). The pad bytes and anything
/// after the last stroke are not read; parameters are clamped to their ranges; the paper is
/// sanitized.
pub fn beta_page_deserialize(buf: &[u8]) -> Option<BetaPage> {
    let len = buf.len();
    let mut r = BetaR { d: buf, n: len, pos: 0, err: false };
    let m = beta_r_take(&mut r, 4)?;
    if m != b"QPG1" {
        return None;
    }
    let ver = beta_r_u16(&mut r);
    let mut paper = beta_r_u16(&mut r) as u32;
    let w = beta_r_u32(&mut r);
    let h = beta_r_u32(&mut r);
    if ver == 2 {
        paper = beta_r_u32(&mut r);
    }
    let ns = beta_r_u32(&mut r);
    if r.err || (ver != 1 && ver != 2) || w == 0 || h == 0 || w > 20000 || h > 20000 {
        return None;
    }
    if ns as usize > (len - r.pos) / BETA_STROKE_HEADER_BYTES {
        return None;
    }
    let mut p = BetaPage { paper: beta_paper_sanitize(paper, BETA_PAPER_BLANK), w, h, strokes: Vec::new() };
    let mut i = 0;
    while i < ns && !r.err {
        i += 1;
        let tool = beta_r_u8(&mut r);
        beta_r_take(&mut r, 3);
        let seed = beta_r_u32(&mut r);
        let mut prm = [0f32; 13];
        for k in 0..13 {
            prm[k] = beta_r_f32(&mut r);
        }
        let np = beta_r_u32(&mut r) as usize;
        if r.err || tool as u32 >= BETA_TOOL_COUNT || np > (len - r.pos) / BETA_POINT_BYTES {
            r.err = true;
            break;
        }
        let mut s = BetaStroke { tool, seed, params: [0.0; 13], pts: Vec::new() };
        for k in 0..13 {
            s.params[k] = beta_clampf(prm[k], BETA_PARAM_RANGE[k][0], BETA_PARAM_RANGE[k][1]);
        }
        for _ in 0..np {
            let mut q = BetaPoint::default();
            q.x = beta_clampf(beta_r_f32(&mut r), -10000.0, 30000.0);
            q.y = beta_clampf(beta_r_f32(&mut r), -10000.0, 30000.0);
            q.pressure = beta_r_u16(&mut r) as f32 / 65535.0;
            q.tilt_x = (beta_r_u16(&mut r) as i16) as f32 / 100.0;
            q.tilt_y = (beta_r_u16(&mut r) as i16) as f32 / 100.0;
            q.t_ms = beta_r_u32(&mut r);
            s.pts.push(q);
        }
        p.strokes.push(s);
    }
    if r.err {
        return None;
    }
    Some(p)
}

/// What 1.0.0-beta writes back after opening a page and saving it (its autosave after any edit):
/// read, then written. None if it rejects the page.
pub fn beta_resave(buf: &[u8]) -> Option<Vec<u8>> {
    let p = beta_page_deserialize(buf)?;
    beta_page_serialize(&p)
}

/// What store_load_page does with a page file it cannot read: it renames it to
/// <path>.damaged-<time> (and shows a blank page in its place). Returns the reader's result
/// (0, or -1 for a page it rejects).
pub fn beta_load_page_file(path: &str, now: i64) -> i32 {
    // a missing page is a blank page, nothing set aside
    let Ok(buf) = std::fs::read(path) else { return 0 };
    if beta_page_deserialize(&buf).is_some() {
        0
    } else {
        let aside = format!("{path}.damaged-{now}");
        let _ = std::fs::rename(path, aside);
        -1
    }
}
