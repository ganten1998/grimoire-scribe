// test_keyboard.c: the on-screen keyboard through its public API (keyboard.h). The text is
// UTF-8: keys type whole characters, backspace removes whole characters, shift gives accented
// capitals, the accents page is one key away, and a name opened for renaming keeps its accents
// and is never cut inside a character.
mod testlib;

use grimoire::prelude::*;
use std::cell::RefCell;
use testlib::*;

thread_local! {
    static G_C: RefCell<Canvas> = RefCell::new(Canvas::default());
}
const K_SCREEN: Rect = Rect { x: 0, y: 0, w: 1860, h: 2480 };

fn draw(kb: &mut Keyboard) {
    G_C.with(|c| keyboard_draw(kb, &mut c.borrow_mut(), K_SCREEN));
}

// Presses and lifts the key with this code on the page showing now (the way a tap arrives);
// false if that page has no such key.
fn press(kb: &mut Keyboard, code: u32) -> bool {
    let mut r = Rect::default();
    let mut d = Rect::default();
    if !keyboard_key_rect(kb, code, &mut r) {
        return false;
    }
    let x = r.x as f32 + r.w as f32 / 2.0;
    let y = r.y as f32 + r.h as f32 / 2.0;
    keyboard_pointer(kb, 0, x, y, 0, &mut d);
    keyboard_pointer(kb, 2, x, y, 40, &mut d);
    draw(kb);
    true
}

fn open_kb(kb: &mut Keyboard, initial: &[u8]) {
    keyboard_open(kb, "Name", initial);
    draw(kb);
}

// Types valid UTF-8 key by key, as a person would: a capital with shift and its small letter's
// key, an accented letter from the accents page. False if a character has no key.
fn type_text(kb: &mut Keyboard, s: &str) -> bool {
    for ch in s.chars() {
        let cp = ch as u32;
        let mut key = cp;
        let mut c = 0x20;
        while c < 0x180 && key == cp {
            if c != cp && keyboard_upper(c) == cp {
                key = c;
            }
            c += 1;
        }
        let mut r = Rect::default();
        if !keyboard_key_rect(kb, key, &mut r) {
            press(kb, KB_KEY_PAGE);
        }
        if !keyboard_key_rect(kb, key, &mut r) {
            return false;
        }
        if key != cp && !kb.shift {
            press(kb, KB_KEY_SHIFT);
        }
        press(kb, key);
    }
    true
}

// Whether s is well-formed UTF-8 (no stray, missing or overlong bytes).
fn valid_utf8(s: &[u8]) -> bool {
    let mut i = 0;
    while i < s.len() {
        let c = s[i];
        let mut n: i32 = if c < 0x80 {
            0
        } else if (0xC2..0xE0).contains(&c) {
            1
        } else if (0xE0..0xF0).contains(&c) {
            2
        } else if (0xF0..0xF5).contains(&c) {
            3
        } else {
            -1
        };
        if n < 0 {
            return false;
        }
        i += 1;
        while n > 0 {
            n -= 1;
            // (a missing byte is the string's NUL in the C, which is no continuation either)
            if i >= s.len() || (s[i] & 0xC0) != 0x80 {
                return false;
            }
            i += 1;
        }
    }
    true
}

fn text(kb: &Keyboard) -> &[u8] {
    kb.text.as_bytes()
}

