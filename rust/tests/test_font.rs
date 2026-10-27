// test_font.c: text drawing. The CRC table below was recorded at 56d931c, before the Gothic
// bold learned to spare accents: every printable ASCII character, in every face, both themes
// and a range of sizes, must draw byte for byte as it did then, and so must every face that is
// never overprinted (accents included, all but the inverted exclamation mark, which Jersey 10
// now sets lower) and the symbols the pixel faces borrow from Terminess.
// GRIMOIRE_FONT_CRC=print prints a fresh table instead of checking (after a deliberate change).
mod testlib;

use grimoire::prelude::*;
use testlib::*;

// Terminess is anti-aliased: its edge pixels come out of float math that can round differently
// where the compiler fuses multiply-adds differently (gcc, or an x86 host). The table was
// recorded with clang on arm64; elsewhere only the pixel faces (thresholded, exact) are compared.
// (stb_truetype is the C compiled by build.rs, with the host's cc: clang on an Apple Silicon Mac.)
const OUTLINE_EXACT: bool = cfg!(all(target_arch = "aarch64", target_vendor = "apple"));

fn crc_table() -> [u32; 256] {
    let mut t = [0u32; 256];
    for i in 0..256u32 {
        let mut c = i;
        for _ in 0..8 {
            c = if c & 1 != 0 { 0xEDB88320u32 ^ (c >> 1) } else { c >> 1 };
        }
        t[i as usize] = c;
    }
    t
}

fn crc_add(table: &[u32; 256], crc: u32, data: &[u8]) -> u32 {
    let mut crc = !crc;
    for &b in data {
        crc = table[((crc ^ b as u32) & 0xFF) as usize] ^ (crc >> 8);
    }
    !crc
}

/// A byte string of UTF-8 (the C's "\xc3\xb1" escapes), as a &str.
fn u(b: &[u8]) -> &str {
    std::str::from_utf8(b).unwrap()
}

// The strings: printable ASCII in order; words with the usual kerning pairs; the same words cut
// to fit a narrow box (the ellipsis path); accented letters and the punctuation Spanish, French,
// German, Catalan and the Nordic languages need; symbols the pixel faces lack (drawn from
// Terminess). The inverted exclamation mark is not in the accents: Jersey 10 sets it lower on
// purpose (see inverted_exclamation_is_not_an_i).
const STR_ASCII: usize = 0;
const STR_WORDS: usize = 1;
const STR_FIT: usize = 2;
const STR_ACCENTS: usize = 3;
const STR_FALLBACK: usize = 4;
const STR_COUNT: usize = 5;
const K_WORDS: &str = "The quick brown fox jumps over the lazy dog. AV To Wa Yo LT 0123456789 (x) [y] {z}";
const K_ACCENTS: &[u8] = b"\xc3\xb1\xc3\x91\xc3\xbc\xc3\x9c\xc3\xb6\xc3\x96\xc3\xa4\xc3\x84\xc3\xab\xc3\xa9\xc3\xa8\xc3\xaa\xc3\xa7\
\xc3\xa1\xc3\xad\xc3\xb3\xc3\xba\xc3\xa0\xc3\xac\xc3\xb2\xc3\xb9\xc3\xa2\xc3\xae\xc3\xb4\xc3\xbb\xc3\xa3\
\xc3\xb5\xc2\xbf\xc2\xab\xc2\xbb\xc3\x9f\xc5\x93\xc5\x92\xc3\xa6\xc3\x86\xc3\xb8\xc3\x98\xc3\xa5\
\xc3\x85\xc3\xbf\xc5\xb8\xc2\xba\xc2\xaa \xc3\x87 M\xc3\xbasica A\xc3\xb1adir col\xc2\xb7lecci\xc3\xb3";
const K_FALLBACK: &[u8] = b"\xe2\x86\x90 \xe2\x86\x92 \xc2\xb1 \xc2\xb2 \xc2\xb3 \xc2\xb5 \xc2\xbd";

fn str_of(s: usize, ascii: &str) -> String {
    match s {
        STR_ASCII => ascii.to_string(),
        STR_WORDS | STR_FIT => K_WORDS.to_string(),
        STR_ACCENTS => u(K_ACCENTS).to_string(),
        _ => u(K_FALLBACK).to_string(),
    }
}

const K_SIZES: [f32; 13] = [12.0, 20.0, 26.0, 27.0, 34.0, 40.0, 44.0, 48.4, 56.0, 61.0, 64.0, 80.0, 100.0];
const NSIZES: usize = K_SIZES.len();

const FACES: [FontFace; FontFace::COUNT] =
    [FontFace::Regular, FontFace::Bold, FontFace::Title, FontFace::Caps, FontFace::Fixed, FontFace::FixedBold];

/// Draws one string black on white in a canvas sized to it and returns the CRC of the pixels,
/// the canvas size and the advance.
fn render_crc(table: &[u32; 256], ascii: &str, f: FontFace, px: f32, s: usize) -> u32 {
    let str_ = str_of(s, ascii);
    let asc = font_ascent(f, px);
    let desc = font_descent(f, px);
    let adv = font_measure(f, px, &str_);
    let pad = px as i32;
    let w = adv + 2 * pad;
    let h = asc + desc + 2 * pad;
    let mut c = Canvas::default();
    if canvas_init(&mut c, w, h) != 0 {
        return 0;
    }
    let mut drawn = adv;
    if s == STR_FIT {
        font_draw_centered(&mut c, f, px, Rect { x: pad, y: pad, w: adv * 2 / 5, h: asc + desc }, &str_, 0);
    } else {
        drawn = font_draw(&mut c, f, px, pad, pad + asc, &str_, 0);
    }
    let head: [i32; 5] = [w, h, adv, drawn, asc * 1000 + desc];
    let head_bytes: Vec<u8> = head.iter().flat_map(|v| v.to_ne_bytes()).collect();
    let mut crc = crc_add(table, 0, &head_bytes);
    for y in 0..h {
        let at = (y * c.stride) as usize;
        crc = crc_add(table, crc, &c.px[at..at + w as usize]);
    }
    canvas_free(&mut c);
    crc
}

