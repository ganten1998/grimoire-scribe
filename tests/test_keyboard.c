// test_keyboard.c: the on-screen keyboard through its public API (keyboard.h). The text is
// UTF-8: keys type whole characters, backspace removes whole characters, shift gives accented
// capitals, the accents page is one key away, and a name opened for renaming keeps its accents
// and is never cut inside a character.
#include <string.h>
#include <unistd.h>

#include "export.h"
#include "keyboard.h"
#include "testlib.h"

static Canvas g_c;
static const Rect k_screen = {0, 0, 1860, 2480};

// Presses and lifts the key with this code on the page showing now (the way a tap arrives);
// false if that page has no such key.
static bool press(Keyboard *kb, uint32_t code) {
    Rect r, d;
    if (!keyboard_key_rect(kb, code, &r)) return false;
    float x = (float)r.x + (float)r.w / 2.f, y = (float)r.y + (float)r.h / 2.f;
    keyboard_pointer(kb, 0, x, y, 0, &d);
    keyboard_pointer(kb, 2, x, y, 40, &d);
    keyboard_draw(kb, &g_c, k_screen);
    return true;
}

static void open_kb(Keyboard *kb, const char *initial) {
    keyboard_open(kb, "Name", initial);
    keyboard_draw(kb, &g_c, k_screen);
}

// Types valid UTF-8 key by key, as a person would: a capital with shift and its small letter's
// key, an accented letter from the accents page. False if a character has no key.
static bool type_text(Keyboard *kb, const char *s) {
    while (*s) {
        uint32_t cp = (unsigned char)*s++;
        if (cp >= 0xC0) {
            int extra = cp >= 0xF0 ? 3 : cp >= 0xE0 ? 2 : 1;
            cp &= 0x3Fu >> extra;
            while (extra--) cp = cp << 6 | ((unsigned char)*s++ & 0x3Fu);
        }
        uint32_t key = cp;
        for (uint32_t c = 0x20; c < 0x180 && key == cp; c++)
            if (c != cp && keyboard_upper(c) == cp) key = c;
        Rect r;
        if (!keyboard_key_rect(kb, key, &r)) press(kb, KB_KEY_PAGE);
        if (!keyboard_key_rect(kb, key, &r)) return false;
        if (key != cp && !kb->shift) press(kb, KB_KEY_SHIFT);
        press(kb, key);
    }
    return true;
}

// Whether s is well-formed UTF-8 (no stray, missing or overlong bytes).
static bool valid_utf8(const char *s) {
    const unsigned char *p = (const unsigned char *)s;
    while (*p) {
        int n = *p < 0x80 ? 0 : *p >= 0xC2 && *p < 0xE0 ? 1 : *p >= 0xE0 && *p < 0xF0 ? 2 : *p >= 0xF0 && *p < 0xF5 ? 3 : -1;
        if (n < 0) return false;
        p++;
        while (n--)
            if ((*p++ & 0xC0) != 0x80) return false;
    }
    return true;
}