fn types_musica_through_the_accents_page() {
    let mut kb = Keyboard::default();
    open_kb(&mut kb, b"");
    check!(kb.page == 0 && !keyboard_key_rect(&kb, 0xFA, &mut Rect::default()));
    check!(press(&mut kb, KB_KEY_SHIFT) && kb.shift);
    check!(press(&mut kb, 'm' as u32) && !kb.shift); // one-shot
    check!(press(&mut kb, KB_KEY_PAGE) && kb.page == 1);
    check!(!keyboard_key_rect(&kb, 'q' as u32, &mut Rect::default())); // the letters are on the other page
    check!(press(&mut kb, 0xFA)); // u acute
    check!(text(&kb) == b"M\xc3\xba");
    check!(press(&mut kb, KB_KEY_BKSP) && text(&kb) == b"M"); // the whole character
    check!(press(&mut kb, 0xFA) && kb.page == 1); // typing stays on the page
    check!(press(&mut kb, KB_KEY_PAGE) && kb.page == 0);
    check!(press(&mut kb, 's' as u32) && press(&mut kb, 'i' as u32) && press(&mut kb, 'c' as u32) && press(&mut kb, 'a' as u32) && press(&mut kb, ' ' as u32));
    check!(press(&mut kb, KB_KEY_PAGE) && press(&mut kb, KB_KEY_SHIFT) && kb.shift);
    check!(press(&mut kb, 0xF1) && !kb.shift); // n tilde, shifted: N tilde, and shift lets go
    check!(text(&kb) == b"M\xc3\xbasica \xc3\x91");
    check!(press(&mut kb, KB_KEY_BKSP) && text(&kb) == b"M\xc3\xbasica ");
    check!(press(&mut kb, KB_KEY_BKSP) && text(&kb) == b"M\xc3\xbasica");
    check!(press(&mut kb, KB_KEY_PAGE) && kb.page == 0);
    // Backspace through the accent to the start, and once more on nothing.
    for _ in 0..7 {
        check!(press(&mut kb, KB_KEY_BKSP) && valid_utf8(text(&kb)));
    }
    check!(text(&kb).is_empty());
}

fn shift_gives_accented_capitals() {
    let mut kb = Keyboard::default();
    open_kb(&mut kb, b"");
    press(&mut kb, KB_KEY_PAGE);
    // Each key on the page, shifted: the capital where one exists, otherwise the key itself
    // (and then shift stays on, as it does for digits).
    const K: [(u32, u32); 34] = [
        (0xE1, 0xC1), (0xE9, 0xC9), (0xED, 0xCD), (0xF3, 0xD3), (0xFA, 0xDA), (0xE0, 0xC0), (0xE8, 0xC8),
        (0xEC, 0xCC), (0xF2, 0xD2), (0xF9, 0xD9), (0xE2, 0xC2), (0xEA, 0xCA), (0xEE, 0xCE), (0xF4, 0xD4),
        (0xFB, 0xDB), (0xE4, 0xC4), (0xEB, 0xCB), (0xEF, 0xCF), (0xF6, 0xD6), (0xFC, 0xDC), (0xF1, 0xD1),
        (0xE7, 0xC7), (0xE3, 0xC3), (0xF5, 0xD5), (0xDF, 0xDF), (0x153, 0x152), (0xE6, 0xC6), (0xF8, 0xD8),
        (0xE5, 0xC5), (0xBF, 0xBF), (0xA1, 0xA1), (0xAB, 0xAB), (0xBB, 0xBB), (0xB7, 0xB7),
    ];
    for (key, typed) in K {
        kb.text.clear();
        if !kb.shift {
            press(&mut kb, KB_KEY_SHIFT);
        }
        let there = press(&mut kb, key);
        let mut want = [0u8; 4];
        if typed < 0x800 {
            want[0] = (0xC0 | typed >> 6) as u8;
            want[1] = (0x80 | (typed & 0x3F)) as u8;
        }
        let end = want.iter().position(|&b| b == 0).unwrap_or(4);
        let ok = there && text(&kb) == &want[..end] && kb.shift == (typed == key);
        if !ok {
            eprintln!("  key U+{:04X} typed \"{}\"", key, kb.text);
        }
        check!(ok);
    }
}

