//! SHA-256 (FIPS 180-4), the digest that says two files hold the same bytes. About 150 lines of our own, no
//! crate, laid out as md5.rs is: anyone can check a digest against `sha256sum` or `shasum -a 256`.
//!
//! What is already in memory goes through `sha256` (the 32 bytes) or `sha256_of` (their hex). A caller with
//! a big file reads it in pieces and gives each to `Sha256::update`, in a background job: the whole of it is
//! never in memory, and the UI never waits for it.

/// The first 32 bits of the fractional parts of the cube roots of the first 64 primes (FIPS 180-4, 4.2.2).
const K: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be,
    0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa,
    0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85,
    0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3,
    0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f,
    0x682e6ff3, 0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

/// A SHA-256 being computed.
pub struct Sha256 {
    state: [u32; 8],
    buf: [u8; 64],
    fill: usize,
    len: u64,
}

impl Default for Sha256 {
    fn default() -> Self {
        Self::new()
    }
}

impl Sha256 {
    /// Starts from the first 32 bits of the fractional parts of the square roots of the first 8 primes (5.3.3).
    pub fn new() -> Sha256 {
        Sha256 {
            state: [0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19],
            buf: [0; 64],
            fill: 0,
            len: 0,
        }
    }

    fn block(&mut self, b: &[u8]) {
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([b[4 * i], b[4 * i + 1], b[4 * i + 2], b[4 * i + 3]]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16].wrapping_add(s0).wrapping_add(w[i - 7]).wrapping_add(s1);
        }
        let [mut a, mut bb, mut c, mut d, mut e, mut f, mut g, mut h] = self.state;
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ (!e & g);
            let t1 = h.wrapping_add(s1).wrapping_add(ch).wrapping_add(K[i]).wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & bb) ^ (a & c) ^ (bb & c);
            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = bb;
            bb = a;
            a = t1.wrapping_add(s0.wrapping_add(maj));
        }
        for (s, v) in self.state.iter_mut().zip([a, bb, c, d, e, f, g, h]) {
            *s = s.wrapping_add(v);
        }
    }

    /// Whole blocks are compressed straight from `data`; only a block's unfinished part is kept.
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

    /// The padding (a 1 bit, zeros, the length in bits as 64 big-endian bits), then the eight words.
    pub fn finish(mut self) -> [u8; 32] {
        let bits = self.len.wrapping_mul(8);
        self.update(&[0x80]);
        while self.fill != 56 {
            self.update(&[0]);
        }
        self.update(&bits.to_be_bytes());
        let mut out = [0u8; 32];
        for (i, w) in self.state.iter().enumerate() {
            out[4 * i..4 * i + 4].copy_from_slice(&w.to_be_bytes());
        }
        out
    }
}

/// SHA-256 of `data`.
pub fn sha256(data: &[u8]) -> [u8; 32] {
    let mut s = Sha256::new();
    s.update(data);
    s.finish()
}

/// The 64 lower-case hex digits of a digest.
pub fn hex32(d: &[u8; 32]) -> String {
    d.iter().map(|b| format!("{b:02x}")).collect()
}

/// The digest that 64 hex digits spell: exactly 64 of `0-9a-f`, and anything else is None (a text that
/// `hex32` did not write is not a digest).
pub fn unhex32(s: &str) -> Option<[u8; 32]> {
    let b = s.as_bytes();
    if b.len() != 64 {
        return None;
    }
    let digit = |c: u8| match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        _ => None,
    };
    let mut out = [0u8; 32];
    for (i, o) in out.iter_mut().enumerate() {
        *o = (digit(b[2 * i])? << 4) | digit(b[2 * i + 1])?;
    }
    Some(out)
}

/// The 64 lower-case hex digits of a digest: `hex32` under the name that goes with md5.rs's `md5_hex`.
pub fn sha256_hex(d: &[u8; 32]) -> String {
    hex32(d)
}

/// SHA-256 of `data`, as hex.
pub fn sha256_of(data: &[u8]) -> String {
    sha256_hex(&sha256(data))
}

#[cfg(test)]
mod tests {
    use super::*;

    // Every expected SHA-256 in these tests is `shasum -a 256`'s (6.02, 2026-11-13) of the same bytes: none
    // is this module's own output. The generated messages are the first n bytes of the pattern below:
    //   perl -e 'print pack("C*", map { ($_ * 7 + 3) % 256 } 0 .. $ARGV[0] - 1)' n | shasum -a 256
    fn pattern(n: usize) -> Vec<u8> {
        (0..n).map(|i| (i * 7 + 3) as u8).collect()
    }

    fn in_pieces(data: &[u8], step: usize) -> [u8; 32] {
        let mut s = Sha256::new();
        for c in data.chunks(step) {
            s.update(c);
        }
        s.finish()
    }

    // FIPS 180-4's examples: the empty message, one block, the 448-bit message whose padding takes a second
    // block, and one million "a". Each through every public name.
    #[test]
    fn fips_180_4_vectors() {
        let million = "a".repeat(1_000_000);
        let v: [(&str, &str); 4] = [
            ("", "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"),
            ("abc", "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"),
            (
                "abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq",
                "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1",
            ),
            (&million, "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0"),
        ];
        for (s, h) in v {
            let what = if s.len() > 100 { "one million a" } else { s };
            assert_eq!(sha256_of(s.as_bytes()), h, "{what:?}");
            let d = sha256(s.as_bytes());
            assert_eq!(hex32(&d), h, "{what:?}");
            assert_eq!(sha256_hex(&d), h, "{what:?}");
            assert_eq!(unhex32(h), Some(d), "{what:?}");
            let mut m = Sha256::default();
            m.update(s.as_bytes());
            assert_eq!(m.finish(), d, "{what:?}");
            assert_eq!(in_pieces(s.as_bytes(), 997), d, "{what:?}");
        }
    }

