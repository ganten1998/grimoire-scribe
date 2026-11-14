//! zpieces: stb_image_write's zlib compressor (third_party/stb_image_write.h, stbi_zlib_compress) in
//! Rust, with the same bytes out, gathered in pieces of a fixed size.
//!
//! Why it is not stb's own any more for the exports (issue 79): stb gathers its output in one block
//! that it doubles through realloc. A Scribe-sized colour page that hardly compresses (13.8 MB of
//! RGB, about 13.9 MB out) ended in a block of 24 MiB, and an allocator that moves a block to grow
//! it held the 12 MiB one beside the new one while it copied: 12 MiB more at the peak on some runs
//! and not on others (measured on a Mac, 2026-11-13, by a library in front of realloc that moved
//! every large block; GitHub's macOS runner read 84 and 96 MiB for the same mirror). stb's output
//! block is a local of its function: there is no telling it a size.
//!
//! Here the output is a list of pieces of PIECE bytes, none of them ever grown or moved, so what is
//! held is what was written, and a stream that turns out longer than the bytes stored as they are
//! (stb's own test, made as soon as it can be known) is dropped before the stored one is written.
//!
//! The bytes are stb's, bit for bit: the fixed Huffman block, its hash of three bytes into 16384
//! lists of `2 * quality` places at most (half of a full list forgotten), its lazy match, its
//! stored blocks of 32767 bytes when compressing made it longer, the Adler-32 at the end. The tests
//! hold this module against stbi_zlib_compress itself. Even stb's stream for no bytes at all (a
//! header and a checksum with no block between them) is kept: nothing here calls it with none.

/// A piece's size. (On the Kindle's allocator a block of this size is mapped by itself and given
/// back by itself.)
pub const PIECE: usize = 256 << 10;

/// A zlib stream held in pieces.
#[derive(Default)]
pub struct ZStream {
    pieces: Vec<Vec<u8>>,
    len: usize,
}

impl ZStream {
    /// A stream that is already in one block (a record read back from the PDF cache).
    pub fn from_vec(v: Vec<u8>) -> ZStream {
        let len = v.len();
        ZStream { pieces: vec![v], len }
    }
    pub fn len(&self) -> usize {
        self.len
    }
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
    /// The stream, piece after piece.
    pub fn pieces(&self) -> impl Iterator<Item = &[u8]> + '_ {
        self.pieces.iter().map(|p| p.as_slice())
    }
    /// The bytes from `from` up to `to` (not included), piece after piece: a zip entry is the stream
    /// without its 2-byte header and its 4-byte checksum.
    pub fn range(&self, from: usize, to: usize) -> impl Iterator<Item = &[u8]> + '_ {
        let mut at = 0usize;
        self.pieces.iter().filter_map(move |p| {
            let (a, b) = (at, at + p.len());
            at = b;
            let (lo, hi) = (from.max(a), to.min(b));
            (lo < hi).then(|| &p[lo - a..hi - a])
        })
    }
    /// The stream in one block (the tests, and small streams).
    pub fn to_vec(&self) -> Vec<u8> {
        let mut v = Vec::with_capacity(self.len);
        for p in self.pieces() {
            v.extend_from_slice(p);
        }
        v
    }
}

/// The output being written: bytes into the last piece, a new piece when it is full.
struct Out {
    z: ZStream,
    /// a piece could not be had: nothing more is taken, and there is no stream
    failed: bool,
}