fn upper_table() {
    for c in 'a' as u32..='z' as u32 {
        check!(keyboard_upper(c) == c - 32);
    }
    for c in 0xE0u32..=0xFE {
        check!(keyboard_upper(c) == if c == 0xF7 { c } else { c - 32 });
    }
    const K: [[u32; 2]; 42] = [
        ['A' as u32, 'A' as u32], ['1' as u32, '1' as u32], [0xDF, 0xDF], [0xF7, 0xF7], [0xFF, 0x178], [0xB5, 0xB5],
        [0x100, 0x100], [0x101, 0x100], [0x107, 0x106], [0x10D, 0x10C], [0x119, 0x118], [0x11F, 0x11E],
        [0x130, 0x130], [0x131, 'I' as u32], [0x133, 0x132], [0x137, 0x136], [0x138, 0x138], [0x139, 0x139],
        [0x13A, 0x139], [0x13F, 0x13F], [0x140, 0x13F], [0x142, 0x141], [0x144, 0x143], [0x148, 0x147],
        [0x149, 0x149], [0x14B, 0x14A], [0x151, 0x150], [0x153, 0x152], [0x159, 0x158], [0x15B, 0x15A],
        [0x15F, 0x15E], [0x161, 0x160], [0x16F, 0x16E], [0x171, 0x170], [0x177, 0x176], [0x178, 0x178],
        [0x17A, 0x179], [0x17C, 0x17B], [0x17E, 0x17D], [0x17F, 0x17F], [0x180, 0x180], [0x20AC, 0x20AC],
    ];
    for k in K {
        if keyboard_upper(k[0]) != k[1] {
            eprintln!("  U+{:04X} -> U+{:04X}", k[0], keyboard_upper(k[0]));
        }
        check!(keyboard_upper(k[0]) == k[1]);
    }
    for c in 0..0x200u32 {
        check!(keyboard_upper(keyboard_upper(c)) == keyboard_upper(c));
    }
}

fn open_keeps_accents() {
    let mut kb = Keyboard::default();
    open_kb(&mut kb, "M\u{fa}sica".as_bytes());
    check!(text(&kb) == b"M\xc3\xbasica");
    let long = "\u{bf}Cu\u{e1}nto? \u{d1}and\u{fa} \u{152}uvre \u{20ac} 5 \u{1f4d3}";
    open_kb(&mut kb, long.as_bytes());
    check!(text(&kb) == long.as_bytes());
    // Control characters and broken UTF-8 go: a C0 control, a stray 0xFF, a lead byte without
    // its continuation, a tab, DEL, a C1 control, a surrogate, an overlong slash, a cut-off
    // character at the end.
    open_kb(&mut kb, b"A\x01B\xffC\xc3(D\t\x7f\xc2\x85E\xed\xa0\x80F\xc0\xafG\xe2\x82");
    check!(text(&kb) == b"ABC(DEFG");
    // (The C also opens it on NULL; here an empty slice is the nothing there is to pass.)
    open_kb(&mut kb, b"");
    check!(text(&kb).is_empty());
}

fn never_cut_inside_a_character() {
    let mut kb = Keyboard::default();
    let two = "\u{e9}".repeat(60); // e acute, 2 bytes
    let three = "\u{20ac}".repeat(60); // euro sign, 3 bytes
    let odd = format!("a{}", "\u{f1}".repeat(60)); // n tilde after one ASCII byte
    open_kb(&mut kb, two.as_bytes());
    check!(text(&kb).len() == 94 && valid_utf8(text(&kb))); // 47 of them; a 48th would need 96 bytes
    open_kb(&mut kb, three.as_bytes());
    check!(text(&kb).len() == 93 && valid_utf8(text(&kb)));
    open_kb(&mut kb, odd.as_bytes());
    check!(text(&kb).len() == 95 && valid_utf8(text(&kb)));
    // Typing into a full field: nothing that would not fit is typed.
    open_kb(&mut kb, two.as_bytes());
    press(&mut kb, KB_KEY_PAGE);
    check!(press(&mut kb, 0xE9) && text(&kb).len() == 94);
    press(&mut kb, KB_KEY_PAGE);
    check!(press(&mut kb, 'z' as u32) && text(&kb).len() == 95 && valid_utf8(text(&kb)));
    check!(press(&mut kb, 'z' as u32) && text(&kb).len() == 95);
    check!(press(&mut kb, KB_KEY_BKSP) && text(&kb).len() == 94);
    check!(press(&mut kb, KB_KEY_BKSP) && text(&kb).len() == 92 && valid_utf8(text(&kb)));
    // From empty, by key only: e acute until full.
    open_kb(&mut kb, b"");
    press(&mut kb, KB_KEY_PAGE);
    for _ in 0..60 {
        press(&mut kb, 0xE9);
    }
    check!(text(&kb).len() == 94 && valid_utf8(text(&kb)) && std::mem::size_of_val(&kb.text) == KB_TEXT_MAX);
}

