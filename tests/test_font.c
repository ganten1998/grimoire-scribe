// test_font.c: text drawing. The CRC table below was recorded at 56d931c, before the Gothic
// bold learned to spare accents: every printable ASCII character, in every face, both themes
// and a range of sizes, must draw byte for byte as it did then, and so must every face that is
// never overprinted (accents included) and the symbols the pixel faces borrow from Terminess.
// GRIMOIRE_FONT_CRC=print prints a fresh table instead of checking (after a deliberate change).
#include <stdint.h>

#include "font.h"
#include "ui.h"
#include "testlib.h"

// Terminess is anti-aliased: its edge pixels come out of float math that can round differently
// where the compiler fuses multiply-adds differently (gcc, or an x86 host). The table was
// recorded with clang on arm64; elsewhere only the pixel faces (thresholded, exact) are compared.
#if defined(__clang__) && (defined(__aarch64__) || defined(__arm64__))
#define OUTLINE_EXACT 1
#else
#define OUTLINE_EXACT 0
#endif

static uint32_t crc_table[256];

static void crc_init(void) {
    for (uint32_t i = 0; i < 256; i++) {
        uint32_t c = i;
        for (int k = 0; k < 8; k++) c = c & 1 ? 0xEDB88320u ^ (c >> 1) : c >> 1;
        crc_table[i] = c;
    }
}

static uint32_t crc_add(uint32_t crc, const void *data, size_t n) {
    const uint8_t *p = data;
    crc = ~crc;
    while (n--) crc = crc_table[(crc ^ *p++) & 0xFF] ^ (crc >> 8);
    return ~crc;
}

// The strings: printable ASCII in order; words with the usual kerning pairs; the same words cut
// to fit a narrow box (the ellipsis path); accented letters and the punctuation Spanish, French,
// German and the Nordic languages need; symbols the pixel faces lack (drawn from Terminess).
enum { STR_ASCII, STR_WORDS, STR_FIT, STR_ACCENTS, STR_FALLBACK, STR_COUNT };
static char g_ascii[96];
static const char *k_words = "The quick brown fox jumps over the lazy dog. AV To Wa Yo LT 0123456789 (x) [y] {z}";
static const char *k_accents =
    "\xc3\xb1\xc3\x91\xc3\xbc\xc3\x9c\xc3\xb6\xc3\x96\xc3\xa4\xc3\x84\xc3\xab\xc3\xa9\xc3\xa8\xc3\xaa\xc3\xa7"
    "\xc3\xa1\xc3\xad\xc3\xb3\xc3\xba\xc3\xa0\xc3\xac\xc3\xb2\xc3\xb9\xc3\xa2\xc3\xae\xc3\xb4\xc3\xbb\xc3\xa3"
    "\xc3\xb5\xc2\xbf\xc2\xa1\xc2\xab\xc2\xbb\xc3\x9f\xc5\x93\xc5\x92\xc3\xa6\xc3\x86\xc3\xb8\xc3\x98\xc3\xa5"
    "\xc3\x85\xc3\xbf\xc5\xb8\xc2\xba\xc2\xaa \xc3\x87 M\xc3\xbasica A\xc3\xb1" "adir";
static const char *k_fallback = "\xe2\x86\x90 \xe2\x86\x92 \xc2\xb1 \xc2\xb2 \xc2\xb3 \xc2\xb5 \xc2\xbd";

static const char *str_of(int s) {
    switch (s) {
    case STR_ASCII: return g_ascii;
    case STR_WORDS:
    case STR_FIT: return k_words;
    case STR_ACCENTS: return k_accents;
    default: return k_fallback;
    }
}

static const float k_sizes[] = {12.f, 20.f, 26.f, 27.f, 34.f, 40.f, 44.f, 48.4f, 56.f, 61.f, 64.f, 80.f, 100.f};
#define NSIZES (int)(sizeof k_sizes / sizeof k_sizes[0])

