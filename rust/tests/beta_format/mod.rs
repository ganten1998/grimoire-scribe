// beta_format: tests/beta_format.h for the Rust ports of the C unit tests. The page reader and
// writer of the released Grimoire 1.0.0-beta, frozen, so tests can ask what 1.0.0-beta does with a
// file a later version writes. 1.0.0-rc.1 reads and writes pages the same way (its store.c,
// paper.c and brush.c are identical to 1.0.0-beta's).
//
// Copied from `git show v1.0.0-beta:src/store.c` (page_serialize, page_deserialize and what
// store_load_page does with a page it cannot read), src/paper.c (paper_sanitize and the custom
// music decoding it needs) and src/brush.c (the parameter ranges the reader clamps to), renamed
// beta_* and working on plain bytes and structs of their own, with no dependency on the current
// sources. Never update this file: 1.0.0-beta is in people's hands as it is.
#![allow(dead_code, clippy::manual_range_contains)]

pub const BETA_PAPER_MUSIC: u32 = 3;
pub const BETA_PAPER_BLANK: u32 = 4;
pub const BETA_PAPER_COUNT: u32 = 17;
pub const BETA_PAPER_CUSTOM_FLAG: u32 = 0x40000000;
pub const BETA_PART_NONE: i32 = 0;
pub const BETA_PART_COUNT: i32 = 12;
pub const BETA_MUSIC_MAX_PARTS: usize = 6;
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
    /// the points (the C's pts and n)
    pub pts: Vec<BetaPoint>,
}

#[derive(Clone, Debug, Default)]
pub struct BetaPage {
    /// always a kind 1.0.0-beta knows (sanitized on read)
    pub paper: u32,
    pub w: u32,
    pub h: u32,
    /// the strokes (the C's strokes and n)
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

// page_serialize: version 2 only for custom music; the pad bytes are always written as 0, and
// nothing follows the last stroke. 0 (out holds the bytes), or -1.
pub fn beta_page_serialize(p: &BetaPage, out: &mut Vec<u8>) -> i32 {
    let mut w: Vec<u8> = Vec::new();
    let custom = beta_paper_is_custom(p.paper);
    w.extend_from_slice(b"QPG1");
    w.extend_from_slice(&(if custom { 2u16 } else { 1u16 }).to_le_bytes());
    w.extend_from_slice(&(if custom { BETA_PAPER_MUSIC as u16 } else { p.paper as u16 }).to_le_bytes());
    w.extend_from_slice(&p.w.to_le_bytes());
    w.extend_from_slice(&p.h.to_le_bytes());
    if custom {
        w.extend_from_slice(&p.paper.to_le_bytes());
    }
    w.extend_from_slice(&(p.strokes.len() as u32).to_le_bytes());
    for s in &p.strokes {
        w.extend_from_slice(&[s.tool, 0, 0, 0]);
        w.extend_from_slice(&s.seed.to_le_bytes());
        for k in 0..13 {
            w.extend_from_slice(&s.params[k].to_bits().to_le_bytes());
        }
        w.extend_from_slice(&(s.pts.len() as u32).to_le_bytes());
        for q in &s.pts {
            w.extend_from_slice(&q.x.to_bits().to_le_bytes());
            w.extend_from_slice(&q.y.to_bits().to_le_bytes());
            // lrintf: to nearest, ties to even
            w.extend_from_slice(&(((beta_clampf(q.pressure, 0.0, 1.0) * 65535.0).round_ties_even() as i64) as u16).to_le_bytes());
            w.extend_from_slice(&(((beta_clampf(q.tilt_x, -90.0, 90.0) * 100.0).round_ties_even() as i64) as i16 as u16).to_le_bytes());
            w.extend_from_slice(&(((beta_clampf(q.tilt_y, -90.0, 90.0) * 100.0).round_ties_even() as i64) as i16 as u16).to_le_bytes());
            w.extend_from_slice(&q.t_ms.to_le_bytes());
        }
    }
    *out = w;
    0
}

struct BetaR<'a> {
    d: &'a [u8],
    pos: usize,
    err: bool,
}