/// Gothic FONT_BOLD and FONT_CAPS overprint; their accented letters are the ones allowed to change.
fn pinned(theme: UiTheme, face: FontFace, s: usize) -> bool {
    !(s == STR_ACCENTS && theme == UiTheme::Gothic && (face == FontFace::Bold || face == FontFace::Caps))
}

/// Whether the string draws through Terminess (anti-aliased) in that theme and face.
fn outline(theme: UiTheme, face: FontFace, s: usize) -> bool {
    theme == UiTheme::Classic || face == FontFace::Fixed || face == FontFace::FixedBold || s == STR_FALLBACK
}

const NCRC: usize = 2 * FontFace::COUNT * NSIZES * STR_COUNT;
/// Recorded at 56d931c (clang, arm64). Order: theme (Gothic, Classic), face, size, string;
/// 0 = not pinned.
static K_CRC: [u32; NCRC] = [
    0x8D79336A, 0xF50559C7, 0x89EA6B5F, 0xB6BBB110, 0xBC2013CF, 0xF97FB7A5,
    0x2EEFC06C, 0x73234FF1, 0x10F4896E, 0x7614D942, 0x289EBBD4, 0xB65571BB,
    0x8A532B0E, 0x3E9E687C, 0xA7274629, 0xD9C55742, 0x25FE266C, 0xD34A2243,
    0xEC8BA815, 0x0EA25A85, 0xA7D52727, 0xB37CABA6, 0x0F512DD3, 0x93FE4BFC,
    0x8BE49938, 0x649A98EC, 0x60B09D7A, 0x7113F261, 0xDE987AE2, 0x68B8E04D,
    0xF82B0070, 0x83279D58, 0x6F930FDD, 0x055B92AF, 0x18BF62E6, 0xE78022F7,
    0x98FA2857, 0x63C18BA8, 0x268B04AC, 0x2F6887F9, 0x3D97305A, 0xE1646CE8,
    0x57BD8F3B, 0xE2E540B4, 0x75C67173, 0x54BD71D4, 0x5AB1ACC1, 0x50262B93,
    0x3DC54B6B, 0xB8E0663E, 0x3D4B312E, 0x49FC5B56, 0x2886006C, 0x0C8313D2,
    0x07F1CD3E, 0x595A77F2, 0x85599589, 0x168CDDBF, 0x6C70BE28, 0x0B6D93BE,
    0xD56F8137, 0xCD74D6CE, 0x53FF26FB, 0x68610B83, 0x391C09E3, 0xAB14842D,
    0x5924B57F, 0x39076CB3, 0x00000000, 0x706B449F, 0x191E4BF6, 0xB56FFB9E,
    0xCFFBE7F4, 0x00000000, 0xD5085BAE, 0x4663E6DF, 0x577D641C, 0xBB688B86,
    0x00000000, 0xF67B0DB5, 0xB6D85B01, 0xCA2991A0, 0xACF3AA2C, 0x00000000,
    0xF75F9211, 0xC0D86DC5, 0x9C1202CA, 0xA2E19845, 0x00000000, 0x5C1027BA,
    0x8697E8AF, 0x643D95AE, 0xAE4C1614, 0x00000000, 0xB8DBAB99, 0xF93AD865,
    0x99BD41FE, 0x13D0C654, 0x00000000, 0x12B66E15, 0xC7C415E2, 0x99F69114,
    0xDE39762C, 0x00000000, 0x3CAC323C, 0xBE2656D2, 0xED01F71B, 0x387AA963,
    0x00000000, 0xFCBE16D8, 0x99683CAB, 0x3BB272D3, 0xF3BB62B4, 0x00000000,
    0xBA62E7A6, 0x5A9160E0, 0xEDD4D9A6, 0xF86FA247, 0x00000000, 0x49D631D1,
    0x3C80E6E8, 0x46AF5761, 0x4DFCD982, 0x00000000, 0xF0BBAE3A, 0xA5F46A9F,
    0x1289C267, 0x434A25F2, 0x00000000, 0x365B0A7A, 0x45466955, 0xA8B9E2A0,
    0x8CD43526, 0x22D52FBC, 0xF93E6504, 0x9036F86F, 0xA4F956D0, 0x86EED8BA,
    0x19D8D0AB, 0xEEAFD3B1, 0x599C30F4, 0xF12A1ECD, 0x39BD9080, 0x67DEF86E,
    0x0A4CD7F8, 0x7CD01329, 0x051EB8E8, 0x0B8D3C06, 0x6AC4694E, 0x10303F12,
    0x1B70396E, 0x78431E8C, 0x4301158B, 0xCE7A1DCD, 0xA2D38B0A, 0x82102305,
    0x8FC222E9, 0x3D4FC254, 0xA4CD90EB, 0x4B64BA76, 0x706305EB, 0x7151AEB3,
    0xC4FFB9CF, 0x524115CA, 0x66F6DD97, 0x4CAABD27, 0xF9FC40F9, 0x69646099,
    0x9F14C708, 0xDD1D6CC0, 0x6E0E8AED, 0xA5A861EF, 0x5F191305, 0x3B228739,
    0x2FB5C505, 0xF2005FE6, 0x7FABDD53, 0xDC35DB73, 0x2A7B6867, 0x4E9A823E,
    0x5C8671ED, 0xC7F7FE62, 0x1AB60105, 0x16FE2FBF, 0xCE17997F, 0x43700163,
    0x839EE47F, 0xEF05DD19, 0x29641F13, 0xD572ED9D, 0x4E169F6E, 0xAA0EF2EA,
    0xEF6FF330, 0x2A557C25, 0xE45C5CD3, 0xAB14842D, 0x5924B57F, 0x39076CB3,
    0x00000000, 0x706B449F, 0x191E4BF6, 0xB56FFB9E, 0xCFFBE7F4, 0x00000000,
    0xD5085BAE, 0x4663E6DF, 0x577D641C, 0xBB688B86, 0x00000000, 0xF67B0DB5,
    0xB6D85B01, 0xCA2991A0, 0xACF3AA2C, 0x00000000, 0xF75F9211, 0xC0D86DC5,
    0x9C1202CA, 0xA2E19845, 0x00000000, 0x5C1027BA, 0x8697E8AF, 0x643D95AE,
    0xAE4C1614, 0x00000000, 0xB8DBAB99, 0xF93AD865, 0x99BD41FE, 0x13D0C654,
    0x00000000, 0x12B66E15, 0xC7C415E2, 0x99F69114, 0xDE39762C, 0x00000000,
    0x3CAC323C, 0xBE2656D2, 0xED01F71B, 0x387AA963, 0x00000000, 0xFCBE16D8,
    0x99683CAB, 0x3BB272D3, 0xF3BB62B4, 0x00000000, 0xBA62E7A6, 0x5A9160E0,
    0xEDD4D9A6, 0xF86FA247, 0x00000000, 0x49D631D1, 0x3C80E6E8, 0x46AF5761,
    0x4DFCD982, 0x00000000, 0xF0BBAE3A, 0xA5F46A9F, 0x1289C267, 0x434A25F2,
    0x00000000, 0x365B0A7A, 0xFBE5AB20, 0xA6F102A7, 0xB993C598, 0xAE77C13B,
    0x64C2AF3D, 0xABB59CB6, 0x4E8B46A3, 0xD2C9A466, 0x4215FEA9, 0xBE1A3BD4,
    0x07902E15, 0x414D2F8D, 0xCD2CB35B, 0x216CD482, 0xF77C1875, 0x8F1F3EAE,
    0xA68026C5, 0xE5DA31E2, 0xC0D1E024, 0xC12FE0EE, 0x67BF35B7, 0x9EB2C046,
    0xF8D2527F, 0x2F6D33D2, 0x18B3BE04, 0x150E9A42, 0xB8FE1345, 0x4188D3CC,
    0xC92E4FC0, 0xF41ED0AD, 0xCA332E07, 0x5ED4EB55, 0xAEA965C1, 0xEF597E31,
    0x27C35354, 0x1B4DC403, 0xB0C458B4, 0xB3F5341D, 0x48C35007, 0xDDE64D0A,
    0x3637B46E, 0x798D4FEA, 0xA6F049F1, 0x44136FA7, 0xDA2F9574, 0x11B2F37F,
    0x1FB94067, 0xE7189716, 0xCE570A4A, 0xC88403C0, 0x8322AAA6, 0x002B58B4,
    0xB66F2798, 0xA763B708, 0x1239FF31, 0x1FB470C7, 0x7F8BC7AE, 0x3146AF34,
    0xC0A00C93, 0x2C44D991, 0x54FB7217, 0xE326AE52, 0x6286D547, 0xD7FF0BEA,
    0xE1A64EBE, 0x1E5897E0, 0xF2208B19, 0xC1C43694, 0xE9EC08D2, 0xD7638795,
    0x99F872F2, 0xEAF0FFE0, 0x27D47C4D, 0xE57E1170, 0xE035021E, 0xE8B414B4,
    0x8CC98DAC, 0x78C731FA, 0x86CD32C4, 0xCEE7AE21, 0xB4FAAE66, 0x83FC626A,
    0xEBB55E85, 0x7406074B, 0x62EF00EE, 0x8C83A6D5, 0xBC1CF1EE, 0xF79435EC,
    0x0CAAFC4F, 0xEEDFBF3D, 0xEA04179B, 0x62004280, 0xB3BA3010, 0xB3A7B03E,
    0x3A92A4A5, 0xE4C4AADC, 0xACF7A858, 0xD6B27D54, 0x8A513B09, 0xF8F70665,
    0x11CE3121, 0x33B98874, 0x3A25619B, 0xEFA983CA, 0xFD7EF127, 0xBAB7259B,
    0x377E9519, 0x7347CC50, 0xB58AD568, 0xDAFA9585, 0x520532AE, 0x1A9EF4D7,
    0x7F011A92, 0x46A00122, 0x6831C877, 0xADB116BC, 0x520F16F7, 0x895DA953,
    0x1C9BFC67, 0x9B16DA62, 0x6E1FCA2C, 0x26B62016, 0x34BF342A, 0xCFE5C5C7,
    0x070D840E, 0xCC7849D6, 0xF7B2A3AA, 0xCD5E13A2, 0xA74CFBBE, 0x42FA0C32,
    0xFBE5AB20, 0xA6F102A7, 0xB993C598, 0xAE77C13B, 0x64C2AF3D, 0xABB59CB6,
    0x4E8B46A3, 0xD2C9A466, 0x4215FEA9, 0xBE1A3BD4, 0x07902E15, 0x414D2F8D,
    0xCD2CB35B, 0x216CD482, 0xF77C1875, 0x8F1F3EAE, 0xA68026C5, 0xE5DA31E2,
    0xC0D1E024, 0xC12FE0EE, 0x67BF35B7, 0x9EB2C046, 0xF8D2527F, 0x2F6D33D2,
    0x18B3BE04, 0x150E9A42, 0xB8FE1345, 0x4188D3CC, 0xC92E4FC0, 0xF41ED0AD,
    0xCA332E07, 0x5ED4EB55, 0xAEA965C1, 0xEF597E31, 0x27C35354, 0x1B4DC403,
    0xB0C458B4, 0xB3F5341D, 0x48C35007, 0xDDE64D0A, 0x3637B46E, 0x798D4FEA,
    0xA6F049F1, 0x44136FA7, 0xDA2F9574, 0x11B2F37F, 0x1FB94067, 0xE7189716,
    0xCE570A4A, 0xC88403C0, 0x8322AAA6, 0x002B58B4, 0xB66F2798, 0xA763B708,
    0x1239FF31, 0x1FB470C7, 0x7F8BC7AE, 0x3146AF34, 0xC0A00C93, 0x2C44D991,
    0x54FB7217, 0xE326AE52, 0x6286D547, 0xD7FF0BEA, 0xE1A64EBE, 0x1E5897E0,
    0xF2208B19, 0xC1C43694, 0xE9EC08D2, 0xD7638795, 0x99F872F2, 0xEAF0FFE0,
    0x27D47C4D, 0xE57E1170, 0xE035021E, 0xE8B414B4, 0x8CC98DAC, 0x78C731FA,
    0x86CD32C4, 0xCEE7AE21, 0xB4FAAE66, 0x83FC626A, 0xEBB55E85, 0x7406074B,
    0x62EF00EE, 0x8C83A6D5, 0xBC1CF1EE, 0xF79435EC, 0x0CAAFC4F, 0xEEDFBF3D,
    0xEA04179B, 0x62004280, 0xB3BA3010, 0xB3A7B03E, 0x3A92A4A5, 0xE4C4AADC,
    0xACF7A858, 0xD6B27D54, 0x8A513B09, 0xF8F70665, 0x11CE3121, 0x33B98874,
    0x3A25619B, 0xEFA983CA, 0xFD7EF127, 0xBAB7259B, 0x377E9519, 0x7347CC50,
    0xB58AD568, 0xDAFA9585, 0x520532AE, 0x1A9EF4D7, 0x7F011A92, 0x46A00122,
    0x6831C877, 0xADB116BC, 0x520F16F7, 0x895DA953, 0x1C9BFC67, 0x9B16DA62,
    0x6E1FCA2C, 0x26B62016, 0x34BF342A, 0xCFE5C5C7, 0x070D840E, 0xCC7849D6,
    0xF7B2A3AA, 0xCD5E13A2, 0xA74CFBBE, 0x42FA0C32, 0x1E5897E0, 0xF2208B19,
    0xC1C43694, 0xE9EC08D2, 0xD7638795, 0x99F872F2, 0xEAF0FFE0, 0x27D47C4D,
    0xE57E1170, 0xE035021E, 0xE8B414B4, 0x8CC98DAC, 0x78C731FA, 0x86CD32C4,
    0xCEE7AE21, 0xB4FAAE66, 0x83FC626A, 0xEBB55E85, 0x7406074B, 0x62EF00EE,
    0x8C83A6D5, 0xBC1CF1EE, 0xF79435EC, 0x0CAAFC4F, 0xEEDFBF3D, 0xEA04179B,
    0x62004280, 0xB3BA3010, 0xB3A7B03E, 0x3A92A4A5, 0xE4C4AADC, 0xACF7A858,
    0xD6B27D54, 0x8A513B09, 0xF8F70665, 0x11CE3121, 0x33B98874, 0x3A25619B,
    0xEFA983CA, 0xFD7EF127, 0xBAB7259B, 0x377E9519, 0x7347CC50, 0xB58AD568,
    0xDAFA9585, 0x520532AE, 0x1A9EF4D7, 0x7F011A92, 0x46A00122, 0x6831C877,
    0xADB116BC, 0x520F16F7, 0x895DA953, 0x1C9BFC67, 0x9B16DA62, 0x6E1FCA2C,
    0x26B62016, 0x34BF342A, 0xCFE5C5C7, 0x070D840E, 0xCC7849D6, 0xF7B2A3AA,
    0xCD5E13A2, 0xA74CFBBE, 0x42FA0C32, 0x1E5897E0, 0xF2208B19, 0xC1C43694,
    0xE9EC08D2, 0xD7638795, 0x99F872F2, 0xEAF0FFE0, 0x27D47C4D, 0xE57E1170,
    0xE035021E, 0xE8B414B4, 0x8CC98DAC, 0x78C731FA, 0x86CD32C4, 0xCEE7AE21,
    0xB4FAAE66, 0x83FC626A, 0xEBB55E85, 0x7406074B, 0x62EF00EE, 0x8C83A6D5,
    0xBC1CF1EE, 0xF79435EC, 0x0CAAFC4F, 0xEEDFBF3D, 0xEA04179B, 0x62004280,
    0xB3BA3010, 0xB3A7B03E, 0x3A92A4A5, 0xE4C4AADC, 0xACF7A858, 0xD6B27D54,
    0x8A513B09, 0xF8F70665, 0x11CE3121, 0x33B98874, 0x3A25619B, 0xEFA983CA,
    0xFD7EF127, 0xBAB7259B, 0x377E9519, 0x7347CC50, 0xB58AD568, 0xDAFA9585,
    0x520532AE, 0x1A9EF4D7, 0x7F011A92, 0x46A00122, 0x6831C877, 0xADB116BC,
    0x520F16F7, 0x895DA953, 0x1C9BFC67, 0x9B16DA62, 0x6E1FCA2C, 0x26B62016,
    0x34BF342A, 0xCFE5C5C7, 0x070D840E, 0xCC7849D6, 0xF7B2A3AA, 0xCD5E13A2,
    0xA74CFBBE, 0x42FA0C32, 0xFBE5AB20, 0xA6F102A7, 0xB993C598, 0xAE77C13B,
    0x64C2AF3D, 0xABB59CB6, 0x4E8B46A3, 0xD2C9A466, 0x4215FEA9, 0xBE1A3BD4,
    0x07902E15, 0x414D2F8D, 0xCD2CB35B, 0x216CD482, 0xF77C1875, 0x8F1F3EAE,
    0xA68026C5, 0xE5DA31E2, 0xC0D1E024, 0xC12FE0EE, 0x67BF35B7, 0x9EB2C046,
    0xF8D2527F, 0x2F6D33D2, 0x18B3BE04, 0x150E9A42, 0xB8FE1345, 0x4188D3CC,
    0xC92E4FC0, 0xF41ED0AD, 0xCA332E07, 0x5ED4EB55, 0xAEA965C1, 0xEF597E31,
    0x27C35354, 0x1B4DC403, 0xB0C458B4, 0xB3F5341D, 0x48C35007, 0xDDE64D0A,
    0x3637B46E, 0x798D4FEA, 0xA6F049F1, 0x44136FA7, 0xDA2F9574, 0x11B2F37F,
    0x1FB94067, 0xE7189716, 0xCE570A4A, 0xC88403C0, 0x8322AAA6, 0x002B58B4,
    0xB66F2798, 0xA763B708, 0x1239FF31, 0x1FB470C7, 0x7F8BC7AE, 0x3146AF34,
    0xC0A00C93, 0x2C44D991, 0x54FB7217, 0xE326AE52, 0x6286D547, 0xD7FF0BEA,
    0xE1A64EBE, 0x1E5897E0, 0xF2208B19, 0xC1C43694, 0xE9EC08D2, 0xD7638795,
    0x99F872F2, 0xEAF0FFE0, 0x27D47C4D, 0xE57E1170, 0xE035021E, 0xE8B414B4,
    0x8CC98DAC, 0x78C731FA, 0x86CD32C4, 0xCEE7AE21, 0xB4FAAE66, 0x83FC626A,
    0xEBB55E85, 0x7406074B, 0x62EF00EE, 0x8C83A6D5, 0xBC1CF1EE, 0xF79435EC,
    0x0CAAFC4F, 0xEEDFBF3D, 0xEA04179B, 0x62004280, 0xB3BA3010, 0xB3A7B03E,
    0x3A92A4A5, 0xE4C4AADC, 0xACF7A858, 0xD6B27D54, 0x8A513B09, 0xF8F70665,
    0x11CE3121, 0x33B98874, 0x3A25619B, 0xEFA983CA, 0xFD7EF127, 0xBAB7259B,
    0x377E9519, 0x7347CC50, 0xB58AD568, 0xDAFA9585, 0x520532AE, 0x1A9EF4D7,
    0x7F011A92, 0x46A00122, 0x6831C877, 0xADB116BC, 0x520F16F7, 0x895DA953,
    0x1C9BFC67, 0x9B16DA62, 0x6E1FCA2C, 0x26B62016, 0x34BF342A, 0xCFE5C5C7,
    0x070D840E, 0xCC7849D6, 0xF7B2A3AA, 0xCD5E13A2, 0xA74CFBBE, 0x42FA0C32,
];