TEST(types_musica_through_the_accents_page) {
    Keyboard kb;
    open_kb(&kb, "");
    CHECK(kb.page == 0 && !keyboard_key_rect(&kb, 0xFA, &(Rect){0}));
    CHECK(press(&kb, KB_KEY_SHIFT) && kb.shift);
    CHECK(press(&kb, 'm') && !kb.shift);  // one-shot
    CHECK(press(&kb, KB_KEY_PAGE) && kb.page == 1);
    CHECK(!keyboard_key_rect(&kb, 'q', &(Rect){0}));  // the letters are on the other page
    CHECK(press(&kb, 0xFA));                           // u acute
    CHECK(!strcmp(kb.text, "M\xc3\xba"));
    CHECK(press(&kb, KB_KEY_BKSP) && !strcmp(kb.text, "M"));  // the whole character
    CHECK(press(&kb, 0xFA) && kb.page == 1);                  // typing stays on the page
    CHECK(press(&kb, KB_KEY_PAGE) && kb.page == 0);
    CHECK(press(&kb, 's') && press(&kb, 'i') && press(&kb, 'c') && press(&kb, 'a') && press(&kb, ' '));
    CHECK(press(&kb, KB_KEY_PAGE) && press(&kb, KB_KEY_SHIFT) && kb.shift);
    CHECK(press(&kb, 0xF1) && !kb.shift);  // n tilde, shifted: N tilde, and shift lets go
    CHECK(!strcmp(kb.text, "M\xc3\xbasica \xc3\x91"));
    CHECK(press(&kb, KB_KEY_BKSP) && !strcmp(kb.text, "M\xc3\xbasica "));
    CHECK(press(&kb, KB_KEY_BKSP) && !strcmp(kb.text, "M\xc3\xbasica"));
    CHECK(press(&kb, KB_KEY_PAGE) && kb.page == 0);
    // Backspace through the accent to the start, and once more on nothing.
    for (int i = 0; i < 7; i++) CHECK(press(&kb, KB_KEY_BKSP) && valid_utf8(kb.text));
    CHECK(kb.text[0] == 0);
}

TEST(shift_gives_accented_capitals) {
    Keyboard kb;
    open_kb(&kb, "");
    press(&kb, KB_KEY_PAGE);
    // Each key on the page, shifted: the capital where one exists, otherwise the key itself
    // (and then shift stays on, as it does for digits).
    static const struct {
        uint32_t key, typed;
    } k[] = {{0xE1, 0xC1}, {0xE9, 0xC9}, {0xED, 0xCD}, {0xF3, 0xD3}, {0xFA, 0xDA}, {0xE0, 0xC0}, {0xE8, 0xC8},
             {0xEC, 0xCC}, {0xF2, 0xD2}, {0xF9, 0xD9}, {0xE2, 0xC2}, {0xEA, 0xCA}, {0xEE, 0xCE}, {0xF4, 0xD4},
             {0xFB, 0xDB}, {0xE4, 0xC4}, {0xEB, 0xCB}, {0xEF, 0xCF}, {0xF6, 0xD6}, {0xFC, 0xDC}, {0xF1, 0xD1},
             {0xE7, 0xC7}, {0xE3, 0xC3}, {0xF5, 0xD5}, {0xDF, 0xDF}, {0x153, 0x152}, {0xE6, 0xC6}, {0xF8, 0xD8},
             {0xE5, 0xC5}, {0xBF, 0xBF}, {0xA1, 0xA1}, {0xAB, 0xAB}, {0xBB, 0xBB}, {0xB7, 0xB7}};
    for (int i = 0; i < (int)(sizeof k / sizeof k[0]); i++) {
        kb.text[0] = 0;
        if (!kb.shift) press(&kb, KB_KEY_SHIFT);
        bool there = press(&kb, k[i].key);
        char want[4] = {0};
        if (k[i].typed < 0x800) want[0] = (char)(0xC0 | k[i].typed >> 6), want[1] = (char)(0x80 | (k[i].typed & 0x3F));
        bool ok = there && !strcmp(kb.text, want) && kb.shift == (k[i].typed == k[i].key);
        if (!ok) fprintf(stderr, "  key U+%04X typed \"%s\"\n", k[i].key, kb.text);
        CHECK(ok);
    }
}