impl<'a> BetaR<'a> {
    fn take(&mut self, n: usize) -> Option<&'a [u8]> {
        if self.err || self.d.len() - self.pos < n {
            self.err = true;
            return None;
        }
        let p = &self.d[self.pos..self.pos + n];
        self.pos += n;
        Some(p)
    }
    fn u8(&mut self) -> u8 {
        self.take(1).map_or(0, |p| p[0])
    }
    fn u16(&mut self) -> u16 {
        self.take(2).map_or(0, |p| u16::from_le_bytes([p[0], p[1]]))
    }
    fn u32(&mut self) -> u32 {
        self.take(4).map_or(0, |p| u32::from_le_bytes([p[0], p[1], p[2], p[3]]))
    }
    fn f32(&mut self) -> f32 {
        let f = f32::from_bits(self.u32());
        if f.is_finite() {
            f
        } else {
            0.0
        }
    }
}

pub const BETA_STROKE_HEADER_BYTES: usize = 4 + 4 + 13 * 4 + 4;
pub const BETA_POINT_BYTES: usize = 4 + 4 + 2 + 2 + 2 + 4;

// page_deserialize: 0, or -1 when the page is rejected (bad magic, a version other than 1 or 2,
// impossible sizes, a tool id of 10 or more, or a cut-off file). The pad bytes and anything after
// the last stroke are not read; parameters are clamped to their ranges; the paper is sanitized.
pub fn beta_page_deserialize(buf: &[u8], out: &mut BetaPage) -> i32 {
    *out = BetaPage::default();
    let len = buf.len();
    let mut r = BetaR { d: buf, pos: 0, err: false };
    let m = r.take(4);
    if m != Some(&b"QPG1"[..]) {
        return -1;
    }
    let ver = r.u16();
    let mut paper = r.u16() as u32;
    let w = r.u32();
    let h = r.u32();
    if ver == 2 {
        paper = r.u32();
    }
    let ns = r.u32();
    if r.err || (ver != 1 && ver != 2) || w == 0 || h == 0 || w > 20000 || h > 20000 {
        return -1;
    }
    if ns as usize > (len - r.pos) / BETA_STROKE_HEADER_BYTES {
        return -1;
    }
    let mut p = BetaPage { paper: beta_paper_sanitize(paper, BETA_PAPER_BLANK), w, h, strokes: Vec::new() };
    let mut i = 0;
    while i < ns && !r.err {
        let tool = r.u8();
        r.take(3);
        let seed = r.u32();
        let mut prm = [0.0f32; 13];
        for k in 0..13 {
            prm[k] = r.f32();
        }
        let np = r.u32();
        if r.err || tool as u32 >= BETA_TOOL_COUNT || np as usize > (len - r.pos) / BETA_POINT_BYTES {
            r.err = true;
            break;
        }
        let mut s = BetaStroke { tool, seed, ..Default::default() };
        for k in 0..13 {
            s.params[k] = beta_clampf(prm[k], BETA_PARAM_RANGE[k][0], BETA_PARAM_RANGE[k][1]);
        }
        for _ in 0..np {
            let x = beta_clampf(r.f32(), -10000.0, 30000.0);
            let y = beta_clampf(r.f32(), -10000.0, 30000.0);
            let pressure = r.u16() as f32 / 65535.0;
            let tilt_x = r.u16() as i16 as f32 / 100.0;
            let tilt_y = r.u16() as i16 as f32 / 100.0;
            let t_ms = r.u32();
            s.pts.push(BetaPoint { x, y, pressure, tilt_x, tilt_y, t_ms });
        }
        p.strokes.push(s);
        i += 1;
    }
    if r.err {
        return -1;
    }
    *out = p;
    0
}

// What 1.0.0-beta writes back after opening a page and saving it (its autosave after any edit):
// read, then written. -1 if it rejects the page.
pub fn beta_resave(buf: &[u8], out: &mut Vec<u8>) -> i32 {
    let mut p = BetaPage::default();
    if beta_page_deserialize(buf, &mut p) != 0 {
        return -1;
    }
    beta_page_serialize(&p, out)
}

// What store_load_page does with a page file it cannot read: it renames it to
// <path>.damaged-<time> (and shows a blank page in its place). Returns the reader's result.
pub fn beta_load_page_file(path: &str, now: i64) -> i32 {
    let Ok(buf) = std::fs::read(path) else { return 0 }; // a missing page is a blank page, nothing set aside
    let mut p = BetaPage::default();
    let rc = beta_page_deserialize(&buf, &mut p);
    if rc != 0 {
        let aside = format!("{path}.damaged-{now}");
        let _ = std::fs::rename(path, aside);
    }
    rc
}