// The tail of a long name steps whole characters, so the field never starts with a broken
// one. In Gothic the field's text is all pixel-face glyphs, pure black on white; a broken
// character would draw U+FFFD from Terminess, anti-aliased, with grays. Titles of every length
// move where the cut falls. (47 oe overflow the field after any of these titles.)
fn long_name_tail_is_whole_characters() {
    ui_set_theme(UiTheme::Gothic);
    let mut kb = Keyboard::default();
    let many = "\u{153}".repeat(47);
    const WORDS: &str = "Name of the new folder";
    // (the C's sizeof words counts the NUL: t runs to the whole title)
    for t in 1..=WORDS.len() {
        let title = &WORDS[..t];
        keyboard_open(&mut kb, title, many.as_bytes());
        draw(&mut kb);
        let inside = rect_inflate(Rect { x: kb.area.x + 30, y: kb.area.y + 30, w: kb.area.w - 60, h: 120 }, -20); // inside its frame
        let mut grays = 0;
        G_C.with(|c| {
            let c = c.borrow();
            for y in inside.y..inside.y + inside.h {
                for x in inside.x..inside.x + inside.w {
                    let v = c.px[(y * c.stride + x) as usize];
                    grays += (v > 1 && v < 255) as i32;
                }
            }
        });
        if grays != 0 {
            eprintln!("  title \"{}\": the field starts with a broken character", title);
        }
        check!(grays == 0);
    }
}

// The letters page kept its keys where they were (tests/scenarios/b_library.qs types "School"
// by coordinates), with the accents key beside the space bar.
fn letters_layout_kept() {
    let mut kb = Keyboard::default();
    open_kb(&mut kb, b"");
    const TAPS: [[i32; 2]; 7] = [[175, 2205], [398, 2045], [747, 2205], [1106, 2045], [1549, 1885], [1549, 1885], [1637, 2045]];
    let mut d = Rect::default();
    for t in TAPS {
        keyboard_pointer(&mut kb, 0, t[0] as f32, t[1] as f32, 0, &mut d);
        keyboard_pointer(&mut kb, 2, t[0] as f32, t[1] as f32, 40, &mut d);
    }
    check!(text(&kb) == b"School");
    let (mut r, mut sh, mut sp) = (Rect::default(), Rect::default(), Rect::default());
    check!(keyboard_key_rect(&kb, KB_KEY_PAGE, &mut r) && keyboard_key_rect(&kb, KB_KEY_SHIFT, &mut sh) && keyboard_key_rect(&kb, ' ' as u32, &mut sp));
    check!(r.w == sh.w && r.h == sh.h && r.y == sp.y && r.x + r.w < sp.x);
    keyboard_pointer(&mut kb, 0, 1643.0, 2365.0, 0, &mut d);
    check!(keyboard_pointer(&mut kb, 2, 1643.0, 2365.0, 40, &mut d) == KbResult::Done);
}

// In Spanish the letters page has n tilde right of l, as Spanish keyboards do: "Año" and "AÑO"
// type without the accents page. The row is then as wide as the others; in English it is not
// there (letters_layout_kept holds the English page's keys where they were).
fn spanish_letters_have_enye() {
    i18n_set("es");
    let mut kb = Keyboard::default();
    open_kb(&mut kb, b"");
    let (mut l, mut n, mut q, mut p) = (Rect::default(), Rect::default(), Rect::default(), Rect::default());
    check!(keyboard_key_rect(&kb, 0xF1, &mut n) && keyboard_key_rect(&kb, 'l' as u32, &mut l));
    check!(n.y == l.y && n.x > l.x + l.w && n.w == l.w);
    check!(keyboard_key_rect(&kb, 'q' as u32, &mut q) && keyboard_key_rect(&kb, 'p' as u32, &mut p));
    check!(n.x + n.w == p.x + p.w);
    press(&mut kb, 'a' as u32);
    press(&mut kb, 0xF1);
    press(&mut kb, 'o' as u32);
    check!(kb.page == 0 && text(&kb) == b"a\xc3\xb1o");
    press(&mut kb, KB_KEY_SHIFT);
    press(&mut kb, 0xF1);
    check!(kb.page == 0 && text(&kb) == b"a\xc3\xb1o\xc3\x91");
    i18n_set("en");
    open_kb(&mut kb, b"");
    check!(!keyboard_key_rect(&kb, 0xF1, &mut n));
    press(&mut kb, KB_KEY_PAGE);
    check!(keyboard_key_rect(&kb, 0xF1, &mut n)); // still on the accents page in every language
}