fn ascii_unchanged() {
    let ascii: String = (0..95u8).map(|i| (32 + i) as char).collect();
    let table = crc_table();
    let print = std::env::var("GRIMOIRE_FONT_CRC").map_or(false, |v| v == "print");
    let (mut i, mut compared, mut bad) = (0usize, 0usize, 0usize);
    if print {
        println!("static K_CRC: [u32; NCRC] = [");
    }
    for theme in [UiTheme::Gothic, UiTheme::Classic] {
        ui_set_theme(theme);
        for face in FACES {
            for z in 0..NSIZES {
                for s in 0..STR_COUNT {
                    let crc = if pinned(theme, face, s) { render_crc(&table, &ascii, face, K_SIZES[z], s) } else { 0 };
                    if print {
                        print!("{}0x{:08X},{}", if i % 6 != 0 { " " } else { "    " }, crc, if i % 6 == 5 || i == NCRC - 1 { "\n" } else { "" });
                        i += 1;
                        continue;
                    }
                    i += 1;
                    if crc == 0 || (outline(theme, face, s) && !OUTLINE_EXACT) {
                        continue;
                    }
                    compared += 1;
                    if crc != K_CRC[i - 1] {
                        bad += 1;
                        eprintln!(
                            "  changed: {} face {} at {:.1} px, string {}",
                            if theme == UiTheme::Gothic { "gothic" } else { "classic" },
                            face as i32,
                            K_SIZES[z],
                            s
                        );
                    }
                }
            }
        }
    }
    if print {
        println!("];");
    }
    ui_set_theme(UiTheme::Gothic);
    check!(i == NCRC);
    check!(print || compared > NCRC / 2 || !OUTLINE_EXACT);
    check!(bad == 0);
}

