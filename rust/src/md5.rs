//! MD5 (RFC 1321) and KOReader's partial MD5, the key of a book (DESIGN.md 3.13). About 80 lines of
//! our own, no crate: the key is not a secret, only the name KOReader and Grimoire give one file, so
//! its notebook is found again whichever of them made it.
//!
//! KOReader's `util.partialMD5` hashes 1 KiB samples taken at `lshift(1024, 2 * i)` for `i = -1 .. 10`,
//! stopping at the first read that finds nothing. LuaJIT's `lshift(1024, -2)` masks the count to 30 and
//! overflows 32 bits to 0, so the offsets are 0, 1 KiB, 4 KiB, 16 KiB ... 1 GiB: twelve samples at most.

/// Per-round shift amounts.
const S: [u32; 64] = [
    7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 5, 9, 14, 20, 5, 9, 14, 20, 5, 9, 14, 20, 5, 9, 14, 20, 4, 11, 16, 23, 4, 11,
    16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21,
];

/// floor(2^32 * abs(sin(i + 1))), as RFC 1321 tabulates it.
const K: [u32; 64] = [
    0xd76aa478, 0xe8c7b756, 0x242070db, 0xc1bdceee, 0xf57c0faf, 0x4787c62a, 0xa8304613, 0xfd469501, 0x698098d8, 0x8b44f7af, 0xffff5bb1,
    0x895cd7be, 0x6b901122, 0xfd987193, 0xa679438e, 0x49b40821, 0xf61e2562, 0xc040b340, 0x265e5a51, 0xe9b6c7aa, 0xd62f105d, 0x02441453,
    0xd8a1e681, 0xe7d3fbc8, 0x21e1cde6, 0xc33707d6, 0xf4d50d87, 0x455a14ed, 0xa9e3e905, 0xfcefa3f8, 0x676f02d9, 0x8d2a4c8a, 0xfffa3942,
    0x8771f681, 0x6d9d6122, 0xfde5380c, 0xa4beea44, 0x4bdecfa9, 0xf6bb4b60, 0xbebfbc70, 0x289b7ec6, 0xeaa127fa, 0xd4ef3085, 0x04881d05,
    0xd9d4d039, 0xe6db99e5, 0x1fa27cf8, 0xc4ac5665, 0xf4292244, 0x432aff97, 0xab9423a7, 0xfc93a039, 0x655b59c3, 0x8f0ccc92, 0xffeff47d,
    0x85845dd1, 0x6fa87e4f, 0xfe2ce6e0, 0xa3014314, 0x4e0811a1, 0xf7537e82, 0xbd3af235, 0x2ad7d2bb, 0xeb86d391,
];

/// An MD5 being computed.
pub struct Md5 {
    state: [u32; 4],
    buf: [u8; 64],
    fill: usize,
    len: u64,
}

impl Default for Md5 {
    fn default() -> Self {
        Self::new()
    }
}

impl Md5 {
    pub fn new() -> Md5 {
        Md5 { state: [0x67452301, 0xefcdab89, 0x98badcfe, 0x10325476], buf: [0; 64], fill: 0, len: 0 }
    }

    fn block(&mut self, b: &[u8]) {
        let mut m = [0u32; 16];
        for (i, w) in m.iter_mut().enumerate() {
            *w = u32::from_le_bytes([b[4 * i], b[4 * i + 1], b[4 * i + 2], b[4 * i + 3]]);
        }
        let [mut a, mut bb, mut c, mut d] = self.state;
        for i in 0..64 {
            let (f, g) = match i / 16 {
                0 => ((bb & c) | (!bb & d), i),
                1 => ((d & bb) | (!d & c), (5 * i + 1) % 16),
                2 => (bb ^ c ^ d, (3 * i + 5) % 16),
                _ => (c ^ (bb | !d), (7 * i) % 16),
            };
            let t = a.wrapping_add(f).wrapping_add(K[i]).wrapping_add(m[g]);
            a = d;
            d = c;
            c = bb;
            bb = bb.wrapping_add(t.rotate_left(S[i]));
        }
        self.state[0] = self.state[0].wrapping_add(a);
        self.state[1] = self.state[1].wrapping_add(bb);
        self.state[2] = self.state[2].wrapping_add(c);
        self.state[3] = self.state[3].wrapping_add(d);
    }