TEST(upper_table) {
    for (uint32_t c = 'a'; c <= 'z'; c++) CHECK(keyboard_upper(c) == c - 32);
    for (uint32_t c = 0xE0; c <= 0xFE; c++) CHECK(keyboard_upper(c) == (c == 0xF7 ? c : c - 32));
    static const uint32_t k[][2] = {
        {'A', 'A'},     {'1', '1'},     {0xDF, 0xDF},   {0xF7, 0xF7},   {0xFF, 0x178},  {0xB5, 0xB5},
        {0x100, 0x100}, {0x101, 0x100}, {0x107, 0x106}, {0x10D, 0x10C}, {0x119, 0x118}, {0x11F, 0x11E},
        {0x130, 0x130}, {0x131, 'I'},   {0x133, 0x132}, {0x137, 0x136}, {0x138, 0x138}, {0x139, 0x139},
        {0x13A, 0x139}, {0x13F, 0x13F}, {0x140, 0x13F}, {0x142, 0x141}, {0x144, 0x143}, {0x148, 0x147},
        {0x149, 0x149}, {0x14B, 0x14A}, {0x151, 0x150}, {0x153, 0x152}, {0x159, 0x158}, {0x15B, 0x15A},
        {0x15F, 0x15E}, {0x161, 0x160}, {0x16F, 0x16E}, {0x171, 0x170}, {0x177, 0x176}, {0x178, 0x178},
        {0x17A, 0x179}, {0x17C, 0x17B}, {0x17E, 0x17D}, {0x17F, 0x17F}, {0x180, 0x180}, {0x20AC, 0x20AC}};
    for (int i = 0; i < (int)(sizeof k / sizeof k[0]); i++) {
        if (keyboard_upper(k[i][0]) != k[i][1]) fprintf(stderr, "  U+%04X -> U+%04X\n", k[i][0], keyboard_upper(k[i][0]));
        CHECK(keyboard_upper(k[i][0]) == k[i][1]);
    }
    for (uint32_t c = 0; c < 0x200; c++) CHECK(keyboard_upper(keyboard_upper(c)) == keyboard_upper(c));
}

TEST(open_keeps_accents) {
    Keyboard kb;
    open_kb(&kb, "M\xc3\xbasica");
    CHECK(!strcmp(kb.text, "M\xc3\xbasica"));
    open_kb(&kb, "\xc2\xbf" "Cu\xc3\xa1nto? \xc3\x91" "and\xc3\xba \xc5\x92uvre \xe2\x82\xac 5 \xf0\x9f\x93\x93");
    CHECK(!strcmp(kb.text, "\xc2\xbf" "Cu\xc3\xa1nto? \xc3\x91" "and\xc3\xba \xc5\x92uvre \xe2\x82\xac 5 \xf0\x9f\x93\x93"));
    // Control characters and broken UTF-8 go: a C0 control, a stray 0xFF, a lead byte without
    // its continuation, a tab, DEL, a C1 control, a surrogate, an overlong slash, a cut-off
    // character at the end.
    open_kb(&kb, "A\x01" "B\xff" "C\xc3(" "D\t\x7f" "\xc2\x85" "E\xed\xa0\x80" "F\xc0\xaf" "G\xe2\x82");
    CHECK(!strcmp(kb.text, "ABC(DEFG"));
    open_kb(&kb, NULL);
    CHECK(kb.text[0] == 0);
}

TEST(never_cut_inside_a_character) {
    Keyboard kb;
    char two[200] = "", three[200] = "", odd[200] = "a";
    for (int i = 0; i < 60; i++) strcat(two, "\xc3\xa9");      // e acute, 2 bytes
    for (int i = 0; i < 60; i++) strcat(three, "\xe2\x82\xac");  // euro sign, 3 bytes
    for (int i = 0; i < 60; i++) strcat(odd, "\xc3\xb1");      // n tilde after one ASCII byte
    open_kb(&kb, two);
    CHECK(strlen(kb.text) == 94 && valid_utf8(kb.text));  // 47 of them; a 48th would need 96 bytes
    open_kb(&kb, three);
    CHECK(strlen(kb.text) == 93 && valid_utf8(kb.text));
    open_kb(&kb, odd);
    CHECK(strlen(kb.text) == 95 && valid_utf8(kb.text));
    // Typing into a full field: nothing that would not fit is typed.
    open_kb(&kb, two);
    press(&kb, KB_KEY_PAGE);
    CHECK(press(&kb, 0xE9) && strlen(kb.text) == 94);
    press(&kb, KB_KEY_PAGE);
    CHECK(press(&kb, 'z') && strlen(kb.text) == 95 && valid_utf8(kb.text));
    CHECK(press(&kb, 'z') && strlen(kb.text) == 95);
    CHECK(press(&kb, KB_KEY_BKSP) && strlen(kb.text) == 94);
    CHECK(press(&kb, KB_KEY_BKSP) && strlen(kb.text) == 92 && valid_utf8(kb.text));
    // From empty, by key only: e acute until full.
    open_kb(&kb, "");
    press(&kb, KB_KEY_PAGE);
    for (int i = 0; i < 60; i++) press(&kb, 0xE9);
    CHECK(strlen(kb.text) == 94 && valid_utf8(kb.text) && sizeof kb.text == KB_TEXT_MAX);
}

