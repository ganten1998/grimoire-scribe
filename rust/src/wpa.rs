//! WPA/WPA2 Personal's pre-shared key: PBKDF2-HMAC-SHA1 over the passphrase and the network name
//! (IEEE 802.11i, 4096 rounds, 256 bits), computed here so Settings > Wi-Fi can add a network on a
//! firmware without wpa_passphrase. No secret is logged or kept beyond the caller's buffer.

const HEX: &[u8; 16] = b"0123456789abcdef";

/// The 32-byte PSK for `passphrase` and `ssid`.
pub fn psk(passphrase: &[u8], ssid: &[u8]) -> [u8; 32] {
    let mut out = [0u8; 32];
    let mut done = 0;
    let mut block = 1u32;
    while done < out.len() {
        let mut salt = Vec::with_capacity(ssid.len() + 4);
        salt.extend_from_slice(ssid);
        salt.extend_from_slice(&block.to_be_bytes());
        let mut u = hmac_sha1(passphrase, &salt);
        let mut t = u;
        for _ in 1..4096 {
            u = hmac_sha1(passphrase, &u);
            for (a, b) in t.iter_mut().zip(u.iter()) {
                *a ^= *b;
            }
        }
        let n = (out.len() - done).min(t.len());
        out[done..done + n].copy_from_slice(&t[..n]);
        done += n;
        block += 1;
        salt.fill(0);
        u.fill(0);
        t.fill(0);
    }
    out
}

/// The PSK as wpa_passphrase writes it: 64 lower-case hex digits.
pub fn psk_hex(passphrase: &[u8], ssid: &[u8]) -> [u8; 64] {
    let mut raw = psk(passphrase, ssid);
    let mut hex = [0u8; 64];
    for (i, byte) in raw.iter().enumerate() {
        hex[i * 2] = HEX[(byte >> 4) as usize];
        hex[i * 2 + 1] = HEX[(byte & 0x0f) as usize];
    }
    raw.fill(0);
    hex
}

fn sha1(data: &[u8]) -> [u8; 20] {
    let mut h: [u32; 5] = [0x67452301, 0xefcdab89, 0x98badcfe, 0x10325476, 0xc3d2e1f0];
    let mut msg = Vec::with_capacity(data.len() + 72);
    msg.extend_from_slice(data);
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&((data.len() as u64) * 8).to_be_bytes());
    for chunk in msg.chunks_exact(64) {
        let mut w = [0u32; 80];
        for (i, word) in chunk.chunks_exact(4).enumerate() {
            w[i] = u32::from_be_bytes([word[0], word[1], word[2], word[3]]);
        }
        for i in 16..80 {
            w[i] = (w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16]).rotate_left(1);
        }
        let (mut a, mut b, mut c, mut d, mut e) = (h[0], h[1], h[2], h[3], h[4]);
        for (i, &wi) in w.iter().enumerate() {
            let (f, k) = match i {
                0..=19 => ((b & c) | ((!b) & d), 0x5a827999),
                20..=39 => (b ^ c ^ d, 0x6ed9eba1),
                40..=59 => ((b & c) | (b & d) | (c & d), 0x8f1bbcdc),
                _ => (b ^ c ^ d, 0xca62c1d6),
            };
            let t = a
                .rotate_left(5)
                .wrapping_add(f)
                .wrapping_add(e)
                .wrapping_add(k)
                .wrapping_add(wi);
            e = d;
            d = c;
            c = b.rotate_left(30);
            b = a;
            a = t;
        }
        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
    }
    let mut out = [0u8; 20];
    for (i, word) in h.iter().enumerate() {
        out[i * 4..i * 4 + 4].copy_from_slice(&word.to_be_bytes());
    }
    out
}

fn hmac_sha1(key: &[u8], data: &[u8]) -> [u8; 20] {
    let mut k = [0u8; 64];
    if key.len() > 64 {
        k[..20].copy_from_slice(&sha1(key));
    } else {
        k[..key.len()].copy_from_slice(key);
    }
    let mut inner = Vec::with_capacity(64 + data.len());
    let mut outer = Vec::with_capacity(64 + 20);
    for &byte in k.iter() {
        inner.push(byte ^ 0x36);
        outer.push(byte ^ 0x5c);
    }
    inner.extend_from_slice(data);
    let digest = sha1(&inner);
    outer.extend_from_slice(&digest);
    let out = sha1(&outer);
    k.fill(0);
    inner.fill(0);
    outer.fill(0);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{b:02x}")).collect()
    }

    #[test]
    fn sha1_matches_the_standard_vectors() {
        assert_eq!(hex(&sha1(b"")), "da39a3ee5e6b4b0d3255bfef95601890afd80709");
        assert_eq!(hex(&sha1(b"abc")), "a9993e364706816aba3e25717850c26c9cd0d89d");
        assert_eq!(
            hex(&sha1(b"The quick brown fox jumps over the lazy dog")),
            "2fd4e1c67a2d28fced849ee1bb76e7391b93eb12"
        );
    }

    #[test]
    fn hmac_sha1_matches_rfc_2202() {
        assert_eq!(
            hex(&hmac_sha1(&[0x0b; 20], b"Hi There")),
            "b617318655057264e28bc0b6fb378c8ef146be00"
        );
        assert_eq!(
            hex(&hmac_sha1(b"Jefe", b"what do ya want for nothing?")),
            "effcdf6ae5eb2fa2d27416d5f184df9c259a7c79"
        );
    }

    #[test]
    fn wpa_psks_match_the_published_vectors() {
        assert_eq!(
            hex(&psk(b"password", b"IEEE")),
            "f42c6fc52df0ebef9ebb4b90b38a5f902e83fe1b135a70e23aed762e9710a12e"
        );
        assert_eq!(
            hex(&psk(b"ThisIsAPassword", b"ThisIsASSID")),
            "0dc0d6eb90555ed6419756b9a15ec3e3209b63df707dd508d14581f8982721af"
        );
        assert_eq!(
            std::str::from_utf8(&psk_hex(b"password", b"IEEE")).unwrap(),
            "f42c6fc52df0ebef9ebb4b90b38a5f902e83fe1b135a70e23aed762e9710a12e"
        );
    }

    #[test]
    fn the_longest_name_and_passphrase_are_handled() {
        assert_eq!(
            hex(&psk(&[b'z'; 63], &[b'x'; 32])),
            "b8549bd407232e409d48061bfcd8199b6bc2512e62e9f3434e147ab50e2602dd"
        );
    }
}