impl Out {
    fn piece(&mut self) -> bool {
        let mut p: Vec<u8> = Vec::new();
        if self.failed || p.try_reserve_exact(PIECE).is_err() {
            self.failed = true;
            return false;
        }
        self.z.pieces.push(p);
        true
    }
    #[inline]
    fn push(&mut self, b: u8) {
        match self.z.pieces.last_mut() {
            Some(p) if p.len() < PIECE => p.push(b),
            _ => {
                if !self.piece() {
                    return;
                }
                self.z.pieces.last_mut().unwrap().push(b);
            }
        }
        self.z.len += 1;
    }
    fn put(&mut self, mut s: &[u8]) {
        while !s.is_empty() {
            if self.z.pieces.last().is_none_or(|p| p.len() >= PIECE) && !self.piece() {
                return;
            }
            let p = self.z.pieces.last_mut().unwrap();
            let n = s.len().min(PIECE - p.len());
            p.extend_from_slice(&s[..n]);
            self.z.len += n;
            s = &s[n..];
        }
    }
    /// Back to the first `n` bytes (n is within the first piece): the pieces after it are given back.
    fn truncate(&mut self, n: usize) {
        self.z.pieces.truncate(1);
        if let Some(p) = self.z.pieces.first_mut() {
            p.truncate(n);
        }
        self.z.len = self.z.pieces.first().map_or(0, |p| p.len());
    }
}

/// stb's bit writer: codes go in from the low bit, whole bytes go out.
struct Bits {
    buf: u32,
    count: u32,
}

impl Bits {
    #[inline]
    fn add(&mut self, out: &mut Out, code: u32, bits: u32) {
        self.buf |= code << self.count;
        self.count += bits;
        while self.count >= 8 {
            out.push(self.buf as u8);
            self.buf >>= 8;
            self.count -= 8;
        }
    }
}

fn bitrev(mut code: u32, mut bits: u32) -> u32 {
    let mut r = 0;
    while bits > 0 {
        r = (r << 1) | (code & 1);
        code >>= 1;
        bits -= 1;
    }
    r
}

/// The fixed Huffman code of a literal or a length symbol (0 to 287), as (code, bits).
fn huff(n: u32) -> (u32, u32) {
    match n {
        0..=143 => (bitrev(0x30 + n, 8), 8),
        144..=255 => (bitrev(0x190 + n - 144, 9), 9),
        256..=279 => (bitrev(n - 256, 7), 7),
        _ => (bitrev(0xc0 + n - 280, 8), 8),
    }
}

const ZHASH: usize = 16384;

#[inline]
fn zhash(d: &[u8]) -> usize {
    let mut h = d[0] as u32 + ((d[1] as u32) << 8) + ((d[2] as u32) << 16);
    h ^= h << 3;
    h = h.wrapping_add(h >> 5);
    h ^= h << 4;
    h = h.wrapping_add(h >> 17);
    h ^= h << 25;
    h = h.wrapping_add(h >> 6);
    h as usize & (ZHASH - 1)
}

/// How many bytes at `a` and at `b` are the same, of `limit` and of 258 at most.
#[inline]
fn countm(data: &[u8], a: usize, b: usize, limit: usize) -> usize {
    let n = limit.min(258);
    data[a..a + n].iter().zip(&data[b..b + n]).take_while(|(x, y)| x == y).count()
}

static LENGTHC: [u16; 30] = [3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131, 163, 195, 227, 258, 259];
static LENGTHEB: [u8; 29] = [0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0];
static DISTC: [u16; 31] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537, 2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577, 32768,
];
static DISTEB: [u8; 30] = [0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13, 13];