// ---- the Gothic bold and accents ----
// The bold overprints the glyph one design pixel to the right. Accent marks it would fill in
// (the dots of u-umlaut, the tilde, the circumflex, the ring) are drawn as designed instead; in
// sharp s and o slash a one-pixel gap moves right instead of closing; everything else is exactly
// the overprint.

fn ink(c: &Canvas, x: i32, y: i32) -> bool {
    c.px[(y * c.stride + x) as usize] < 128
}

/// A canvas for one glyph at px, white.
fn blank(c: &mut Canvas, px: f32) -> bool {
    canvas_init(c, (6.0 * px) as i32, (4.0 * px) as i32) == 0
}

/// One string, black, its pen at x = px + dx and its baseline at y = 2 px.
fn draw1(c: &mut Canvas, f: FontFace, px: f32, dx: i32, s: &str) {
    font_draw(c, f, px, px as i32 + dx, (2.0 * px) as i32, s, 0);
}

fn top_row(c: &Canvas) -> i32 {
    for y in 0..c.h {
        for x in 0..c.w {
            if ink(c, x, y) {
                return y;
            }
        }
    }
    -1
}

/// Separate (8-connected) dark shapes in the rows above y_end.
fn components_above(c: &Canvas, y_end: i32) -> i32 {
    let (mut n, w) = (0, c.w);
    let mut stack = vec![0i32; w as usize * y_end as usize];
    let mut seen = vec![0u8; w as usize * y_end as usize];
    for i in 0..w * y_end {
        if seen[i as usize] != 0 || !ink(c, i % w, i / w) {
            continue;
        }
        n += 1;
        let mut sp = 0;
        seen[i as usize] = 1;
        stack[sp] = i;
        sp += 1;
        while sp != 0 {
            sp -= 1;
            let p = stack[sp];
            let (px, py) = (p % w, p / w);
            for dy in -1..=1 {
                for dx in -1..=1 {
                    let (qx, qy) = (px + dx, py + dy);
                    if qx < 0 || qy < 0 || qx >= w || qy >= y_end || seen[(qy * w + qx) as usize] != 0 || !ink(c, qx, qy) {
                        continue;
                    }
                    seen[(qy * w + qx) as usize] = 1;
                    stack[sp] = qy * w + qx;
                    sp += 1;
                }
            }
        }
    }
    n
}