// The tail of a long name steps whole characters, so the field never starts with a broken
// one. In Gothic the field's text is all pixel-face glyphs, pure black on white; a broken
// character would draw U+FFFD from Terminess, anti-aliased, with grays. Titles of every length
// move where the cut falls. (47 oe overflow the field after any of these titles.)
TEST(long_name_tail_is_whole_characters) {
    ui_set_theme(THEME_GOTHIC);
    Keyboard kb;
    char many[100] = "";
    for (int i = 0; i < 47; i++) strcat(many, "\xc5\x93");
    static const char words[] = "Name of the new folder";
    for (int t = 1; t < (int)sizeof words; t++) {
        char title[sizeof words];
        memcpy(title, words, (size_t)t);
        title[t] = 0;
        keyboard_open(&kb, title, many);
        keyboard_draw(&kb, &g_c, k_screen);
        Rect in = rect_inflate((Rect){kb.area.x + 30, kb.area.y + 30, kb.area.w - 60, 120}, -20);  // inside its frame
        int grays = 0;
        for (int y = in.y; y < in.y + in.h; y++)
            for (int x = in.x; x < in.x + in.w; x++) {
                uint8_t v = g_c.px[y * g_c.stride + x];
                grays += v > 1 && v < 255;
            }
        if (grays) fprintf(stderr, "  title \"%s\": the field starts with a broken character\n", title);
        CHECK(grays == 0);
    }
}

// The letters page kept its keys where they were (tests/scenarios/b_library.qs types "School"
// by coordinates), with the accents key beside the space bar.
TEST(letters_layout_kept) {
    Keyboard kb;
    open_kb(&kb, "");
    static const int taps[][2] = {{175, 2205}, {398, 2045}, {747, 2205}, {1106, 2045}, {1549, 1885}, {1549, 1885}, {1637, 2045}};
    Rect d;
    for (int i = 0; i < 7; i++) {
        keyboard_pointer(&kb, 0, (float)taps[i][0], (float)taps[i][1], 0, &d);
        keyboard_pointer(&kb, 2, (float)taps[i][0], (float)taps[i][1], 40, &d);
    }
    CHECK(!strcmp(kb.text, "School"));
    Rect r = {0}, sh = {0}, sp = {0};
    CHECK(keyboard_key_rect(&kb, KB_KEY_PAGE, &r) && keyboard_key_rect(&kb, KB_KEY_SHIFT, &sh) && keyboard_key_rect(&kb, ' ', &sp));
    CHECK(r.w == sh.w && r.h == sh.h && r.y == sp.y && r.x + r.w < sp.x);
    keyboard_pointer(&kb, 0, 1643, 2365, 0, &d);
    CHECK(keyboard_pointer(&kb, 2, 1643, 2365, 40, &d) == KB_DONE);
}