// Draws one string black on white in a canvas sized to it and returns the CRC of the pixels,
// the canvas size and the advance.
static uint32_t render_crc(FontFace f, float px, int s) {
    const char *str = str_of(s);
    int asc = font_ascent(f, px), desc = font_descent(f, px);
    int adv = font_measure(f, px, str);
    int pad = (int)px;
    int w = adv + 2 * pad, h = asc + desc + 2 * pad;
    Canvas c;
    if (canvas_init(&c, w, h) != 0) return 0;
    int drawn = adv;
    if (s == STR_FIT)
        font_draw_centered(&c, f, px, (Rect){pad, pad, adv * 2 / 5, asc + desc}, str, 0);
    else
        drawn = font_draw(&c, f, px, pad, pad + asc, str, 0);
    int head[5] = {w, h, adv, drawn, asc * 1000 + desc};
    uint32_t crc = crc_add(0, head, sizeof head);
    for (int y = 0; y < h; y++) crc = crc_add(crc, c.px + y * c.stride, (size_t)w);
    canvas_free(&c);
    return crc;
}

// Gothic FONT_BOLD and FONT_CAPS overprint; their accented letters are the ones allowed to change.
static bool pinned(int theme, int face, int s) {
    return !(s == STR_ACCENTS && theme == THEME_GOTHIC && (face == FONT_BOLD || face == FONT_CAPS));
}

// Whether the string draws through Terminess (anti-aliased) in that theme and face.
static bool outline(int theme, int face, int s) {
    return theme == THEME_CLASSIC || face == FONT_FIXED || face == FONT_FIXED_BOLD || s == STR_FALLBACK;
}