/// The most separate dark runs in any one row above y_end.
fn max_runs_above(c: &Canvas, y_end: i32) -> i32 {
    let mut best = 0;
    for y in 0..y_end {
        let mut runs = 0;
        for x in 0..c.w {
            runs += (ink(c, x, y) && (x == 0 || !ink(c, x - 1, y))) as i32;
        }
        if runs > best {
            best = runs;
        }
    }
    best
}

fn same_rows(a: &Canvas, b: &Canvas, y0: i32, y1: i32) -> bool {
    for y in y0..y1 {
        for x in 0..a.w {
            if ink(a, x, y) != ink(b, x, y) {
                return false;
            }
        }
    }
    true
}

/// The UI's bold sizes: small (34), toasts and menus (44), single-glyph keyboard keys (64),
/// button labels (FONT_CAPS, 30 and 44), and the smallest step (k = 1).
const K_BOLD: [(FontFace, f32); 6] = [
    (FontFace::Bold, 20.0),
    (FontFace::Bold, 34.0),
    (FontFace::Bold, 44.0),
    (FontFace::Bold, 64.0),
    (FontFace::Caps, 30.0),
    (FontFace::Caps, 44.0),
];
const NBOLD: usize = K_BOLD.len();

fn bold_umlauts_keep_two_dots() {
    ui_set_theme(UiTheme::Gothic);
    // Each base letter and the same letter with a diaeresis (the body is the same glyph).
    let pairs: [(&str, &[u8]); 7] = [
        ("u", b"\xc3\xbc"),
        ("U", b"\xc3\x9c"),
        ("o", b"\xc3\xb6"),
        ("O", b"\xc3\x96"),
        ("a", b"\xc3\xa4"),
        ("A", b"\xc3\x84"),
        ("e", b"\xc3\xab"),
    ];
    for z in 0..NBOLD {
        for p in &pairs {
            let (mut base, mut acc) = (Canvas::default(), Canvas::default());
            if !blank(&mut base, K_BOLD[z].1) || !blank(&mut acc, K_BOLD[z].1) {
                check!(false); // canvas
                return;
            }
            draw1(&mut base, K_BOLD[z].0, K_BOLD[z].1, 0, p.0);
            draw1(&mut acc, K_BOLD[z].0, K_BOLD[z].1, 0, u(p.1));
            let top = top_row(&base);
            let n = if top > 0 { components_above(&acc, top) } else { -1 };
            if n != 2 {
                eprintln!("  {} at {:.0} px: {} dark shapes above the letter", u(p.1), K_BOLD[z].1, n);
            }
            check!(n == 2);
            canvas_free(&mut base);
            canvas_free(&mut acc);
        }
    }
}