// Every character on the accents page, and its capital, is in every face of both themes (no
// borrowing from Terminess, no missing glyph).
fn accents_in_every_face() {
    let mut kb = Keyboard::default();
    open_kb(&mut kb, b"");
    press(&mut kb, KB_KEY_PAGE);
    let mut found = 0;
    const FACES: [FontFace; FontFace::COUNT] = [FontFace::Regular, FontFace::Bold, FontFace::Title, FontFace::Caps, FontFace::Fixed, FontFace::FixedBold];
    for cp in 0xA0u32..0x180 {
        let mut r = Rect::default();
        if !keyboard_key_rect(&kb, cp, &mut r) {
            continue;
        }
        found += 1;
        for (theme, th) in [UiTheme::Gothic, UiTheme::Classic].into_iter().enumerate() {
            ui_set_theme(th);
            for (f, face) in FACES.into_iter().enumerate() {
                let ok = font_has_glyph(face, cp) && font_has_glyph(face, keyboard_upper(cp));
                if !ok {
                    eprintln!("  U+{:04X} missing in face {} (theme {})", cp, f, theme);
                }
                check!(ok);
            }
        }
    }
    ui_set_theme(UiTheme::Gothic);
    check!(found == 34);
}

// A key with one character shows it at the title size, whatever its length in UTF-8 bytes; a
// key with a word ("abc", "shift") at the text size. Inside its frame, each key holds exactly
// what font_draw_centered draws at that size, in both styles.
fn one_character_keys_at_title_size() {
    // page, code, label, word
    const K: [(i32, u32, &str, bool); 10] = [
        (0, 'a' as u32, "a", false),
        (1, 0xE1, "\u{e1}", false),
        (1, 0xF1, "\u{f1}", false),
        (1, 0xFC, "\u{fc}", false),
        (1, 0x153, "\u{153}", false),
        (1, 0xBF, "\u{bf}", false),
        (1, 0xB7, "\u{b7}", false),
        (1, KB_KEY_PAGE, "abc", true),
        (0, KB_KEY_SHIFT, "shift", true),
        (0, KB_KEY_PAGE, "\u{e1}\u{e9}", true),
    ];
    for (theme, th) in [UiTheme::Gothic, UiTheme::Classic].into_iter().enumerate() {
        ui_set_theme(th);
        let mut kb = Keyboard::default();
        open_kb(&mut kb, b"");
        for (page, code, label, word) in K {
            if kb.page != page {
                press(&mut kb, KB_KEY_PAGE);
            }
            let mut r = Rect::default();
            let mut want = Canvas::default();
            if !keyboard_key_rect(&kb, code, &mut r) || canvas_init(&mut want, r.w, r.h) != 0 {
                check!(false); // "key"
                continue;
            }
            canvas_fill(&mut want, Rect { x: 0, y: 0, w: r.w, h: r.h }, 255);
            font_draw_centered(&mut want, FontFace::Bold, if word { ui_text_size() } else { ui_title_size() }, Rect { x: 0, y: 0, w: r.w, h: r.h }, label, 0);
            let mut differ = 0;
            G_C.with(|c| {
                let c = c.borrow();
                for y in 20..r.h - 20 {
                    // clear of the frame
                    for x in 20..r.w - 20 {
                        differ += (c.px[((r.y + y) * c.stride + r.x + x) as usize] != want.px[(y * want.stride + x) as usize]) as i32;
                    }
                }
            });
            if differ != 0 {
                eprintln!("  key \"{}\" (theme {}): {} pixels off", label, theme, differ);
            }
            check!(differ == 0);
            canvas_free(&mut want);
        }
    }
    ui_set_theme(UiTheme::Gothic);
}