    pub fn update(&mut self, mut data: &[u8]) {
        self.len = self.len.wrapping_add(data.len() as u64);
        if self.fill > 0 {
            let take = (64 - self.fill).min(data.len());
            self.buf[self.fill..self.fill + take].copy_from_slice(&data[..take]);
            self.fill += take;
            data = &data[take..];
            if self.fill < 64 {
                return;
            }
            let b = self.buf;
            self.block(&b);
            self.fill = 0;
        }
        while data.len() >= 64 {
            self.block(&data[..64]);
            data = &data[64..];
        }
        self.buf[..data.len()].copy_from_slice(data);
        self.fill = data.len();
    }

    pub fn finish(mut self) -> [u8; 16] {
        let bits = self.len.wrapping_mul(8);
        self.update(&[0x80]);
        while self.fill != 56 {
            self.update(&[0]);
        }
        self.update(&bits.to_le_bytes());
        let mut out = [0u8; 16];
        for (i, w) in self.state.iter().enumerate() {
            out[4 * i..4 * i + 4].copy_from_slice(&w.to_le_bytes());
        }
        out
    }
}

/// The 32 lower-case hex digits of a digest.
pub fn md5_hex(d: &[u8; 16]) -> String {
    d.iter().map(|b| format!("{b:02x}")).collect()
}

/// MD5 of `data`, as hex.
pub fn md5_of(data: &[u8]) -> String {
    let mut m = Md5::new();
    m.update(data);
    md5_hex(&m.finish())
}

/// The offsets of KOReader's partial MD5 samples, and each one's size.
pub const PARTIAL_SAMPLE: usize = 1024;
pub fn partial_offsets() -> impl Iterator<Item = u64> {
    // i = -1 is lshift(1024, -2) = 0 in LuaJIT; i = 0 .. 10 is 1024 << 2i
    std::iter::once(0u64).chain((0..=10u32).map(|i| 1024u64 << (2 * i)))
}

/// KOReader's partial MD5 of a file of any size, given a positional read (`read_at(buf, offset)`: the bytes
/// it could read, 0 at or past the end). Twelve reads of 1 KiB at fixed offsets and no parsing: safe for the
/// UI process and the writer to compute. An error reading counts as the end, as a nil from Lua's `read` does.
pub fn partial_md5(mut read_at: impl FnMut(&mut [u8], u64) -> usize) -> String {
    let mut m = Md5::new();
    let mut buf = [0u8; PARTIAL_SAMPLE];
    for off in partial_offsets() {
        let mut got = 0;
        while got < PARTIAL_SAMPLE {
            let n = read_at(&mut buf[got..], off + got as u64);
            if n == 0 {
                break;
            }
            got += n;
        }
        if got == 0 {
            break;
        }
        m.update(&buf[..got]);
    }
    md5_hex(&m.finish())
}