fn bold_tildes_and_hats_keep_their_gaps() {
    ui_set_theme(UiTheme::Gothic);
    // n/N/a/o tilde, the circumflexes, a/A ring, carons and the double acute: the mark is
    // drawn exactly as in the regular weight and still has a gap in it.
    let pairs: [(&str, &[u8]); 13] = [
        ("n", b"\xc3\xb1"),
        ("N", b"\xc3\x91"),
        ("a", b"\xc3\xa3"),
        ("o", b"\xc3\xb5"),
        ("a", b"\xc3\xa2"),
        ("e", b"\xc3\xaa"),
        ("o", b"\xc3\xb4"),
        ("u", b"\xc3\xbb"),
        ("a", b"\xc3\xa5"),
        ("A", b"\xc3\x85"),
        ("c", b"\xc4\x8d"),
        ("S", b"\xc5\xa0"),
        ("o", b"\xc5\x91"),
    ];
    for z in 0..NBOLD {
        for p in &pairs {
            let (mut base, mut bold, mut reg) = (Canvas::default(), Canvas::default(), Canvas::default());
            if !blank(&mut base, K_BOLD[z].1) || !blank(&mut bold, K_BOLD[z].1) || !blank(&mut reg, K_BOLD[z].1) {
                check!(false); // canvas
                return;
            }
            draw1(&mut base, K_BOLD[z].0, K_BOLD[z].1, 0, p.0);
            draw1(&mut bold, K_BOLD[z].0, K_BOLD[z].1, 0, u(p.1));
            draw1(&mut reg, FontFace::Regular, K_BOLD[z].1, 0, u(p.1));
            let top = top_row(&base);
            let same = top > 0 && same_rows(&bold, &reg, 0, top);
            let runs = if top > 0 { max_runs_above(&bold, top) } else { 0 };
            if !same || runs < 2 {
                eprintln!(
                    "  {} at {:.0} px: mark {}, {} runs",
                    u(p.1),
                    K_BOLD[z].1,
                    if same { "as designed" } else { "changed" },
                    runs
                );
            }
            check!(same && runs >= 2);
            canvas_free(&mut base);
            canvas_free(&mut bold);
            canvas_free(&mut reg);
        }
    }
}