// In Spanish the letters page has n tilde right of l, as Spanish keyboards do: "Año" and "AÑO"
// type without the accents page. The row is then as wide as the others; in English it is not
// there (letters_layout_kept holds the English page's keys where they were).
TEST(spanish_letters_have_enye) {
    i18n_set("es");
    Keyboard kb;
    open_kb(&kb, "");
    Rect l = {0}, n = {0}, q = {0}, p = {0};
    CHECK(keyboard_key_rect(&kb, 0xF1, &n) && keyboard_key_rect(&kb, 'l', &l));
    CHECK(n.y == l.y && n.x > l.x + l.w && n.w == l.w);
    CHECK(keyboard_key_rect(&kb, 'q', &q) && keyboard_key_rect(&kb, 'p', &p));
    CHECK(n.x + n.w == p.x + p.w);
    press(&kb, 'a');
    press(&kb, 0xF1);
    press(&kb, 'o');
    CHECK(!kb.page && !strcmp(kb.text, "a\xc3\xb1o"));
    press(&kb, KB_KEY_SHIFT);
    press(&kb, 0xF1);
    CHECK(!kb.page && !strcmp(kb.text, "a\xc3\xb1o\xc3\x91"));
    i18n_set("en");
    open_kb(&kb, "");
    CHECK(!keyboard_key_rect(&kb, 0xF1, &n));
    press(&kb, KB_KEY_PAGE);
    CHECK(keyboard_key_rect(&kb, 0xF1, &n));  // still on the accents page in every language
}

// Every character on the accents page, and its capital, is in every face of both themes (no
// borrowing from Terminess, no missing glyph).
TEST(accents_in_every_face) {
    Keyboard kb;
    open_kb(&kb, "");
    press(&kb, KB_KEY_PAGE);
    int found = 0;
    for (uint32_t cp = 0xA0; cp < 0x180; cp++) {
        Rect r;
        if (!keyboard_key_rect(&kb, cp, &r)) continue;
        found++;
        for (int theme = THEME_GOTHIC; theme <= THEME_CLASSIC; theme++) {
            ui_set_theme((UiTheme)theme);
            for (int f = 0; f < FONT_FACE_COUNT; f++) {
                bool ok = font_has_glyph((FontFace)f, cp) && font_has_glyph((FontFace)f, keyboard_upper(cp));
                if (!ok) fprintf(stderr, "  U+%04X missing in face %d (theme %d)\n", cp, f, theme);
                CHECK(ok);
            }
        }
    }
    ui_set_theme(THEME_GOTHIC);
    CHECK(found == 34);
}

// A key with one character shows it at the title size, whatever its length in UTF-8 bytes; a
// key with a word ("abc", "shift") at the text size. Inside its frame, each key holds exactly
// what font_draw_centered draws at that size, in both styles.
TEST(one_character_keys_at_title_size) {
    static const struct {
        int page;
        uint32_t code;
        const char *label;
        bool word;
    } k[] = {{0, 'a', "a", false},           {1, 0xE1, "\xc3\xa1", false},     {1, 0xF1, "\xc3\xb1", false},
             {1, 0xFC, "\xc3\xbc", false},   {1, 0x153, "\xc5\x93", false},    {1, 0xBF, "\xc2\xbf", false},
             {1, 0xB7, "\xc2\xb7", false},   {1, KB_KEY_PAGE, "abc", true},    {0, KB_KEY_SHIFT, "shift", true},
             {0, KB_KEY_PAGE, "\xc3\xa1\xc3\xa9", true}};
    for (int theme = THEME_GOTHIC; theme <= THEME_CLASSIC; theme++) {
        ui_set_theme((UiTheme)theme);
        Keyboard kb;
        open_kb(&kb, "");
        for (int i = 0; i < (int)(sizeof k / sizeof k[0]); i++) {
            if (kb.page != k[i].page) press(&kb, KB_KEY_PAGE);
            Rect r;
            Canvas want;
            if (!keyboard_key_rect(&kb, k[i].code, &r) || canvas_init(&want, r.w, r.h) != 0) {
                CHECK(!"key");
                continue;
            }
            canvas_fill(&want, (Rect){0, 0, r.w, r.h}, 255);
            font_draw_centered(&want, FONT_BOLD, k[i].word ? ui_text_size() : ui_title_size(), (Rect){0, 0, r.w, r.h},
                               k[i].label, 0);
            int differ = 0;
            for (int y = 20; y < r.h - 20; y++)  // clear of the frame
                for (int x = 20; x < r.w - 20; x++)
                    differ += g_c.px[(r.y + y) * g_c.stride + r.x + x] != want.px[y * want.stride + x];
            if (differ) fprintf(stderr, "  key \"%s\" (theme %d): %d pixels off\n", k[i].label, theme, differ);
            CHECK(differ == 0);
            canvas_free(&want);
        }
    }
    ui_set_theme(THEME_GOTHIC);
}