    // Input given in pieces gives the same digest as in one, whatever the pieces' length: every length from
    // 1 to 130 (the buffer's seams, with a block and with two inside one piece), and 200 as md5.rs has.
    #[test]
    fn pieces_make_the_same_digest() {
        let data = pattern(1000);
        let one = sha256(&data);
        assert_eq!(hex32(&one), "1e9bc38cbf860b9ec31918b065f9b52476c549a782e0e7990bed8ce3868d2371");
        for step in (1usize..=130).chain([200]) {
            assert_eq!(in_pieces(&data, step), one, "step {step}");
        }
    }

    // A message cut in two at every place, the empty piece at either end included.
    #[test]
    fn any_split_gives_the_same_digest() {
        let data = pattern(300);
        let one = sha256(&data);
        assert_eq!(hex32(&one), "04773f8726c81cafcfa1a09a82664b98b00d2021031a1715bca1154f2dad3472");
        for cut in 0..=data.len() {
            let mut s = Sha256::new();
            s.update(&data[..cut]);
            s.update(&data[cut..]);
            assert_eq!(s.finish(), one, "cut at {cut}");
        }
    }

    // The padding's edges: 55 bytes is the most that leaves room for the padding in the message's last
    // block, 56 to 63 push the length into a block of its own, 64 is a whole block and nothing else, and
    // 119 and 120 are the same edge a block later.
    #[test]
    fn the_paddings_edges() {
        let v: [(usize, &str); 7] = [
            (55, "e7313d333c272e639f790978283f9eb392e843d0f29b7016828bb1daa4aac70b"),
            (56, "4324d65f3c103567f5589c710bc08f8523f929a9272e3af36fc968e52abc6c27"),
            (63, "81c80242132f230c3bd41b3e63bbcff16107339549214a99614ff26664625055"),
            (64, "39e3d7b6b5d075d37d053ad89b24b41bef4f3c29760c84447cab3f3be1882241"),
            (65, "aacca6ff74fdbb296d165a45cecfa04e5127bc008770fbbdd48006f2d2fae95e"),
            (119, "9ce7368e4daf32341631b492e80359dc9f594b48453cd0dd5bf0b19279cc177e"),
            (120, "7836b787757e95e58b3ca5aec90b1b004e8deba1e50e9675af9cabf1a13a04b5"),
        ];
        for (n, h) in v {
            let data = pattern(n);
            assert_eq!(sha256_of(&data), h, "{n} bytes");
            assert_eq!(hex32(&in_pieces(&data, 1)), h, "{n} bytes, one at a time");
            assert_eq!(hex32(&in_pieces(&data, 7)), h, "{n} bytes, seven at a time");
        }
    }

    // A digest's hex reads back as the digest, and nothing else reads as one.
    #[test]
    fn hex_round_trip_is_strict() {
        let d = sha256(b"abc");
        let h = hex32(&d);
        assert_eq!(h.len(), 64);
        assert!(h.bytes().all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c)), "{h}");
        assert_eq!(sha256_hex(&d), h);
        assert_eq!(unhex32(&h), Some(d));
        let every: [u8; 32] = std::array::from_fn(|i| (i * 8 + i / 4) as u8);
        assert_eq!(hex32(&every), "0008101821293139424a525a636b737b848c949ca5adb5bdc6ced6dee7eff7ff");
        assert_eq!(unhex32(&hex32(&every)), Some(every));
        assert_eq!(unhex32(&"0".repeat(64)), Some([0; 32]));
        assert_eq!(unhex32(&"f".repeat(64)), Some([0xff; 32]));

        assert_eq!(unhex32(""), None);
        assert_eq!(unhex32(&h[..63]), None, "63 digits");
        assert_eq!(unhex32(&format!("{h}0")), None, "65 digits");
        assert_eq!(unhex32(&h[..62]), None, "31 bytes");
        assert_eq!(unhex32(&format!("{h}00")), None, "33 bytes");
        assert_eq!(unhex32(&h.to_uppercase()), None, "upper case");
        for at in [0, 1, 31, 62, 63] {
            for bad in ["A", "F", "g", "G", " ", "+", "-", "x", "/", ":", "`", "\n", "\0"] {
                let mut s = h.clone();
                s.replace_range(at..at + 1, bad);
                assert_eq!(s.len(), 64);
                assert_eq!(unhex32(&s), None, "{bad:?} at {at}");
            }
        }
        // 64 bytes that are not 64 characters
        let wide = format!("{}\u{e9}", &h[..62]);
        assert_eq!(wide.len(), 64);
        assert_eq!(unhex32(&wide), None);
        assert_eq!(unhex32(&format!("0x{}", &h[..62])), None);
        assert_eq!(unhex32(&format!(" {}", &h[..63])), None);
    }

    #[test]
    fn no_em_dash() {
        assert!(!include_str!("sha256.rs").contains('\u{2014}'));
    }
}