// Every glyph of the face from U+0021 to U+017F: the bold covers the regular weight (but in
// sharp s and o slash, whose gaps move right) and never more than the plain overprint; ASCII
// and letters without such marks are exactly the plain overprint, letters with them are not.
fn bold_differs_only_at_marks() {
    ui_set_theme(UiTheme::Gothic);
    const HARMLESS: [u32; 19] = [0xE7, 0xC7, 0xE6, 0x153, 0xAB, 0xBB, 0xE9, 0xE1, 0xED, 0xF3, 0xFA, 0xE0, 0xF9, 0xBF, 0xA1, 0xFF, 0xEF, 0x142, 0xB7];
    // (i circumflex is not here: its hat's gap is two pixels wide, so the overprint keeps it.)
    const SPARED: [u32; 19] = [0xFC, 0xF6, 0xE4, 0xEB, 0xDC, 0xD6, 0xC4, 0xF1, 0xD1, 0xE3, 0xF5, 0xE2, 0xEA, 0xF4, 0xFB, 0xE5, 0xC5, 0x10D, 0x151];
    const MOVED: [u32; 3] = [0xDF, 0xF8, 0xD8];
    const SIZES: [f32; 3] = [20.0, 44.0, 64.0];
    let mut checked = 0;
    for z in 0..3 {
        let px = SIZES[z];
        for cp in 0x21u32..=0x17F {
            if (cp > 0x7E && cp < 0xA1) || !font_has_glyph(FontFace::Bold, cp) {
                continue;
            }
            // (cp < 0x80 is one byte, else the two bytes of a code point below U+0800)
            let s = char::from_u32(cp).unwrap().to_string();
            let k = font_measure(FontFace::Bold, px, &s) - font_measure(FontFace::Regular, px, &s);
            let (mut a, mut r, mut b) = (Canvas::default(), Canvas::default(), Canvas::default());
            if !blank(&mut a, px) || !blank(&mut r, px) || !blank(&mut b, px) {
                check!(false); // canvas
                return;
            }
            draw1(&mut a, FontFace::Bold, px, 0, &s);
            draw1(&mut r, FontFace::Regular, px, 0, &s);
            for d in 0..=k {
                draw1(&mut b, FontFace::Regular, px, d, &s);
            }
            let (mut covers, mut within, mut equal) = (true, true, true);
            for y in 0..a.h {
                for x in 0..a.w {
                    covers &= !ink(&r, x, y) || ink(&a, x, y);
                    within &= !ink(&a, x, y) || ink(&b, x, y);
                    equal &= ink(&a, x, y) == ink(&b, x, y);
                }
            }
            let mut want_equal = cp < 0x80;
            let mut want_spared = false;
            let mut want_moved = false;
            for h in HARMLESS {
                want_equal |= h == cp;
            }
            for sp in SPARED {
                want_spared |= sp == cp;
            }
            for m in MOVED {
                want_moved |= m == cp;
            }
            let ok = k > 0 && (covers || want_moved) && within && (!want_equal || equal) && (!(want_spared || want_moved) || !equal);
            if !ok {
                eprintln!("  U+{:04X} at {:.0} px: k {} covers {} within {} equal {}", cp, px, k, covers as i32, within as i32, equal as i32);
            }
            check!(ok);
            checked += 1;
            canvas_free(&mut a);
            canvas_free(&mut r);
            canvas_free(&mut b);
        }
    }
    check!(checked > 3 * 250);
}

/// Gaps in row y: runs of paper with ink on both sides.
fn gaps_in_row(c: &Canvas, y: i32) -> i32 {
    let (mut first, mut last, mut gaps) = (-1, -1, 0);
    for x in 0..c.w {
        if ink(c, x, y) {
            if first < 0 {
                first = x;
            }
            last = x;
        }
    }
    let mut x = first + 1;
    while first >= 0 && x <= last {
        gaps += (!ink(c, x, y) && ink(c, x - 1, y)) as i32;
        x += 1;
    }
    gaps
}

/// The first run of ink in row y, in pixels (0 if the row has none).
fn first_run(c: &Canvas, y: i32) -> i32 {
    let (mut x, mut n) = (0, 0);
    while x < c.w && !ink(c, x, y) {
        x += 1;
    }
    while x < c.w && {
        let on = ink(c, x, y);
        x += 1;
        on
    } {
        n += 1;
    }
    n
}