#define NCRC (2 * FONT_FACE_COUNT * NSIZES * STR_COUNT)
// Recorded at 56d931c (clang, arm64). Order: theme (Gothic, Classic), face, size, string;
// 0 = not pinned.
static const uint32_t k_crc[NCRC] = {
    0x8D79336Au, 0xF50559C7u, 0x89EA6B5Fu, 0xB78D9852u, 0xBC2013CFu, 0xF97FB7A5u,
    0x2EEFC06Cu, 0x73234FF1u, 0x150A24ECu, 0x7614D942u, 0x289EBBD4u, 0xB65571BBu,
    0x8A532B0Eu, 0x48C0EC93u, 0xA7274629u, 0xD9C55742u, 0x25FE266Cu, 0xD34A2243u,
    0xA42F578Eu, 0x0EA25A85u, 0xA7D52727u, 0xB37CABA6u, 0x0F512DD3u, 0xFDD970E7u,
    0x8BE49938u, 0x649A98ECu, 0x60B09D7Au, 0x7113F261u, 0x2A95F37Fu, 0x68B8E04Du,
    0xF82B0070u, 0x83279D58u, 0x6F930FDDu, 0xAFF53771u, 0x18BF62E6u, 0xE78022F7u,
    0x98FA2857u, 0x63C18BA8u, 0xC451CE65u, 0x2F6887F9u, 0x3D97305Au, 0xE1646CE8u,
    0x57BD8F3Bu, 0xD42AF289u, 0x75C67173u, 0x54BD71D4u, 0x5AB1ACC1u, 0x50262B93u,
    0x565CB48Cu, 0xB8E0663Eu, 0x3D4B312Eu, 0x49FC5B56u, 0x2886006Cu, 0xF7B1815Cu,
    0x07F1CD3Eu, 0x595A77F2u, 0x85599589u, 0x168CDDBFu, 0xD1AFD234u, 0x0B6D93BEu,
    0xD56F8137u, 0xCD74D6CEu, 0x53FF26FBu, 0x58EDC8B4u, 0x391C09E3u, 0xAB14842Du,
    0x5924B57Fu, 0x39076CB3u, 0x00000000u, 0x706B449Fu, 0x191E4BF6u, 0xB56FFB9Eu,
    0xCFFBE7F4u, 0x00000000u, 0xD5085BAEu, 0x4663E6DFu, 0x577D641Cu, 0xBB688B86u,
    0x00000000u, 0xF67B0DB5u, 0xB6D85B01u, 0xCA2991A0u, 0xACF3AA2Cu, 0x00000000u,
    0xF75F9211u, 0xC0D86DC5u, 0x9C1202CAu, 0xA2E19845u, 0x00000000u, 0x5C1027BAu,
    0x8697E8AFu, 0x643D95AEu, 0xAE4C1614u, 0x00000000u, 0xB8DBAB99u, 0xF93AD865u,
    0x99BD41FEu, 0x13D0C654u, 0x00000000u, 0x12B66E15u, 0xC7C415E2u, 0x99F69114u,
    0xDE39762Cu, 0x00000000u, 0x3CAC323Cu, 0xBE2656D2u, 0xED01F71Bu, 0x387AA963u,
    0x00000000u, 0xFCBE16D8u, 0x99683CABu, 0x3BB272D3u, 0xF3BB62B4u, 0x00000000u,
    0xBA62E7A6u, 0x5A9160E0u, 0xEDD4D9A6u, 0xF86FA247u, 0x00000000u, 0x49D631D1u,
    0x3C80E6E8u, 0x46AF5761u, 0x4DFCD982u, 0x00000000u, 0xF0BBAE3Au, 0xA5F46A9Fu,
    0x1289C267u, 0x434A25F2u, 0x00000000u, 0x365B0A7Au, 0x45466955u, 0xA8B9E2A0u,
    0x8CD43526u, 0x563CB805u, 0xF93E6504u, 0x9036F86Fu, 0xA4F956D0u, 0x86EED8BAu,
    0x5A79E900u, 0xEEAFD3B1u, 0x599C30F4u, 0xF12A1ECDu, 0x39BD9080u, 0x735F062Du,
    0x0A4CD7F8u, 0x7CD01329u, 0x051EB8E8u, 0x0B8D3C06u, 0x34B41B3Eu, 0x10303F12u,
    0x1B70396Eu, 0x78431E8Cu, 0x4301158Bu, 0xBB89A211u, 0xA2D38B0Au, 0x82102305u,
    0x8FC222E9u, 0x3D4FC254u, 0x8B81E808u, 0x4B64BA76u, 0x706305EBu, 0x7151AEB3u,
    0xC4FFB9CFu, 0x13383B6Fu, 0x66F6DD97u, 0x4CAABD27u, 0xF9FC40F9u, 0x69646099u,
    0x7E73DFAFu, 0xDD1D6CC0u, 0x6E0E8AEDu, 0xA5A861EFu, 0x5F191305u, 0x522647CBu,
    0x2FB5C505u, 0xF2005FE6u, 0x7FABDD53u, 0xDC35DB73u, 0x651407A3u, 0x4E9A823Eu,
    0x5C8671EDu, 0xC7F7FE62u, 0x1AB60105u, 0x9BEEB7F2u, 0xCE17997Fu, 0x43700163u,
    0x839EE47Fu, 0xEF05DD19u, 0xDD7F2845u, 0xD572ED9Du, 0x4E169F6Eu, 0xAA0EF2EAu,
    0xEF6FF330u, 0xC4DE3C1Eu, 0xE45C5CD3u, 0xAB14842Du, 0x5924B57Fu, 0x39076CB3u,
    0x00000000u, 0x706B449Fu, 0x191E4BF6u, 0xB56FFB9Eu, 0xCFFBE7F4u, 0x00000000u,
    0xD5085BAEu, 0x4663E6DFu, 0x577D641Cu, 0xBB688B86u, 0x00000000u, 0xF67B0DB5u,
    0xB6D85B01u, 0xCA2991A0u, 0xACF3AA2Cu, 0x00000000u, 0xF75F9211u, 0xC0D86DC5u,
    0x9C1202CAu, 0xA2E19845u, 0x00000000u, 0x5C1027BAu, 0x8697E8AFu, 0x643D95AEu,
    0xAE4C1614u, 0x00000000u, 0xB8DBAB99u, 0xF93AD865u, 0x99BD41FEu, 0x13D0C654u,
    0x00000000u, 0x12B66E15u, 0xC7C415E2u, 0x99F69114u, 0xDE39762Cu, 0x00000000u,
    0x3CAC323Cu, 0xBE2656D2u, 0xED01F71Bu, 0x387AA963u, 0x00000000u, 0xFCBE16D8u,
    0x99683CABu, 0x3BB272D3u, 0xF3BB62B4u, 0x00000000u, 0xBA62E7A6u, 0x5A9160E0u,
    0xEDD4D9A6u, 0xF86FA247u, 0x00000000u, 0x49D631D1u, 0x3C80E6E8u, 0x46AF5761u,
    0x4DFCD982u, 0x00000000u, 0xF0BBAE3Au, 0xA5F46A9Fu, 0x1289C267u, 0x434A25F2u,
    0x00000000u, 0x365B0A7Au, 0xFBE5AB20u, 0xA6F102A7u, 0xB993C598u, 0x3C926F42u,
    0x64C2AF3Du, 0xABB59CB6u, 0x4E8B46A3u, 0xD2C9A466u, 0xB8CD351Du, 0xBE1A3BD4u,
    0x07902E15u, 0x414D2F8Du, 0xCD2CB35Bu, 0x982AD78Du, 0xF77C1875u, 0x8F1F3EAEu,
    0xA68026C5u, 0xE5DA31E2u, 0xA361E7CCu, 0xC12FE0EEu, 0x67BF35B7u, 0x9EB2C046u,
    0xF8D2527Fu, 0x99BB4494u, 0x18B3BE04u, 0x150E9A42u, 0xB8FE1345u, 0x4188D3CCu,
    0x6C62AEB6u, 0xF41ED0ADu, 0xCA332E07u, 0x5ED4EB55u, 0xAEA965C1u, 0xAC0A9CE9u,
    0x27C35354u, 0x1B4DC403u, 0xB0C458B4u, 0xB3F5341Du, 0x099B7650u, 0xDDE64D0Au,
    0x3637B46Eu, 0x798D4FEAu, 0xA6F049F1u, 0x114C4FFDu, 0xDA2F9574u, 0x11B2F37Fu,
    0x1FB94067u, 0xE7189716u, 0x64355424u, 0xC88403C0u, 0x8322AAA6u, 0x002B58B4u,
    0xB66F2798u, 0x3827E813u, 0x1239FF31u, 0x1FB470C7u, 0x7F8BC7AEu, 0x3146AF34u,
    0xF0F47922u, 0x2C44D991u, 0x54FB7217u, 0xE326AE52u, 0x6286D547u, 0x0C357547u,
    0xE1A64EBEu, 0x1E5897E0u, 0xF2208B19u, 0xC1C43694u, 0xA5F23ED9u, 0xD7638795u,
    0x99F872F2u, 0xEAF0FFE0u, 0x27D47C4Du, 0x853B79FEu, 0xE035021Eu, 0xE8B414B4u,
    0x8CC98DACu, 0x78C731FAu, 0x538C631Eu, 0xCEE7AE21u, 0xB4FAAE66u, 0x83FC626Au,
    0xEBB55E85u, 0xBF3BE233u, 0x62EF00EEu, 0x8C83A6D5u, 0xBC1CF1EEu, 0xF79435ECu,
    0xA6D4EAAAu, 0xEEDFBF3Du, 0xEA04179Bu, 0x62004280u, 0xB3BA3010u, 0x51631E13u,
    0x3A92A4A5u, 0xE4C4AADCu, 0xACF7A858u, 0xD6B27D54u, 0x306C08D2u, 0xF8F70665u,
    0x11CE3121u, 0x33B98874u, 0x3A25619Bu, 0x18AADCCCu, 0xFD7EF127u, 0xBAB7259Bu,
    0x377E9519u, 0x7347CC50u, 0x12A0F79Cu, 0xDAFA9585u, 0x520532AEu, 0x1A9EF4D7u,
    0x7F011A92u, 0x5DE8B7DEu, 0x6831C877u, 0xADB116BCu, 0x520F16F7u, 0x895DA953u,
    0xAD4E6655u, 0x9B16DA62u, 0x6E1FCA2Cu, 0x26B62016u, 0x34BF342Au, 0x06FE41C7u,
    0x070D840Eu, 0xCC7849D6u, 0xF7B2A3AAu, 0xCD5E13A2u, 0x361F4DA1u, 0x42FA0C32u,
    0xFBE5AB20u, 0xA6F102A7u, 0xB993C598u, 0x3C926F42u, 0x64C2AF3Du, 0xABB59CB6u,
    0x4E8B46A3u, 0xD2C9A466u, 0xB8CD351Du, 0xBE1A3BD4u, 0x07902E15u, 0x414D2F8Du,
    0xCD2CB35Bu, 0x982AD78Du, 0xF77C1875u, 0x8F1F3EAEu, 0xA68026C5u, 0xE5DA31E2u,
    0xA361E7CCu, 0xC12FE0EEu, 0x67BF35B7u, 0x9EB2C046u, 0xF8D2527Fu, 0x99BB4494u,
    0x18B3BE04u, 0x150E9A42u, 0xB8FE1345u, 0x4188D3CCu, 0x6C62AEB6u, 0xF41ED0ADu,
    0xCA332E07u, 0x5ED4EB55u, 0xAEA965C1u, 0xAC0A9CE9u, 0x27C35354u, 0x1B4DC403u,
    0xB0C458B4u, 0xB3F5341Du, 0x099B7650u, 0xDDE64D0Au, 0x3637B46Eu, 0x798D4FEAu,
    0xA6F049F1u, 0x114C4FFDu, 0xDA2F9574u, 0x11B2F37Fu, 0x1FB94067u, 0xE7189716u,
    0x64355424u, 0xC88403C0u, 0x8322AAA6u, 0x002B58B4u, 0xB66F2798u, 0x3827E813u,
    0x1239FF31u, 0x1FB470C7u, 0x7F8BC7AEu, 0x3146AF34u, 0xF0F47922u, 0x2C44D991u,
    0x54FB7217u, 0xE326AE52u, 0x6286D547u, 0x0C357547u, 0xE1A64EBEu, 0x1E5897E0u,
    0xF2208B19u, 0xC1C43694u, 0xA5F23ED9u, 0xD7638795u, 0x99F872F2u, 0xEAF0FFE0u,
    0x27D47C4Du, 0x853B79FEu, 0xE035021Eu, 0xE8B414B4u, 0x8CC98DACu, 0x78C731FAu,
    0x538C631Eu, 0xCEE7AE21u, 0xB4FAAE66u, 0x83FC626Au, 0xEBB55E85u, 0xBF3BE233u,
    0x62EF00EEu, 0x8C83A6D5u, 0xBC1CF1EEu, 0xF79435ECu, 0xA6D4EAAAu, 0xEEDFBF3Du,
    0xEA04179Bu, 0x62004280u, 0xB3BA3010u, 0x51631E13u, 0x3A92A4A5u, 0xE4C4AADCu,
    0xACF7A858u, 0xD6B27D54u, 0x306C08D2u, 0xF8F70665u, 0x11CE3121u, 0x33B98874u,
    0x3A25619Bu, 0x18AADCCCu, 0xFD7EF127u, 0xBAB7259Bu, 0x377E9519u, 0x7347CC50u,
    0x12A0F79Cu, 0xDAFA9585u, 0x520532AEu, 0x1A9EF4D7u, 0x7F011A92u, 0x5DE8B7DEu,
    0x6831C877u, 0xADB116BCu, 0x520F16F7u, 0x895DA953u, 0xAD4E6655u, 0x9B16DA62u,
    0x6E1FCA2Cu, 0x26B62016u, 0x34BF342Au, 0x06FE41C7u, 0x070D840Eu, 0xCC7849D6u,
    0xF7B2A3AAu, 0xCD5E13A2u, 0x361F4DA1u, 0x42FA0C32u, 0x1E5897E0u, 0xF2208B19u,
    0xC1C43694u, 0xA5F23ED9u, 0xD7638795u, 0x99F872F2u, 0xEAF0FFE0u, 0x27D47C4Du,
    0x853B79FEu, 0xE035021Eu, 0xE8B414B4u, 0x8CC98DACu, 0x78C731FAu, 0x538C631Eu,
    0xCEE7AE21u, 0xB4FAAE66u, 0x83FC626Au, 0xEBB55E85u, 0xBF3BE233u, 0x62EF00EEu,
    0x8C83A6D5u, 0xBC1CF1EEu, 0xF79435ECu, 0xA6D4EAAAu, 0xEEDFBF3Du, 0xEA04179Bu,
    0x62004280u, 0xB3BA3010u, 0x51631E13u, 0x3A92A4A5u, 0xE4C4AADCu, 0xACF7A858u,
    0xD6B27D54u, 0x306C08D2u, 0xF8F70665u, 0x11CE3121u, 0x33B98874u, 0x3A25619Bu,
    0x18AADCCCu, 0xFD7EF127u, 0xBAB7259Bu, 0x377E9519u, 0x7347CC50u, 0x12A0F79Cu,
    0xDAFA9585u, 0x520532AEu, 0x1A9EF4D7u, 0x7F011A92u, 0x5DE8B7DEu, 0x6831C877u,
    0xADB116BCu, 0x520F16F7u, 0x895DA953u, 0xAD4E6655u, 0x9B16DA62u, 0x6E1FCA2Cu,
    0x26B62016u, 0x34BF342Au, 0x06FE41C7u, 0x070D840Eu, 0xCC7849D6u, 0xF7B2A3AAu,
    0xCD5E13A2u, 0x361F4DA1u, 0x42FA0C32u, 0x1E5897E0u, 0xF2208B19u, 0xC1C43694u,
    0xA5F23ED9u, 0xD7638795u, 0x99F872F2u, 0xEAF0FFE0u, 0x27D47C4Du, 0x853B79FEu,
    0xE035021Eu, 0xE8B414B4u, 0x8CC98DACu, 0x78C731FAu, 0x538C631Eu, 0xCEE7AE21u,
    0xB4FAAE66u, 0x83FC626Au, 0xEBB55E85u, 0xBF3BE233u, 0x62EF00EEu, 0x8C83A6D5u,
    0xBC1CF1EEu, 0xF79435ECu, 0xA6D4EAAAu, 0xEEDFBF3Du, 0xEA04179Bu, 0x62004280u,
    0xB3BA3010u, 0x51631E13u, 0x3A92A4A5u, 0xE4C4AADCu, 0xACF7A858u, 0xD6B27D54u,
    0x306C08D2u, 0xF8F70665u, 0x11CE3121u, 0x33B98874u, 0x3A25619Bu, 0x18AADCCCu,
    0xFD7EF127u, 0xBAB7259Bu, 0x377E9519u, 0x7347CC50u, 0x12A0F79Cu, 0xDAFA9585u,
    0x520532AEu, 0x1A9EF4D7u, 0x7F011A92u, 0x5DE8B7DEu, 0x6831C877u, 0xADB116BCu,
    0x520F16F7u, 0x895DA953u, 0xAD4E6655u, 0x9B16DA62u, 0x6E1FCA2Cu, 0x26B62016u,
    0x34BF342Au, 0x06FE41C7u, 0x070D840Eu, 0xCC7849D6u, 0xF7B2A3AAu, 0xCD5E13A2u,
    0x361F4DA1u, 0x42FA0C32u, 0xFBE5AB20u, 0xA6F102A7u, 0xB993C598u, 0x3C926F42u,
    0x64C2AF3Du, 0xABB59CB6u, 0x4E8B46A3u, 0xD2C9A466u, 0xB8CD351Du, 0xBE1A3BD4u,
    0x07902E15u, 0x414D2F8Du, 0xCD2CB35Bu, 0x982AD78Du, 0xF77C1875u, 0x8F1F3EAEu,
    0xA68026C5u, 0xE5DA31E2u, 0xA361E7CCu, 0xC12FE0EEu, 0x67BF35B7u, 0x9EB2C046u,
    0xF8D2527Fu, 0x99BB4494u, 0x18B3BE04u, 0x150E9A42u, 0xB8FE1345u, 0x4188D3CCu,
    0x6C62AEB6u, 0xF41ED0ADu, 0xCA332E07u, 0x5ED4EB55u, 0xAEA965C1u, 0xAC0A9CE9u,
    0x27C35354u, 0x1B4DC403u, 0xB0C458B4u, 0xB3F5341Du, 0x099B7650u, 0xDDE64D0Au,
    0x3637B46Eu, 0x798D4FEAu, 0xA6F049F1u, 0x114C4FFDu, 0xDA2F9574u, 0x11B2F37Fu,
    0x1FB94067u, 0xE7189716u, 0x64355424u, 0xC88403C0u, 0x8322AAA6u, 0x002B58B4u,
    0xB66F2798u, 0x3827E813u, 0x1239FF31u, 0x1FB470C7u, 0x7F8BC7AEu, 0x3146AF34u,
    0xF0F47922u, 0x2C44D991u, 0x54FB7217u, 0xE326AE52u, 0x6286D547u, 0x0C357547u,
    0xE1A64EBEu, 0x1E5897E0u, 0xF2208B19u, 0xC1C43694u, 0xA5F23ED9u, 0xD7638795u,
    0x99F872F2u, 0xEAF0FFE0u, 0x27D47C4Du, 0x853B79FEu, 0xE035021Eu, 0xE8B414B4u,
    0x8CC98DACu, 0x78C731FAu, 0x538C631Eu, 0xCEE7AE21u, 0xB4FAAE66u, 0x83FC626Au,
    0xEBB55E85u, 0xBF3BE233u, 0x62EF00EEu, 0x8C83A6D5u, 0xBC1CF1EEu, 0xF79435ECu,
    0xA6D4EAAAu, 0xEEDFBF3Du, 0xEA04179Bu, 0x62004280u, 0xB3BA3010u, 0x51631E13u,
    0x3A92A4A5u, 0xE4C4AADCu, 0xACF7A858u, 0xD6B27D54u, 0x306C08D2u, 0xF8F70665u,
    0x11CE3121u, 0x33B98874u, 0x3A25619Bu, 0x18AADCCCu, 0xFD7EF127u, 0xBAB7259Bu,
    0x377E9519u, 0x7347CC50u, 0x12A0F79Cu, 0xDAFA9585u, 0x520532AEu, 0x1A9EF4D7u,
    0x7F011A92u, 0x5DE8B7DEu, 0x6831C877u, 0xADB116BCu, 0x520F16F7u, 0x895DA953u,
    0xAD4E6655u, 0x9B16DA62u, 0x6E1FCA2Cu, 0x26B62016u, 0x34BF342Au, 0x06FE41C7u,
    0x070D840Eu, 0xCC7849D6u, 0xF7B2A3AAu, 0xCD5E13A2u, 0x361F4DA1u, 0x42FA0C32u,
};