// What the keyboard types is what the Library keeps. Rename hands the keyboard's text to
// store_rename (kb_finished in library.c), and the mirror PDF takes the name and the folder,
// accents and all, as its path (export_mirror_path; mirror_follow moves the PDF there). Renaming
// again opens the keyboard on the stored name, whole.
TEST(typed_name_is_stored_whole) {
    char root[600], cmd[700], mirror[700], path[1400], want[1400];
    snprintf(root, sizeof root, "%s/keyboard-%d", tl_tmp(), (int)getpid());
    snprintf(cmd, sizeof cmd, "rm -rf '%s'", root);
    snprintf(mirror, sizeof mirror, "%s/PDFs", root);
    system(cmd);
    static NotebookInfo nb, back;
    if (store_init(root) != 0 || store_create("Notebook Oct 19", "", PAPER_MEDIUM, &nb) != 0) {
        CHECK(!"store");
        return;
    }
    static const char name[] = "M\xc3\xbasica espa\xc3\xb1ola \xc2\xa1Ol\xc3\xa9", folder[] = "Matem\xc3\xa1ticas";
    Keyboard kb;
    open_kb(&kb, nb.name);
    for (int i = 0; i < 40 && kb.text[0]; i++) press(&kb, KB_KEY_BKSP);
    CHECK(type_text(&kb, name) && !strcmp(kb.text, name));
    CHECK(store_rename(nb.id, kb.text) == 0 && store_load_info(nb.id, &back) == 0 && !strcmp(back.name, name));
    open_kb(&kb, "");
    CHECK(type_text(&kb, folder) && !strcmp(kb.text, folder));
    CHECK(store_set_folder(nb.id, kb.text) == 0 && store_load_info(nb.id, &back) == 0);
    export_mirror_path(&back, mirror, path, sizeof path);
    snprintf(want, sizeof want, "%s/%s/%s.pdf", mirror, folder, name);
    if (strcmp(path, want)) fprintf(stderr, "  mirror PDF at \"%s\"\n", path);
    CHECK(!strcmp(path, want));
    CHECK(export_mirror_pdf(&back, 300.f, mirror) == 0 && access(want, F_OK) == 0);
    open_kb(&kb, back.name);
    CHECK(!strcmp(kb.text, name));
    system(cmd);
}

int main(void) {
    ui_init(300.f);
    if (font_init() != 0 || canvas_init(&g_c, k_screen.w, k_screen.h) != 0) return 1;
    ui_set_theme(THEME_GOTHIC);
    RUN(types_musica_through_the_accents_page);
    RUN(shift_gives_accented_capitals);
    RUN(upper_table);
    RUN(open_keeps_accents);
    RUN(never_cut_inside_a_character);
    RUN(long_name_tail_is_whole_characters);
    RUN(letters_layout_kept);
    RUN(spanish_letters_have_enye);
    RUN(accents_in_every_face);
    RUN(one_character_keys_at_title_size);
    RUN(typed_name_is_stored_whole);
    canvas_free(&g_c);
    return tl_done();
}