// Sharp s and o slash in bold: every gap of the regular weight survives in each row (the
// plain overprint closes the notch that sets sharp s apart from B, and fills the counter of
// o slash on both sides of the slash), while the stroke on the left of each row still takes
// the overprint (one design pixel wider: a stem of even weight), and sharp s stays several
// design pixels away from a bold B and a bold 8.
fn bold_letters_keep_their_gaps() {
    ui_set_theme(UiTheme::Gothic);
    let letters: [&[u8]; 3] = [b"\xc3\x9f", b"\xc3\xb8", b"\xc3\x98"];
    for z in 0..NBOLD {
        let (f, px) = K_BOLD[z];
        let k = font_measure(FontFace::Bold, px, "B") - font_measure(FontFace::Regular, px, "B");
        for i in 0..3 {
            let (mut r, mut b) = (Canvas::default(), Canvas::default());
            if !blank(&mut r, px) || !blank(&mut b, px) {
                check!(false); // canvas
                return;
            }
            draw1(&mut r, FontFace::Regular, px, 0, u(letters[i]));
            draw1(&mut b, f, px, 0, u(letters[i]));
            let (mut lost, mut rows, mut thin) = (0, 0, 0);
            for y in 0..r.h {
                let g = gaps_in_row(&r, y);
                rows += (g > 0) as i32;
                lost += (gaps_in_row(&b, y) < g) as i32;
                thin += (first_run(&r, y) != 0 && first_run(&b, y) != first_run(&r, y) + k) as i32;
            }
            if lost != 0 || thin != 0 || rows == 0 {
                eprintln!("  {} at {:.0} px: {} of {} rows lost a gap, {} rows not emboldened", u(letters[i]), px, lost, rows, thin);
            }
            check!(k > 0 && rows > 0 && lost == 0 && thin == 0);
            let mut j = 0;
            while i == 0 && j < 2 {
                let mut o = Canvas::default();
                if !blank(&mut o, px) {
                    check!(false); // canvas
                    return;
                }
                draw1(&mut o, f, px, 0, if j != 0 { "8" } else { "B" });
                let mut differ = 0;
                for y in 0..b.h {
                    for x in 0..b.w {
                        differ += (ink(&b, x, y) != ink(&o, x, y)) as i32;
                    }
                }
                if differ < 4 * k * k {
                    eprintln!("  sharp s at {:.0} px: {} pixels from a {}", px, differ, if j != 0 { "8" } else { "B" });
                }
                check!(k > 0 && differ >= 4 * k * k);
                canvas_free(&mut o);
                j += 1;
            }
            canvas_free(&mut r);
            canvas_free(&mut b);
        }
    }
}

fn bottom_row(c: &Canvas) -> i32 {
    for y in (0..c.h).rev() {
        for x in 0..c.w {
            if ink(c, x, y) {
                return y;
            }
        }
    }
    -1
}

// Jersey 10 draws the inverted exclamation mark exactly as its i. Its own pixels are kept, set
// two design pixels lower: the stem hangs below the baseline (within the face's descent), so a
// Spanish exclamation no longer opens with an i. Jacquard 24 and Terminess have their own shape
// for it, left where it was. In every face, at the UI's sizes, it is not an i.
fn inverted_exclamation_is_not_an_i() {
    const K: [(FontFace, f32); 12] = [
        (FontFace::Regular, 20.0),
        (FontFace::Regular, 34.0),
        (FontFace::Regular, 48.4),
        (FontFace::Bold, 34.0),
        (FontFace::Bold, 44.0),
        (FontFace::Bold, 64.0),
        (FontFace::Caps, 30.0),
        (FontFace::Caps, 44.0),
        (FontFace::Title, 64.0),
        (FontFace::Title, 115.2),
        (FontFace::Fixed, 44.0),
        (FontFace::FixedBold, 44.0),
    ];
    for theme in [UiTheme::Gothic, UiTheme::Classic] {
        ui_set_theme(theme);
        for z in 0..K.len() {
            let (f, px) = K[z];
            let (mut i, mut inv) = (Canvas::default(), Canvas::default());
            if !blank(&mut i, px) || !blank(&mut inv, px) {
                check!(false); // canvas
                return;
            }
            draw1(&mut i, f, px, 0, "i");
            draw1(&mut inv, f, px, 0, u(b"\xc2\xa1"));
            let mut same = true;
            for y in 0..i.h {
                for x in 0..i.w {
                    same &= ink(&i, x, y) == ink(&inv, x, y);
                }
            }
            let base = (2.0 * px) as i32;
            let low = bottom_row(&inv);
            let jersey = theme == UiTheme::Gothic && (f == FontFace::Regular || f == FontFace::Bold || f == FontFace::Caps);
            let mut ok = !same && low >= 0;
            if jersey {
                // An i's pixels, moved down by two design pixels (d screen pixels).
                let d = 2 * (font_measure(FontFace::Bold, px, "i") - font_measure(FontFace::Regular, px, "i"));
                let mut lowered = d > 0;
                for y in 0..i.h {
                    for x in 0..i.w {
                        lowered &= ink(&inv, x, y) == (y >= d && ink(&i, x, y - d));
                    }
                }
                ok &= lowered && low >= base && low < base + font_descent(f, px);
            } else {
                ok &= low < base; // as the font has it: standing on the baseline
            }
            if !ok {
                eprintln!(
                    "  {} face {} at {:.1} px: {}, lowest ink {} (baseline {})",
                    if theme == UiTheme::Gothic { "gothic" } else { "classic" },
                    f as i32,
                    px,
                    if same { "drawn as an i" } else { "not an i" },
                    low,
                    base
                );
            }
            check!(ok);
            canvas_free(&mut i);
            canvas_free(&mut inv);
        }
    }
    ui_set_theme(UiTheme::Gothic);
}

#[test]
fn test_font() {
    ui_init(300.0);
    assert_eq!(font_init(), 0, "font_init");
    run!(ascii_unchanged);
    run!(bold_umlauts_keep_two_dots);
    run!(bold_tildes_and_hats_keep_their_gaps);
    run!(bold_differs_only_at_marks);
    run!(bold_letters_keep_their_gaps);
    run!(inverted_exclamation_is_not_an_i);
    done();
}