TEST(ascii_unchanged) {
    bool print = getenv("GRIMOIRE_FONT_CRC") && !strcmp(getenv("GRIMOIRE_FONT_CRC"), "print");
    int i = 0, compared = 0, bad = 0;
    if (print) printf("static const uint32_t k_crc[NCRC] = {\n");
    for (int theme = THEME_GOTHIC; theme <= THEME_CLASSIC; theme++) {
        ui_set_theme((UiTheme)theme);
        for (int face = 0; face < FONT_FACE_COUNT; face++)
            for (int z = 0; z < NSIZES; z++)
                for (int s = 0; s < STR_COUNT; s++, i++) {
                    uint32_t crc = pinned(theme, face, s) ? render_crc((FontFace)face, k_sizes[z], s) : 0;
                    if (print) {
                        printf("%s0x%08Xu,%s", i % 6 ? " " : "    ", crc, i % 6 == 5 || i == NCRC - 1 ? "\n" : "");
                        continue;
                    }
                    if (!crc || (outline(theme, face, s) && !OUTLINE_EXACT)) continue;
                    compared++;
                    if (crc != k_crc[i]) {
                        bad++;
                        fprintf(stderr, "  changed: %s face %d at %.1f px, string %d\n",
                                theme == THEME_GOTHIC ? "gothic" : "classic", face, k_sizes[z], s);
                    }
                }
    }
    if (print) printf("};\n");
    ui_set_theme(THEME_GOTHIC);
    CHECK(i == NCRC);
    CHECK(print || compared > NCRC / 2 || !OUTLINE_EXACT);
    CHECK(bad == 0);
}

int main(void) {
    for (int i = 0; i < 95; i++) g_ascii[i] = (char)(32 + i);
    crc_init();
    ui_init(300.f);
    if (font_init() != 0) return 1;
    RUN(ascii_unchanged);
    return tl_done();
}