/// stbi_zlib_compress(data, quality): the zlib stream of `data`, or None when the memory is not
/// there (or the data is past what stb's int length can say).
pub fn zlib_compress(data: &[u8], quality: i32) -> Option<ZStream> {
    let n = data.len();
    if n > i32::MAX as usize {
        return None;
    }
    let q = quality.max(5) as usize;
    // What the same bytes take stored as they are: a stream longer than this is not kept.
    let stored = n + 2 + n.div_ceil(32767) * 5;
    let mut out = Out { z: ZStream::default(), failed: false };
    let mut bits = Bits { buf: 0, count: 0 };
    out.push(0x78); // DEFLATE, a 32K window
    out.push(0x5e); // FLEVEL = 1
    bits.add(&mut out, 1, 1); // BFINAL = 1
    bits.add(&mut out, 1, 2); // BTYPE = 1: fixed Huffman
    let mut literal = [(0u32, 0u32); 256];
    for (v, l) in literal.iter_mut().enumerate() {
        *l = huff(v as u32);
    }
    // The places each hash was last seen at, oldest first: `2 * q` a hash at most.
    let cap = 2 * q;
    let mut places: Vec<u32> = Vec::new();
    let mut counts: Vec<u32> = Vec::new();
    if places.try_reserve_exact(ZHASH * cap).is_err() || counts.try_reserve_exact(ZHASH).is_err() {
        return None;
    }
    places.resize(ZHASH * cap, 0);
    counts.resize(ZHASH, 0);
    let mut worse = false;
    let mut i = 0usize;
    while i + 3 < n {
        // (longer already than stored and it can only grow: stb finds out at the end, with the same answer)
        if out.z.len > stored || out.failed {
            worse = true;
            break;
        }
        let h = zhash(&data[i..]);
        let (base, count) = (h * cap, counts[h] as usize);
        let mut best = 3usize;
        let mut bestloc: Option<usize> = None;
        for &p in &places[base..base + count] {
            let p = p as usize;
            if p + 32768 > i {
                // (within the window)
                let d = countm(data, p, i, n - i);
                if d >= best {
                    best = d;
                    bestloc = Some(p);
                }
            }
        }
        // a full list forgets its older half
        let mut count = count;
        if count == 2 * q {
            places.copy_within(base + q..base + 2 * q, base);
            count = q;
        }
        places[base + count] = i as u32;
        counts[h] = count as u32 + 1;
        if bestloc.is_some() {
            // "lazy matching": a better match at the next byte, and this byte goes as a literal
            let h = zhash(&data[i + 1..]);
            let (base, count) = (h * cap, counts[h] as usize);
            for &p in &places[base..base + count] {
                let p = p as usize;
                if p + 32767 > i && countm(data, p, i + 1, n - i - 1) > best {
                    bestloc = None;
                    break;
                }
            }
        }
        if let Some(loc) = bestloc {
            let d = i - loc; // the distance back
            let mut j = 0;
            while best > LENGTHC[j + 1] as usize - 1 {
                j += 1;
            }
            let (code, len) = huff(j as u32 + 257);
            bits.add(&mut out, code, len);
            if LENGTHEB[j] != 0 {
                bits.add(&mut out, (best - LENGTHC[j] as usize) as u32, LENGTHEB[j] as u32);
            }
            let mut j = 0;
            while d > DISTC[j + 1] as usize - 1 {
                j += 1;
            }
            bits.add(&mut out, bitrev(j as u32, 5), 5);
            if DISTEB[j] != 0 {
                bits.add(&mut out, (d - DISTC[j] as usize) as u32, DISTEB[j] as u32);
            }
            i += best;
        } else {
            let (code, len) = literal[data[i] as usize];
            bits.add(&mut out, code, len);
            i += 1;
        }
    }
    drop(places);
    drop(counts);
    if !worse {
        // the last bytes, the end of the block, and zero bits up to a whole byte
        for &b in &data[i..] {
            let (code, len) = literal[b as usize];
            bits.add(&mut out, code, len);
        }
        let (code, len) = huff(256);
        bits.add(&mut out, code, len);
        while bits.count != 0 {
            bits.add(&mut out, 0, 1);
        }
        worse = out.z.len > stored;
    }
    if worse {
        // stored as it is: the header again, then blocks of 32767 bytes
        out.truncate(2);
        let mut j = 0;
        while j < n {
            let len = (n - j).min(32767);
            out.push((n - j == len) as u8); // BFINAL, BTYPE = 0: not compressed
            out.push(len as u8);
            out.push((len >> 8) as u8);
            out.push(!len as u8);
            out.push((!len >> 8) as u8);
            out.put(&data[j..j + len]);
            j += len;
        }
    }
    // Adler-32 of the data
    let (mut s1, mut s2) = (1u32, 0u32);
    for block in data.chunks(5552) {
        for &b in block {
            s1 += b as u32;
            s2 += s1;
        }
        s1 %= 65521;
        s2 %= 65521;
    }
    for b in [(s2 >> 8) as u8, s2 as u8, (s1 >> 8) as u8, s1 as u8] {
        out.push(b);
    }
    if out.failed {
        return None;
    }
    Some(out.z)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// stb's own stream for the same bytes.
    fn stb(data: &[u8], quality: i32) -> Vec<u8> {
        let mut len: libc::c_int = 0;
        // SAFETY: stb reads data.len() bytes of data (never writes through the pointer) and returns
        // a malloc block of `len` bytes, copied and freed here.
        unsafe {
            let p = crate::ffi_stb::stbi_zlib_compress(data.as_ptr() as *mut u8, data.len() as libc::c_int, &mut len, quality);
            assert!(!p.is_null());
            let v = std::slice::from_raw_parts(p, len as usize).to_vec();
            libc::free(p as *mut libc::c_void);
            v
        }
    }

    fn same(name: &str, data: &[u8], quality: i32) {
        let ours = zlib_compress(data, quality).unwrap();
        let theirs = stb(data, quality);
        assert_eq!(ours.len(), theirs.len(), "{name} ({} bytes, quality {quality}): the length", data.len());
        assert!(ours.to_vec() == theirs, "{name} ({} bytes, quality {quality}): the bytes differ", data.len());
        assert!(ours.pieces().all(|p| p.len() <= PIECE));
    }

    /// A generator with no state but its seed (the same data every run).
    struct Rnd(u32);
    impl Rnd {
        fn next(&mut self) -> u32 {
            self.0 = self.0.wrapping_mul(1664525).wrapping_add(1013904223);
            self.0 >> 8
        }
        fn below(&mut self, n: u32) -> u32 {
            self.next() % n
        }
    }

    /// Bytes of several kinds, mixed: noise, runs, text from a small alphabet, copies of what came
    /// before at every distance up to past the window.
    fn mixed(len: usize, seed: u32) -> Vec<u8> {
        let mut r = Rnd(seed);
        let mut v: Vec<u8> = Vec::with_capacity(len + 300);
        while v.len() < len {
            match r.below(6) {
                0 => {
                    for _ in 0..1 + r.below(200) {
                        v.push(r.next() as u8);
                    }
                }
                1 => {
                    let b = r.next() as u8;
                    for _ in 0..1 + r.below(600) {
                        v.push(b);
                    }
                }
                2 => {
                    for _ in 0..1 + r.below(300) {
                        v.push(b"etaoin shrdlu\n"[r.below(14) as usize]);
                    }
                }
                3 if !v.is_empty() => {
                    // a copy from close by
                    let back = 1 + r.below(v.len().min(300) as u32) as usize;
                    for _ in 0..3 + r.below(300) {
                        v.push(v[v.len() - back]);
                    }
                }
                4 if v.len() > 32000 => {
                    // a copy from the window's edge, one side of it and the other
                    let back = 32760 + r.below(16) as usize;
                    if back <= v.len() {
                        for _ in 0..3 + r.below(300) {
                            v.push(v[v.len() - back]);
                        }
                    }
                }
                _ => {
                    for _ in 0..1 + r.below(40) {
                        v.push((r.below(4) * 85) as u8);
                    }
                }
            }
        }
        v.truncate(len);
        v
    }

    #[test]
    fn the_bytes_are_stbs_for_every_small_length() {
        // every length from none up, around the ends of the main loop (the last three bytes)
        for len in 0..=70 {
            same("mixed", &mixed(len, 7 + len as u32), 8);
            same("zeros", &vec![0u8; len], 8);
            same("noise", &(0..len).scan(Rnd(len as u32), |r, _| Some(r.next() as u8)).collect::<Vec<u8>>(), 8);
        }
    }

    #[test]
    fn the_bytes_are_stbs_for_data_of_every_kind() {
        for (k, len) in [200usize, 1000, 4096, 32766, 32767, 32768, 32769, 65534, 65535, 65536, 100_000, 300_000].into_iter().enumerate() {
            for quality in [5, 8, 13] {
                same("mixed", &mixed(len, 100 + k as u32), quality);
            }
            // one byte over and over: the longest matches, at distance 1
            same("one byte", &vec![0xa5u8; len], 8);
            // noise: longer compressed than stored, so stored (blocks of 32767)
            let noise: Vec<u8> = (0..len).scan(Rnd(9 + k as u32), |r, _| Some(r.next() as u8)).collect();
            same("noise", &noise, 8);
            // noise with five bits of it a byte (the colour memory test's photo): just longer than stored
            let photo: Vec<u8> = (0..len).scan(Rnd(31 + k as u32), |r, _| Some(((r.next() as u8) & 31).wrapping_add(96))).collect();
            same("photo", &photo, 8);
            // a period just inside and just outside the window
            for period in [32766usize, 32767, 32768, 32769] {
                let p: Vec<u8> = (0..len).map(|i| (((i % period) * 2654435761usize) >> 7) as u8).collect();
                same("periodic", &p, 8);
            }
        }
    }

    #[test]
    fn the_bytes_are_stbs_for_pages() {
        // a gray page: white paper, ruled lines, a little ink
        let (w, h) = (620usize, 827usize);
        let mut gray = vec![255u8; w * h];
        for y in (40..h).step_by(41) {
            gray[y * w..(y + 1) * w].fill(170);
        }
        let mut r = Rnd(5);
        for _ in 0..4000 {
            let at = r.below((w * h) as u32) as usize;
            gray[at] = r.below(120) as u8;
        }
        same("a gray page", &gray, 8);
        // a colour page that compresses (flat tints) and one that does not (a gradient with noise)
        let mut tint = vec![255u8; w * h * 3];
        let mut photo = vec![0u8; w * h * 3];
        for y in 0..h {
            for x in 0..w {
                let i = (y * w + x) * 3;
                if y % 120 < 3 || x % 240 < 2 {
                    tint[i..i + 3].copy_from_slice(&[90, 120, 200]);
                } else if y < 160 {
                    tint[i..i + 3].copy_from_slice(&[250, 210, 120]);
                }
                photo[i] = ((x * 200 / w) as u8).wrapping_add(r.next() as u8 & 31);
                photo[i + 1] = ((y * 200 / h) as u8).wrapping_add(r.next() as u8 & 31);
                photo[i + 2] = (((x + y) * 100 / (w + h)) as u8).wrapping_add(r.next() as u8 & 31);
            }
        }
        same("a tinted page", &tint, 8);
        same("a photo page", &photo, 8);
    }

    #[test]
    fn a_stream_is_held_in_pieces_and_read_back_in_ranges() {
        // more than two pieces of stored noise
        let noise: Vec<u8> = (0..2 * PIECE + 1000).scan(Rnd(3), |r, _| Some(r.next() as u8)).collect();
        let z = zlib_compress(&noise, 8).unwrap();
        let all = z.to_vec();
        assert_eq!(all.len(), z.len());
        assert!(z.pieces().count() >= 3);
        assert!(z.pieces().all(|p| p.len() <= PIECE));
        // every piece but the last is full: what is held is what was written, to within a piece
        let sizes: Vec<usize> = z.pieces().map(|p| p.len()).collect();
        assert!(sizes[..sizes.len() - 1].iter().all(|&s| s == PIECE));
        for (from, to) in [(0, z.len()), (2, z.len() - 4), (PIECE - 1, PIECE + 1), (PIECE, 2 * PIECE), (5, 5), (z.len() - 1, z.len())] {
            let got: Vec<u8> = z.range(from, to).flatten().copied().collect();
            assert!(got == all[from..to], "the range {from}..{to}");
        }
        // one block read back from a file is a stream too
        let one = ZStream::from_vec(all.clone());
        assert_eq!((one.len(), one.pieces().count()), (all.len(), 1));
        assert!(one.to_vec() == all);
    }
}