// What the keyboard types is what the Library keeps. Rename hands the keyboard's text to
// store_rename (kb_finished in library.c), and the mirror PDF takes the name and the folder,
// accents and all, as its path (export_mirror_path; mirror_follow moves the PDF there). Renaming
// again opens the keyboard on the stored name, whole.
fn typed_name_is_stored_whole() {
    let root = format!("{}/keyboard-{}", tl_tmp(), std::process::id());
    let mirror = format!("{}/PDFs", root);
    let _ = std::fs::remove_dir_all(&root);
    let mut nb = NotebookInfo::default();
    let mut back = NotebookInfo::default();
    if store_init(&root) != 0 || store_create("Notebook Oct 19", "", PAPER_MEDIUM, &mut nb) != 0 {
        check!(false); // "store"
        return;
    }
    const NAME: &str = "M\u{fa}sica espa\u{f1}ola \u{a1}Ol\u{e9}";
    const FOLDER: &str = "Matem\u{e1}ticas";
    let mut kb = Keyboard::default();
    open_kb(&mut kb, nb.name.as_bytes());
    for _ in 0..40 {
        if kb.text.is_empty() {
            break;
        }
        press(&mut kb, KB_KEY_BKSP);
    }
    check!(type_text(&mut kb, NAME) && text(&kb) == NAME.as_bytes());
    check!(store_rename(nb.id.as_str(), kb.text.as_str()) == 0 && store_load_info(nb.id.as_str(), &mut back) == 0 && back.name.as_bytes() == NAME.as_bytes());
    open_kb(&mut kb, b"");
    check!(type_text(&mut kb, FOLDER) && text(&kb) == FOLDER.as_bytes());
    check!(store_set_folder(nb.id.as_str(), kb.text.as_str()) == 0 && store_load_info(nb.id.as_str(), &mut back) == 0);
    let mut path = String::new();
    export_mirror_path(&back, &mirror, &mut path, 1400);
    let mut want = String::new();
    csnprintf!(&mut want, 1400, "%s/%s/%s.pdf", &mirror, FOLDER, NAME);
    if path != want {
        eprintln!("  mirror PDF at \"{}\"", path);
    }
    check!(path == want);
    check!(export_mirror_pdf(&back, 300.0, &mirror) == 0 && std::path::Path::new(&want).exists());
    open_kb(&mut kb, back.name.as_bytes());
    check!(text(&kb) == NAME.as_bytes());
    let _ = std::fs::remove_dir_all(&root);
}

fn setup() {
    ui_init(300.0);
    if font_init() != 0 {
        panic!("font_init failed");
    }
    G_C.with(|c| {
        if canvas_init(&mut c.borrow_mut(), K_SCREEN.w, K_SCREEN.h) != 0 {
            panic!("canvas_init failed");
        }
    });
    ui_set_theme(UiTheme::Gothic);
}

#[test]
fn test_keyboard() {
    setup();
    run!(types_musica_through_the_accents_page);
    run!(shift_gives_accented_capitals);
    run!(upper_table);
    run!(open_keeps_accents);
    run!(never_cut_inside_a_character);
    run!(long_name_tail_is_whole_characters);
    run!(letters_layout_kept);
    run!(spanish_letters_have_enye);
    run!(accents_in_every_face);
    run!(one_character_keys_at_title_size);
    done();
}

// The C's last test; it needs the store (store.rs, wave 2), so it runs on its own once that module
// is translated: cargo test --test test_keyboard -- --ignored
#[test]
#[ignore = "needs store.rs (wave 2): store_init, store_create, store_rename, store_set_folder, store_load_info"]
fn test_keyboard_typed_name_is_stored_whole() {
    setup();
    run!(typed_name_is_stored_whole);
    done();
}