/// The partial MD5 of an open file (pread at each offset).
pub fn partial_md5_fd(fd: i32) -> String {
    partial_md5(|buf, off| {
        // SAFETY: pread writes at most buf.len() bytes into buf; fd is the caller's open descriptor.
        let n = unsafe { libc::pread(fd, buf.as_mut_ptr() as *mut libc::c_void, buf.len(), off as libc::off_t) };
        if n < 0 {
            0
        } else {
            n as usize
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // RFC 1321, A.5: the test suite.
    #[test]
    fn rfc_1321_vectors() {
        let v: [(&str, &str); 7] = [
            ("", "d41d8cd98f00b204e9800998ecf8427e"),
            ("a", "0cc175b9c0f1b6a831c399e269772661"),
            ("abc", "900150983cd24fb0d6963f7d28e17f72"),
            ("message digest", "f96b697d7cb7938d525a2f31aaf161d0"),
            ("abcdefghijklmnopqrstuvwxyz", "c3fcd3d76192e4007dfb496cca67e13b"),
            ("ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789", "d174ab98d277d9f5a5611c2c9f419d9f"),
            ("12345678901234567890123456789012345678901234567890123456789012345678901234567890", "57edf4a22be3c955ac49da2e2107b67a"),
        ];
        for (s, h) in v {
            assert_eq!(md5_of(s.as_bytes()), h, "{s:?}");
        }
    }

    // Input given in odd pieces gives the same digest as in one (the buffer's seams).
    #[test]
    fn pieces_make_the_same_digest() {
        let data: Vec<u8> = (0..1000u32).map(|i| (i * 7 + 3) as u8).collect();
        let one = md5_of(&data);
        for step in [1usize, 3, 63, 64, 65, 200] {
            let mut m = Md5::new();
            for c in data.chunks(step) {
                m.update(c);
            }
            assert_eq!(md5_hex(&m.finish()), one, "step {step}");
        }
    }

    // A file whose byte at offset i is the low byte of (i * 31 + i / 251): sparse, generated, never stored.
    fn pattern(size: u64) -> impl FnMut(&mut [u8], u64) -> usize {
        move |buf, off| {
            if off >= size {
                return 0;
            }
            let n = (buf.len() as u64).min(size - off) as usize;
            for (k, b) in buf[..n].iter_mut().enumerate() {
                let i = off + k as u64;
                *b = (i.wrapping_mul(31).wrapping_add(i / 251)) as u8;
            }
            n
        }
    }

    // Values computed once with an independent implementation of util.partialMD5's algorithm
    // (Python's hashlib over 1 KiB samples at 0, 1 KiB, 4 KiB ... 1 GiB, stopping at the first empty read)
    // for these generated files: 0 B, 1 KiB, 5 KiB, 2 MiB and 70 MiB.
    #[test]
    fn partial_md5_matches_koreaders_algorithm() {
        let vectors: [(u64, &str); 6] = [
            (0, "d41d8cd98f00b204e9800998ecf8427e"),
            (1, PM_1),
            (1024, PM_1K),
            (5 * 1024, PM_5K),
            (2 << 20, PM_2M),
            (70 << 20, PM_70M),
        ];
        for (size, want) in vectors {
            assert_eq!(partial_md5(pattern(size)), want, "{size} bytes");
        }
    }

    // The first read past the end ends it: a file of 1 KiB + 1 holds two samples (0, and a byte at 1 KiB).
    #[test]
    fn the_first_empty_read_stops_the_sampling() {
        let mut offs = Vec::new();
        partial_md5(|buf, off| {
            offs.push(off);
            pattern(1025)(buf, off)
        });
        // the sample at 0 (one read of 1024), the sample at 1 KiB (a read of 1 byte, then the next read for
        // the rest: nothing), and then the sample at 4 KiB finds nothing: sampling stops there
        assert_eq!(offs, vec![0, 1024, 1025, 4096]);
    }

    #[test]
    fn the_offsets_are_koreaders() {
        let o: Vec<u64> = partial_offsets().collect();
        assert_eq!(o.len(), 12);
        assert_eq!(&o[..4], &[0, 1024, 4096, 16384]);
        assert_eq!(o[11], 1 << 30);
    }

    #[test]
    fn no_em_dash() {
        assert!(!include_str!("md5.rs").contains('\u{2014}'));
    }

    const PM_1: &str = "93b885adfe0da089cdf634904fd59f71";
    const PM_1K: &str = "19fa36fd104d1b15e6e46b88444a9e73";
    const PM_5K: &str = "9884fcffb148c77e94a49c7af05982d2";
    const PM_2M: &str = "ddbe197ff9c96d95f1d8b6e6ce31258e";
    const PM_70M: &str = "97038f47cf17c4a1a9a33966cecd438a";
}
