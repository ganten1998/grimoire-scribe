// test_i18n.c: the languages. Lookups, contexts and plurals in the embedded catalogs; every
// translation checked against its English (the same printf arguments, glyphs every face has);
// the choice of language (settings.ini, the Kindle's locale file, the environment); dates; and
// the widths of text the every-screen scenario (tests/scenarios/y_all_screens.qs, which make
// l10n-check runs in every language) cannot reach: rare notices, and text drawn without a fit.
#![allow(clippy::needless_range_loop, clippy::manual_range_contains, clippy::neg_cmp_op_on_partial_ord, clippy::int_plus_one)]
mod testlib;

use grimoire::prelude::*;
use std::cell::RefCell;
use testlib::*;

const THEMES: [UiTheme; 2] = [UiTheme::Gothic, UiTheme::Classic];

// ---- printf formats ----

// The arguments a printf format reads, by position (1-based): 'i' an int (d i u o x X c, and
// the shorter ones it promotes), 'l' long, 'q' long long, 'z' size_t, 'j' intmax_t, 't' ptrdiff_t,
// 'f' double, 'F' long double, 's' a string, 'p' a pointer. Returns how many, or -1 for a format
// printf cannot use safely: %n, a broken directive, numbered and unnumbered ones mixed, the same
// argument read as two types, or a gap among numbered ones (musl's printf then prints nothing).
fn fmt_args(f: &str, types: &mut [u8; 17]) -> i32 {
    *types = [0; 17];
    let b = f.as_bytes();
    // (the C reads a string, so past its end is the NUL)
    let at = |i: usize| -> u8 { b.get(i).copied().unwrap_or(0) };
    let is_digit = |c: u8| c.is_ascii_digit();
    let mut seq: usize = 0;
    let mut max: usize = 0;
    let (mut numbered, mut plain) = (false, false);
    let mut p: usize = 0;
    while at(p) != 0 {
        if at(p) != b'%' {
            p += 1;
            continue;
        }
        p += 1;
        if at(p) == b'%' {
            p += 1;
            continue;
        }
        let mut num: usize = 0;
        let mut q = p;
        while is_digit(at(q)) {
            num = num * 10 + (at(q) - b'0') as usize;
            q += 1;
        }
        let mut pos: usize = 0;
        if at(q) == b'$' && num > 0 {
            pos = num;
            p = q + 1;
            numbered = true;
        } else {
            plain = true;
        }
        while at(p) != 0 && b"-+ #0'".contains(&at(p)) {
            p += 1;
        }
        for part in 0..2 {
            // the width, then the precision
            if part == 1 {
                if at(p) != b'.' {
                    break;
                }
                p += 1;
            }
            if at(p) == b'*' {
                // an int argument holds it
                p += 1;
                let mut sn: usize = 0;
                let mut r = p;
                while is_digit(at(r)) {
                    sn = sn * 10 + (at(r) - b'0') as usize;
                    r += 1;
                }
                let star_at = if at(r) == b'$' && sn > 0 {
                    sn
                } else {
                    seq += 1;
                    seq
                };
                if at(r) == b'$' && sn > 0 {
                    p = r + 1;
                }
                if star_at > 16 || (types[star_at] != 0 && types[star_at] != b'i') {
                    return -1;
                }
                types[star_at] = b'i';
                if star_at > max {
                    max = star_at;
                }
            } else {
                while is_digit(at(p)) {
                    p += 1;
                }
            }
        }
        let mut len: u8 = 0;
        if at(p) == b'h' {
            p += if at(p + 1) == b'h' { 2 } else { 1 };
        } else if at(p) == b'l' {
            len = if at(p + 1) == b'l' { b'q' } else { b'l' };
            p += if at(p + 1) == b'l' { 2 } else { 1 };
        } else if at(p) != 0 && b"zjtL".contains(&at(p)) {
            len = at(p);
            p += 1;
        }
        let t: u8;
        if at(p) != 0 && b"diuoxXc".contains(&at(p)) {
            t = if len != 0 && len != b'L' { len } else { b'i' };
        } else if at(p) != 0 && b"fFeEgGaA".contains(&at(p)) {
            t = if len == b'L' { b'F' } else { b'f' };
        } else if at(p) == b's' {
            t = b's';
        } else if at(p) == b'p' {
            t = b'p';
        } else {
            return -1;
        }
        let arg_at = if pos != 0 {
            pos
        } else {
            seq += 1;
            seq
        };
        if arg_at > 16 || (types[arg_at] != 0 && types[arg_at] != t) {
            return -1;
        }
        types[arg_at] = t;
        if arg_at > max {
            max = arg_at;
        }
        p += 1; // (the for loop's p++)
    }
    if numbered && plain {
        return -1;
    }
    if types[1..=max].contains(&0) {
        return -1;
    }
    max as i32
}

fn format_reader() {
    let mut t = [0u8; 17];
    check!(fmt_args("Saved", &mut t) == 0);
    check!(fmt_args("%d pages, %s", &mut t) == 2 && t[1] == b'i' && t[2] == b's');
    check!(fmt_args("%2$d %1$s, %3$02d:%4$02d", &mut t) == 4 && t[1] == b's' && t[2] == b'i' && t[4] == b'i');
    check!(fmt_args("%d%%   tap to reset", &mut t) == 1 && t[1] == b'i');
    check!(fmt_args("+%.0f%%", &mut t) == 1 && t[1] == b'f');
    check!(fmt_args("%.*s and %zu, %lld", &mut t) == 4 && t[1] == b'i' && t[2] == b's' && t[3] == b'z' && t[4] == b'q');
    check!(fmt_args("%2$s", &mut t) == -1); // argument 1 never read
    check!(fmt_args("%1$d %s", &mut t) == -1); // numbered and not
    check!(fmt_args("%n", &mut t) == -1);
    check!(fmt_args("100%", &mut t) == -1); // a lone percent sign
    check!(fmt_args("%1$d %1$s", &mut t) == -1); // one argument, two types
}

// Whether a translation reads exactly the arguments its English does, with the same types.
fn fmt_compatible(english: &str, translated: &str) -> bool {
    let (mut a, mut b) = ([0u8; 17], [0u8; 17]);
    let na = fmt_args(english, &mut a);
    let nb = fmt_args(translated, &mut b);
    na >= 0 && nb == na && a == b
}

fn format_compatibility() {
    check!(fmt_compatible("%s %d, %02d:%02d", "%2$d %1$s, %3$02d:%4$02d"));
    check!(fmt_compatible("%d pages", "%d p\u{e1}ginas"));
    check!(!fmt_compatible("%d pages", "%s p\u{e1}ginas"));
    check!(!fmt_compatible("%d pages, %s", "%d p\u{e1}ginas")); // drops an argument
    check!(!fmt_compatible("Saved", "Guardado %s")); // reads one the code never passes
    check!(!fmt_compatible("%s %d", "%2$d %1$d"));
}

// ---- the catalogs ----

// The strings of an entry's translation: its nforms forms, each ending in its NUL (the entry's str
// holds them as the catalog does).
fn forms_of(e: &I18nEntry) -> Vec<&'static str> {
    e.str.split('\0').take(e.nforms as usize).collect()
}

fn each_entry_formats(code: &str, e: &I18nEntry, bad: &mut i32) {
    for (k, s) in forms_of(e).into_iter().enumerate() {
        let english = match (k, e.id_plural) {
            (0, _) | (_, None) => e.id,
            (_, Some(plural)) => plural,
        };
        if !fmt_compatible(english, s) {
            eprintln!("  {}: \"{}\" does not read the arguments of \"{}\"", code, s, english);
            *bad += 1;
        }
    }
    if let Some(plural) = e.id_plural {
        if !fmt_compatible(e.id, plural) {
            eprintln!("  {}: \"{}\" and its plural \"{}\" read different arguments", code, e.id, plural);
            *bad += 1;
        }
    }
}

// The next code point of valid UTF-8 (overlong forms, surrogates and stray bytes are not); 0 at
// the end, -1 for broken text.
fn next_cp(s: &mut &[u8]) -> i64 {
    let Some(&p0) = s.first() else { return 0 };
    if p0 == 0 {
        return 0;
    }
    let len: usize = if p0 < 0x80 {
        1
    } else if (0xC2..0xE0).contains(&p0) {
        2
    } else if (0xE0..0xF0).contains(&p0) {
        3
    } else if (0xF0..0xF5).contains(&p0) {
        4
    } else {
        0
    };
    if len == 0 {
        return -1;
    }
    let mut c: i64 = if len == 1 { p0 as i64 } else { (p0 & (0x7F >> len)) as i64 };
    for i in 1..len {
        // (a missing byte is the string's NUL, which is no continuation)
        let pi = s.get(i).copied().unwrap_or(0);
        if (pi & 0xC0) != 0x80 {
            return -1;
        }
        c = c << 6 | (pi & 0x3F) as i64;
    }
    const LEAST: [i64; 5] = [0, 0, 0x80, 0x800, 0x10000];
    if c < LEAST[len] || (0xD800..=0xDFFF).contains(&c) || c > 0x10FFFF {
        return -1;
    }
    *s = &s[len..];
    c
}

// Every face text can be drawn in: Gothic's Jersey 10 (text) and Jacquard 24 (titles), Classic's
// Terminess and its bold. A glyph a pixel face lacks would come from Terminess, smaller and in
// another style: a translation must not need that.
fn missing_glyphs(s: &str) -> i32 {
    let mut missing = 0;
    let mut s = s.as_bytes();
    loop {
        let cp = next_cp(&mut s);
        if cp == 0 {
            return missing;
        }
        if cp < 0 || cp < 0x20 {
            return missing + 1000; // broken UTF-8, or a control character
        }
        ui_set_theme(UiTheme::Gothic);
        let mut ok = font_has_glyph(FontFace::Regular, cp as u32) && font_has_glyph(FontFace::Title, cp as u32);
        ui_set_theme(UiTheme::Classic);
        ok = ok && font_has_glyph(FontFace::Regular, cp as u32) && font_has_glyph(FontFace::Bold, cp as u32);
        ui_set_theme(UiTheme::Gothic);
        if !ok {
            eprintln!("  U+{:04X} is missing from a face", cp);
            missing += 1;
        }
    }
}

fn each_entry_glyphs(code: &str, e: &I18nEntry, bad: &mut i32) {
    for s in forms_of(e) {
        if missing_glyphs(s) != 0 {
            eprintln!("  {}: \"{}\" cannot be drawn in every face", code, s);
            *bad += 1;
        }
    }
}

fn count_entry(code: &str, e: &I18nEntry, forms: i32, bad: &mut i32) {
    if e.id_plural.is_some() && e.nforms != forms {
        eprintln!("  {}: \"{}\" has {} plural forms, the language {}", code, e.id, e.nforms, forms);
        *bad += 1;
    }
}

// The Plural-Forms line each language's catalog must carry: src/i18n.c has the same rule by hand.
const K_RULES: [(&str, &str); 1] = [("es", "Plural-Forms: nplurals=2; plural=(n != 1);")];

fn catalogs_hold_together() {
    // Every catalog is for a language of the list (English needs none), and reads.
    let mut ncat = 0;
    for k in i18n_catalogs.iter() {
        check!(i18n_find(k.code) > 0 && i18n_language_code(i18n_find(k.code)) == Some(k.code));
        check!(i18n_mo_valid(k.mo));
        ncat += 1;
    }
    check!(ncat == i18n_language_count() - 1);
    check!(i18n_language_code(0) == Some("en") && i18n_language_code(i18n_language_count()).is_none());
    for i in 1..i18n_language_count() {
        let code = i18n_language_code(i).unwrap();
        let mut bad = 0;
        let n = i18n_walk(code, &mut |e| each_entry_formats(code, e, &mut bad));
        check!(n > 0);
        i18n_walk(code, &mut |e| each_entry_glyphs(code, e, &mut bad));
        let forms = i18n_plural_forms(code);
        i18n_walk(code, &mut |e| count_entry(code, e, forms, &mut bad));
        check!(bad == 0);
        // Its plural rule is the one written in src/i18n.c.
        let mut rule: Option<&str> = None;
        for (rc, plural_forms) in K_RULES {
            if rc == code {
                rule = Some(plural_forms);
            }
        }
        check!(rule.is_some_and(|r| i18n_header(code).contains(r)));
        if rule.is_none() {
            eprintln!("  {}: add its Plural-Forms to k_rules here, and its rule to src/i18n.c", code);
        }
        // The language's own name is in every face too.
        check!(missing_glyphs(i18n_language_name(i).unwrap_or("")) == 0);
    }
}

fn lookups() {
    check!(i18n_set("en") && i18n_current() == "en");
    let lang = "Language";
    check!(std::ptr::eq(tr("Language"), lang) || tr("Language") == "Language");
    check!(i18n_set("es") && i18n_current() == "es");
    check!(tr("Language") == "Idioma");
    check!(tr("Language: %s") == "Idioma: %s");
    // English for what a catalog lacks: the very string the code passed.
    let none = "A sentence no catalog has";
    check!(std::ptr::eq(tr(none), none));
    check!(tr("").is_empty()); // never the catalog's header
    // Contexts: the same English in two contexts are two strings; a context it lacks is English.
    check!(c_("month", "May") == "may");
    check!(c_("month", "Sep") == "sept");
    check!(c_("nonesuch", "May") == "May");
    check!(tr("May") == "May");
    check!(c_("date", "%s %d") == "%2$d %1$s");
    // A locale name picks its language by the language part; one it lacks is English.
    check!(i18n_set("es_MX.UTF-8") && i18n_current() == "es");
    check!(!i18n_set("fr_FR.UTF-8") && i18n_current() == "en" && tr("Language") == "Language");
    i18n_set("en");
}

fn plurals() {
    i18n_set("es");
    // Spanish: 1 the one form, 0 and 2 (and every other count) the other.
    check!(ng_("%d notebook", "%d notebooks", 0) == "%d cuadernos");
    check!(ng_("%d notebook", "%d notebooks", 1) == "%d cuaderno");
    check!(ng_("%d notebook", "%d notebooks", 2) == "%d cuadernos");
    check!(ng_("%d page, %s", "%d pages, %s", 21) == "%d p\u{e1}ginas, %s");
    // Missing: the English form for the count.
    check!(ng_("%d frog", "%d frogs", 1) == "%d frog" && ng_("%d frog", "%d frogs", 3) == "%d frogs");
    i18n_set("en");
    check!(ng_("%d notebook", "%d notebooks", 1) == "%d notebook");
    check!(ng_("%d notebook", "%d notebooks", 0) == "%d notebooks");
    check!(i18n_plural_forms("en") == 2 && i18n_plural_forms("es") == 2 && i18n_plural_forms("xx") == 0);
}

fn rd32(m: &[u8], at: usize) -> u32 {
    u32::from_ne_bytes(m[at..at + 4].try_into().unwrap())
}
fn wr32(m: &mut [u8], at: usize, v: u32) {
    m[at..at + 4].copy_from_slice(&v.to_ne_bytes());
}

// A catalog that does not read is refused whole (its language is then English), whatever is wrong.
fn damaged_catalogs() {
    let es = i18n_catalogs.iter().find(|k| k.code == "es");
    check!(es.is_some());
    let Some(es) = es else { return };
    let n = es.mo.len();
    let mut m: Vec<u8> = es.mo.to_vec();
    check!(i18n_mo_valid(&m));
    check!(!i18n_mo_valid(&m[..27])); // shorter than the header
    check!(!i18n_mo_valid(&m[..n - 1])); // the last string runs off the end
    check!(!i18n_mo_valid(&[]));
    let count = rd32(&m, 8);
    let orig = rd32(&m, 12);
    let trans = rd32(&m, 16);
    m[0] ^= 1; // the magic
    check!(!i18n_mo_valid(&m));
    m[0] ^= 1;
    wr32(&mut m, 12, 0x7FFFFFF0); // a table past the end
    check!(!i18n_mo_valid(&m));
    wr32(&mut m, 12, orig);
    wr32(&mut m, 8, count.wrapping_add(1000000)); // more strings than the file holds
    check!(!i18n_mo_valid(&m));
    wr32(&mut m, 8, count);
    // Two English strings swapped: out of order, so a search could miss: refused.
    let o = orig as usize;
    let mut e1 = [0u8; 8];
    let mut e2 = [0u8; 8];
    e1.copy_from_slice(&m[o + 8..o + 16]);
    e2.copy_from_slice(&m[o + 16..o + 24]);
    m[o + 8..o + 16].copy_from_slice(&e2);
    m[o + 16..o + 24].copy_from_slice(&e1);
    check!(!i18n_mo_valid(&m));
    m[o + 8..o + 16].copy_from_slice(&e1);
    m[o + 16..o + 24].copy_from_slice(&e2);
    check!(i18n_mo_valid(&m));
    // A string without its terminating NUL.
    let t = trans as usize;
    let len0 = rd32(&m, t + 8) as usize;
    let off0 = rd32(&m, t + 12) as usize;
    m[off0 + len0] = b'x';
    check!(!i18n_mo_valid(&m));
    m[off0 + len0] = 0;
    // The same catalog written big-endian reads too (msgfmt on such a machine).
    let mut be: Vec<u8> = m.clone();
    let swap = |be: &mut Vec<u8>, from: &Vec<u8>, at: usize| {
        let t = [from[at + 3], from[at + 2], from[at + 1], from[at]];
        be[at..at + 4].copy_from_slice(&t);
    };
    let mut at = 0;
    while at < 28 {
        let src = be.clone();
        swap(&mut be, &src, at);
        at += 4;
    }
    let mut at = o;
    while at < o + count as usize * 8 {
        swap(&mut be, &m, at);
        at += 4;
    }
    let mut at = t;
    while at < t + count as usize * 8 {
        swap(&mut be, &m, at);
        at += 4;
    }
    check!(i18n_mo_valid(&be));
}

// A context with plurals (httpd.c sends the studio its words so): the form for the count, English
// for a language without the string.
fn context_plurals() {
    i18n_set("es");
    let one = "PNGs, all %d page (zip)";
    let many = "PNGs, all %d pages (zip)";
    check!(i18n_getnc("studio", one, many, 1) == "PNG de %d p\u{e1}gina (zip)");
    check!(i18n_getnc("studio", one, many, 12) == "PNG de las %d p\u{e1}ginas (zip)");
    // (The C passes no context here, NULL; i18n_getn is that lookup.)
    check!(std::ptr::eq(i18n_getn(one, many, 12), many)); // no such string outside its context
    check!(std::ptr::eq(i18n_getnc("nonesuch", one, many, 1), one));
    i18n_set("en");
    check!(std::ptr::eq(i18n_getnc("studio", one, many, 1), one) && std::ptr::eq(i18n_getnc("studio", one, many, 2), many));
}

// The names of papers, tools and music parts are identifiers (/api/list.json, grimoire.log): English
// in every language. Their labels, which the screens show, are in the language in use, and every
// language has each of them.
fn labels_and_identifiers() {
    for l in 0..i18n_language_count() {
        i18n_set(i18n_language_code(l).unwrap());
        let en = l == 0;
        // (the C compares the pointers: a label is the very name when it is not translated)
        let same = |a: &str, b: &str| std::ptr::eq(a, b);
        check!(paper_name(PAPER_NARROW) == "Narrow rule" && tool_name(TOOL_PENCIL) == "Pencil");
        check!(music_part_name(PART_TREBLE) == "Treble staff");
        for i in 0..PAPER_COUNT {
            let k = paper_at(i);
            check!(if en { same(paper_label(k), paper_name(k)) } else { !same(paper_label(k), paper_name(k)) });
            check!(missing_glyphs(paper_label(k)) == 0);
        }
        let custom = music_default();
        let newer = PaperKind(0x50000001);
        check!(if en { same(paper_label(custom), paper_name(custom)) } else { !same(paper_label(custom), paper_name(custom)) });
        check!(if en { same(paper_label(newer), paper_name(newer)) } else { !same(paper_label(newer), paper_name(newer)) });
        for t in 0..TOOL_COUNT {
            let t = ToolKind(t as i32);
            check!(if en { same(tool_label(t), tool_name(t)) } else { !same(tool_label(t), tool_name(t)) });
        }
        for p in PART_TREBLE.0..PART_COUNT {
            let p = MusicPart(p);
            check!(if en { same(part_label(p), music_part_name(p)) } else { !same(part_label(p), music_part_name(p)) });
        }
        check!(part_label(PART_NONE).is_empty());
    }
    i18n_set("es");
    check!(paper_label(PAPER_NARROW) == "Rayado fino" && tool_label(TOOL_PENCIL) == "Grafito");
    check!(part_label(PART_TREBLE) == "Clave de sol");
    i18n_set("en");
}

// ---- choosing the language ----

fn clear_env() {
    std::env::remove_var("GRIMOIRE_LANG");
    std::env::remove_var("LC_ALL");
    std::env::remove_var("LC_MESSAGES");
    std::env::remove_var("LANG");
}

fn locale_names() {
    check!(i18n_find("es") == 1 && i18n_find("ES") == 1 && i18n_find("es-419") == 1);
    check!(i18n_find("es_MX.utf8") == 1 && i18n_find("es_ES.UTF-8") == 1 && i18n_find("es@euro") == 1);
    check!(i18n_find("en_US.UTF-8") == 0 && i18n_find("en") == 0);
    check!(i18n_find("") == -1 && i18n_find("C") == -1 && i18n_find("POSIX") == -1 && i18n_find("C.UTF-8") == -1);
    // (The C also asks about NULL; a &str has no such value.)
    check!(i18n_find("fr_FR") == -1 && i18n_find("esperanto") == -1 && i18n_find("e") == -1);
    let mut v = String::new();
    i18n_locale_file_lang("LANG=es_ES.UTF-8\nLC_ALL=es_ES.UTF-8\n", &mut v, 32);
    check!(v == "es_ES.UTF-8");
    i18n_locale_file_lang("LC_ALL=ru_RU.utf8\r\nLANG=ru_RU.utf8\r\n", &mut v, 32);
    check!(v == "ru_RU.utf8");
    i18n_locale_file_lang("# langpicker\nexport LANG=\"es_MX.utf8\"\n", &mut v, 32);
    check!(v == "es_MX.utf8");
    i18n_locale_file_lang("LC_ALL=es_AR.UTF-8\n", &mut v, 32); // no LANG= line: LC_ALL=
    check!(v == "es_AR.UTF-8");
    i18n_locale_file_lang("LANGUAGE=es\nLANG=\n", &mut v, 32);
    check!(v.is_empty());
    // (NULL in the C: no file text; an empty one is what a &str can say.)
    i18n_locale_file_lang("", &mut v, 32);
    check!(v.is_empty());
}

fn detection_order() {
    let mut why = String::new();
    clear_env();
    check!(i18n_detect_from(None, &mut why, 128) == 0 && why.contains("no language"));
    std::env::set_var("LANG", "es_ES.UTF-8");
    check!(i18n_detect_from(None, &mut why, 128) == 1 && why.contains("LANG=es_ES.UTF-8"));
    std::env::set_var("LC_MESSAGES", "en_GB.UTF-8"); // LC_MESSAGES before LANG
    check!(i18n_detect_from(None, &mut why, 128) == 0);
    std::env::set_var("LC_ALL", "es_MX.UTF-8"); // LC_ALL before both
    check!(i18n_detect_from(None, &mut why, 128) == 1 && why.contains("LC_ALL"));
    // On the Kindle its own Settings' language ranks above what Grimoire inherits: KOReader's
    // launcher exports LC_ALL=en_US.UTF-8 to everything it starts.
    std::env::set_var("LC_ALL", "en_US.UTF-8");
    check!(i18n_detect_from(Some("LANG=es_ES.UTF-8\nLC_ALL=es_ES.UTF-8\n"), &mut why, 128) == 1);
    check!(why.contains("Kindle's locale es_ES.UTF-8"));
    // GRIMOIRE_LANG ranks first, even there.
    std::env::set_var("GRIMOIRE_LANG", "en");
    check!(i18n_detect_from(Some("LANG=es_ES.UTF-8\n"), &mut why, 128) == 0 && why.contains("GRIMOIRE_LANG=en"));
    std::env::set_var("GRIMOIRE_LANG", ""); // set but empty: not set
    check!(i18n_detect_from(Some("LANG=es_ES.UTF-8\n"), &mut why, 128) == 1);
    // The first one set decides, also for a language Grimoire does not speak: English, said so.
    clear_env();
    std::env::set_var("LANG", "es_ES.UTF-8");
    check!(i18n_detect_from(Some("LANG=ru_RU.utf8\n"), &mut why, 128) == 0 && why.contains("not a language"));
    std::env::set_var("LANG", "C");
    check!(i18n_detect_from(None, &mut why, 128) == 0);
    // An empty or unreadable locale file counts as none.
    std::env::set_var("LANG", "es_CO.UTF-8");
    check!(i18n_detect_from(Some(""), &mut why, 128) == 1);
    // The log line names no more than the locale: printable ASCII, cut short.
    std::env::set_var("LANG", "es_ES\"\u{1}\u{f1}xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx");
    check!(i18n_detect_from(None, &mut why, 128) == 1 && !why.contains('"') && !why.contains('\x01') && why.len() < 80);
    clear_env();
}

fn applying_a_setting() {
    clear_env();
    std::env::set_var("GRIMOIRE_LANG", "es");
    i18n_detect();
    check!(i18n_auto() == "es");
    i18n_apply("auto");
    check!(i18n_current() == "es");
    i18n_apply("en");
    check!(i18n_current() == "en");
    i18n_apply("fr"); // a language a newer version speaks: auto
    check!(i18n_current() == "es");
    i18n_apply("");
    check!(i18n_current() == "es");
    check!(i18n_setting_known("auto") && i18n_setting_known("en") && i18n_setting_known("es"));
    // (The C also asks about NULL; a &str has no such value.)
    check!(!i18n_setting_known("fr") && !i18n_setting_known("es_ES") && !i18n_setting_known(""));
    clear_env();
    i18n_detect();
    check!(i18n_auto() == "en");
    i18n_apply("auto");
}

fn file_text(path: &str) -> Option<String> {
    read_file(path).map(|b| String::from_utf8_lossy(&b).into_owned())
}
fn has_line(text: &str, line: &str) -> bool {
    text.split('\n').any(|l| l == line)
}

// settings.ini language=: auto by default; a language this version lacks acts as auto and goes
// back into the file as it was, until the language is picked; settings_language reads it alone.
fn language_setting() {
    let mut path = String::new();
    csnprintf!(&mut path, 700, "%s/settings.ini", &store_root());
    let mut v = String::new();
    let _ = std::fs::remove_file(&path);
    let mut s = Settings::default();
    settings_load(&mut s);
    check!(s.language == "auto");
    settings_language(&mut v, 16);
    check!(v.is_empty());
    check!(settings_save(&s) == 0);
    let mut t = file_text(&path);
    check!(t.as_deref().is_some_and(|t| has_line(t, "language=auto")));
    let f = "theme=classic\nlanguage=es\n";
    check!(write_file_atomic(&path, f.as_bytes()) == 0);
    check!(settings_load(&mut s) == 0 && s.language == "es");
    settings_language(&mut v, 16);
    check!(v == "es");
    let f = "language=fr\r\ntheme=gothic\n";
    check!(write_file_atomic(&path, f.as_bytes()) == 0);
    check!(settings_load(&mut s) == 0 && s.language == "auto");
    settings_language(&mut v, 16);
    check!(v == "fr");
    for _round in 0..2 {
        // kept on every save while unchanged
        check!(settings_save(&s) == 0);
        t = file_text(&path);
        check!(t.as_deref().is_some_and(|t| has_line(t, "language=fr") && !t.contains("language=auto")));
        check!(settings_load(&mut s) == 0);
    }
    s.language.set("es"); // picked in Settings
    check!(settings_save(&s) == 0);
    s.language.set("auto"); // and then Automatic: saved as set
    check!(settings_save(&s) == 0);
    t = file_text(&path);
    check!(t.as_deref().is_some_and(|t| has_line(t, "language=auto") && !t.contains("language=fr")));
    // The last line counts.
    let f = "language=fr\nlanguage=en\n";
    check!(write_file_atomic(&path, f.as_bytes()) == 0);
    check!(settings_load(&mut s) == 0 && s.language == "en" && settings_save(&s) == 0);
    t = file_text(&path);
    check!(t.as_deref().is_some_and(|t| has_line(t, "language=en") && !t.contains("fr")));
    settings_language(&mut v, 16);
    check!(v == "en");
    let _ = std::fs::remove_file(&path);
}

// ---- dates ----

extern "C" {
    // (the libc crate does not declare it on every target)
    fn tzset();
}

fn dates() {
    std::env::set_var("TZ", "UTC");
    // SAFETY: tzset takes no arguments and only rereads TZ.
    unsafe { tzset() };
    let t: i64 = 1760106720; // 2025-10-10 14:32 UTC
    let mut b = String::new();
    i18n_set("en");
    format_time_short(t, &mut b, 64);
    check!(b == "Oct 10, 14:32");
    format_date_short(t, &mut b, 64);
    check!(b == "Oct 10");
    i18n_set("es");
    format_time_short(t, &mut b, 64);
    check!(b == "10 oct, 14:32");
    format_date_short(t, &mut b, 64);
    check!(b == "10 oct");
    format_time_short(t - 30 * 86400 + 7 * 3600, &mut b, 64); // 2025-09-10 21:32: the long month
    check!(b == "10 sept, 21:32");
    format_time_short(1736208000, &mut b, 64); // 2025-01-07 00:00
    check!(b == "7 ene, 00:00");
    i18n_set("en");
    std::env::remove_var("TZ");
    // SAFETY: as above.
    unsafe { tzset() };
}

// ---- text that must fit ----

// Notices no scenario shows (a full store, a damaged page, a missing pen, the passcode left off),
// in every language and style, on the narrow (upright) screen: a toast has two lines. Each is
// formatted as the code formats it (keep this list in step with the messages); out holds them.
const RARE_MAX: usize = 40;
const RARE_LEN: usize = 256;
fn rare_notices(out: &mut Vec<String>) -> usize {
    out.clear();
    let mut put = |s: String| {
        // (snprintf into a RARE_LEN buffer)
        let mut t = String::new();
        strlcpy_q(&mut t, &s, RARE_LEN);
        out.push(t);
    };
    put(cformat!("%s", tr("Your Kindle passcode was left off. Turn it back on in Kindle Settings.")));
    put(cformat!(tr("No pen found. %s is made for the Kindle Scribe; writing needs its pen."), APP_NAME));
    put(cformat!(tr("Your Kindle passcode is paused while %s is open."), APP_NAME));
    put(cformat!("%s", tr("Could not save this page. Hold power 4 s again to leave anyway.")));
    put(cformat!(tr("Saved. Leaving %s."), APP_NAME));
    put(cformat!(tr("Leaving %s without this page."), APP_NAME));
    put(cformat!(tr("PDF saved to %s"), "grimoire/PDFs"));
    put(cformat!("%s", tr("Could not save the PDF")));
    put(cformat!(tr("A damaged page was set aside; a blank one took its place (see %s)"), "grimoire.log"));
    put(cformat!("%s", tr("Could not create the notebook (storage full?)")));
    put(cformat!("%s", tr("Could not save this page. Is the storage full?")));
    put(cformat!("%s", tr("This page was made by a newer Grimoire. It is read-only here.")));
    put(cformat!("%s", tr("Could not open that page")));
    put(cformat!("%s", tr("Could not open that notebook")));
    put(cformat!("%s", tr("This page is already empty")));
    put(cformat!("%s", tr("Could not delete those pages")));
    put(cformat!("%s", tr("Could not add a page")));
    put(cformat!("%s", tr("Could not copy that page")));
    put(cformat!(tr("This page is a copy of page %d."), 2000));
    put(cformat!("%s", tr("Could not save the sleep screen")));
    put(cformat!("%s", tr("Calibration saved")));
    put(cformat!(tr("Moved to %s"), "grimoire/trash"));
    put(cformat!(ng_("Deleted %d page (kept in %s)", "Deleted %d pages (kept in %s)", 1), 1, "grimoire/trash"));
    put(cformat!(ng_("Deleted %d page (kept in %s)", "Deleted %d pages (kept in %s)", 2000), 2000, "grimoire/trash"));
    put(cformat!(tr("Eraser: %s"), c_("eraser", "Whole strokes"))); // (a controller's Eraser mode)
    // The Buttons sheet's learn that takes a button from another action, with the longest names.
    put(cformat!(tr("That button is no longer for %s."), c_("tool", "Mechanical pencil")));
    put(cformat!(tr("That button is no longer for %s."), tr("Erase while held")));
    put(cformat!(tr("That button is no longer for %s."), tr("Tool picker")));
    put(cformat!(tr("That button is no longer for %s."), tr("Previous in the picker")));
    // A Buttons row held, with the longest names (a row's and an action's).
    put(cformat!(tr("No buttons for %s now."), tr("Previous / next tool")));
    put(cformat!(tr("No buttons for %s now."), tr("Previous / next in the picker")));
    put(cformat!(tr("No buttons for %s now."), tr("Erase while held")));
    // The gestures' toasts (a page turned past the end, the pen near, the pan hint).
    put(cformat!("%s", tr("This is the last page.")));
    put(cformat!("%s", tr("This is the first page.")));
    put(cformat!("%s", tr("Move the pen away, then swipe.")));
    put(cformat!("%s", tr("Here two fingers move the page. Open Pages and Links from the panel.")));
    out.len()
}

// How many bytes the App's toast holds (the C's sizeof ((App *)0)->toast), from the field's own type.
fn toast_cap() -> usize {
    fn cap<const N: usize>(_: fn(&App) -> &FixedStr<N>) -> usize {
        N
    }
    cap(|a| &a.toast)
}

fn rare_notices_fit() {
    let screen = Rect { x: 0, y: 0, w: 1860, h: 2480 };
    let mut toasts: Vec<String> = Vec::with_capacity(RARE_MAX);
    for l in 0..i18n_language_count() {
        i18n_set(i18n_language_code(l).unwrap());
        let nt = rare_notices(&mut toasts);
        for th in THEMES {
            ui_set_theme(th);
            for s in toasts.iter().take(nt) {
                let lines = font_wrap_lines(FontFace::Bold, ui_text_size(), s, ui_toast_room(screen), None);
                if !(1..=2).contains(&lines) {
                    eprintln!("  {}: \"{}\" needs {} lines", i18n_current(), s, lines);
                }
                check!((1..=2).contains(&lines));
                check!(s.len() < toast_cap()); // whole in the toast's buffer
            }
            // The Display sheet's rows for a Kindle without the light (its whole width).
            let room = 1500 - if th == UiTheme::Gothic { 80 } else { 72 };
            check!(font_measure(FontFace::Regular, ui_text_size(), tr("Front light controls are not available here.")) <= room);
            check!(font_measure(FontFace::Regular, ui_text_size(), tr("This Kindle has no warmth control.")) <= room);
            // The Share screen without an address (the scenario has one), and asleep: lines across
            // the sheet's text room, the first in bold at the title size.
            let share = 1860 - 2 * 70 - if th == UiTheme::Gothic { 80 } else { 72 };
            let heads = [tr("Turn on Wi-Fi on the Kindle first"), tr("Could not start the server"), tr("Asleep")];
            let lines = [tr("Then tap Retry."), tr("Is another app using the port? Try again."), tr("Wake the Kindle to share again.")];
            for i in 0..3 {
                if font_measure(FontFace::Bold, ui_title_size(), heads[i]) > share {
                    eprintln!("  too wide: {}", heads[i]);
                }
                check!(font_measure(FontFace::Bold, ui_title_size(), heads[i]) <= share);
                check!(font_measure(FontFace::Regular, ui_text_size(), lines[i]) <= share);
            }
            // Its Retry button, under no icon (404 px, at 44 px or 34 px).
            check!(font_fit_px(FontFace::Caps, ui_text_size(), ui_small_size(), tr("Retry"), 420 - 16) > 0.0);
            // A second address, on its own line in small type (sideways the column is narrower).
            let mut also = String::new();
            csnprintf!(&mut also, 160, tr("Also: %s (%s)"), "http://192.168.15.244:8080", "usb0");
            check!(font_measure(FontFace::Regular, ui_small_size(), &also) <= 1400);
            // Calibration's notes under what to do (the scenario sees them fit), as calibrate.c
            // formats them: whole in their buffer, which is a toast's size.
            let mut off = String::new();
            csnprintf!(&mut off, RARE_LEN, tr("The center was off by %.1f mm. Let's try again."), 88.8);
            let notes = [tr("Those taps did not make sense. Let's try again."), off.as_str(), tr("Pen done.")];
            for n in notes {
                check!(n.len() < toast_cap());
            }
            // Settings' Data line with the Kindle's own path: the label fits the sheet (the path is
            // a name, which a longer one is cut short as, unreported: tests/l10n/long_root.qs).
            let mut data = String::new();
            csnprintf!(&mut data, 160, tr("Data: %s"), "/mnt/us/grimoire");
            check!(font_measure(FontFace::Regular, ui_small_size(), &data) <= if th == UiTheme::Gothic { 1640 } else { 1648 });
        }
    }
    ui_set_theme(UiTheme::Gothic);
    i18n_set("en");
}

// A translation can be longer than the English a buffer was sized for: i18n_copy cuts it between
// two characters, and takes off one that snprintf already cut.
fn copies_cut_between_characters() {
    let mut b = String::new();
    check!(i18n_copy(&mut b, "a\u{f1}o", 8) == 4 && b == "a\u{f1}o"); // "año", whole
    check!(i18n_copy(&mut b, "a\u{f1}o", 4) == 3 && b == "a\u{f1}");
    check!(i18n_copy(&mut b, "a\u{f1}o", 3) == 1 && b == "a"); // not "a" and half an n-tilde
    check!(i18n_copy(&mut b, "\u{2026}!", 3) == 0 && b.is_empty()); // a three-byte character
    check!(i18n_copy(&mut b, "\u{1f600}!", 5) == 4 && b == "\u{1f600}");
    check!(i18n_copy(&mut b, "\u{1f600}!", 4) == 0 && b.is_empty());
    // (The C also copies "ab\xc3", text snprintf cut inside a character, and takes the broken
    // byte off. A &str cannot hold a cut character: cformat!/csnprintf! never make one.)
    check!(i18n_copy(&mut b, "abcdefghij", 8) == 7 && b == "abcdefg");
    check!(i18n_copy(&mut b, "", 8) == 0 && b.is_empty());
    check!(i18n_copy(&mut b, "x", 1) == 0 && b.is_empty());
    b = "z".to_string();
    check!(i18n_copy(&mut b, "x", 0) == 0 && b == "z"); // no room, not even for the NUL: untouched
}

// Pen feel draws its graph's and pad's labels as they are (font_draw: never cut): under the
// graph and over it, 570 px to the test pad; over the pad, the pad's width (1180 px upright).
fn raw_labels_fit() {
    for l in 0..i18n_language_count() {
        i18n_set(i18n_language_code(l).unwrap());
        for th in THEMES {
            ui_set_theme(th);
            let ss = ui_small_size();
            check!(font_measure(FontFace::Regular, ss, tr("pen pressure")) <= 570);
            check!(font_measure(FontFace::Regular, ss, tr("ink")) <= 570);
            check!(font_measure(FontFace::Regular, ss, tr("test pad")) <= 1180);
            check!(font_measure(FontFace::Regular, ss, tr("test pad (the eraser clears it)")) <= 1180);
            check!(font_measure(FontFace::Regular, ss, tr("test pad (white ink: try it on the black)")) <= 1180);
        }
    }
    ui_set_theme(UiTheme::Gothic);
    i18n_set("en");
}

// Paper never changes with the language: the Cornell headings are part of the page, drawn the
// same for everyone (so are PDFs, thumbnails and grimoire-export's renders).
fn paper_is_the_same_in_every_language() {
    let mut a = Canvas::default();
    let mut b = Canvas::default();
    check!(canvas_init(&mut a, 1860, 2480) == 0 && canvas_init(&mut b, 1860, 2480) == 0);
    i18n_set("en");
    paper_render(&mut a, PAPER_CORNELL, Rect { x: 0, y: 0, w: 1860, h: 2480 }, 300.0);
    for l in 1..i18n_language_count() {
        i18n_set(i18n_language_code(l).unwrap());
        paper_render(&mut b, PAPER_CORNELL, Rect { x: 0, y: 0, w: 1860, h: 2480 }, 300.0);
        let mut same = true;
        let mut y = 0;
        while y < a.h && same {
            let (ra, rb) = ((y * a.stride) as usize, (y * b.stride) as usize);
            same = a.px[ra..ra + a.w as usize] == b.px[rb..rb + a.w as usize];
            y += 1;
        }
        check!(same);
    }
    canvas_free(&mut a);
    canvas_free(&mut b);
    i18n_set("en");
}

// ---- the layout helpers ----

struct Hook {
    calls: i32,
    need: i32,
    have: i32,
    text: String,
}
thread_local! {
    static G_HOOK: RefCell<Hook> = const { RefCell::new(Hook { calls: 0, need: 0, have: 0, text: String::new() }) };
}
fn hook(text: &str, need: i32, have: i32) {
    G_HOOK.with(|h| {
        let mut h = h.borrow_mut();
        h.calls += 1;
        h.need = need;
        h.have = have;
        strlcpy_q(&mut h.text, text, 256);
    });
}
fn hook_calls() -> i32 {
    G_HOOK.with(|h| h.borrow().calls)
}
fn set_hook_calls(n: i32) {
    G_HOOK.with(|h| h.borrow_mut().calls = n);
}

fn same_pixels(a: &Canvas, b: &Canvas) -> bool {
    for y in 0..a.h {
        let (ra, rb) = ((y * a.stride) as usize, (y * b.stride) as usize);
        if a.px[ra..ra + a.w as usize] != b.px[rb..rb + a.w as usize] {
            return false;
        }
    }
    true
}

fn overflow_hook() {
    let mut c = Canvas::default();
    check!(canvas_init(&mut c, 800, 200) == 0);
    font_set_overflow_hook(Some(hook));
    set_hook_calls(0);
    font_draw_left(&mut c, FontFace::Regular, 44.0, Rect { x: 0, y: 0, w: 800, h: 100 }, "Fits", 0);
    font_draw_centered(&mut c, FontFace::Regular, 44.0, Rect { x: 0, y: 0, w: 800, h: 100 }, "Fits", 0);
    check!(hook_calls() == 0);
    let longer = "A sentence much too long for a box two hundred pixels wide";
    font_draw_left(&mut c, FontFace::Regular, 44.0, Rect { x: 0, y: 0, w: 200, h: 100 }, longer, 0);
    check!(G_HOOK.with(|h| {
        let h = h.borrow();
        h.calls == 1 && h.text == longer && h.have == 200
    }));
    check!(G_HOOK.with(|h| h.borrow().need) == font_measure(FontFace::Regular, 44.0, longer));
    font_draw_centered(&mut c, FontFace::Bold, 44.0, Rect { x: 0, y: 0, w: 200, h: 100 }, longer, 0);
    check!(hook_calls() == 2);
    // A name, which "..." shortens by design: not reported.
    font_draw_left_name(&mut c, FontFace::Regular, 44.0, Rect { x: 0, y: 0, w: 200, h: 100 }, longer, 0);
    check!(hook_calls() == 2);
    // Wrapped text that fits its lines is not reported; text that does not is, whole.
    font_draw_wrapped(&mut c, FontFace::Regular, 34.0, Rect { x: 0, y: 0, w: 500, h: 200 }, "Two short lines of text here", 2, 0, true, 0);
    check!(hook_calls() == 2);
    font_draw_wrapped(&mut c, FontFace::Regular, 34.0, Rect { x: 0, y: 0, w: 300, h: 200 }, longer, 2, 0, true, 0);
    check!(G_HOOK.with(|h| {
        let h = h.borrow();
        h.calls == 3 && h.text == longer && h.have == 300
    }));
    // A word wider than its line, broken between two of its letters (with no hyphen): reported,
    // with the word's width, though every line fits.
    let word = "Notenlinienpapier";
    let two = "Notenlinienpapier kariert";
    let ww = font_measure(FontFace::Regular, 34.0, word);
    set_hook_calls(0);
    font_draw_wrapped(&mut c, FontFace::Regular, 34.0, Rect { x: 0, y: 0, w: ww - 20, h: 200 }, two, 4, 0, true, 0);
    check!(G_HOOK.with(|h| {
        let h = h.borrow();
        h.calls == 1 && h.text == two && h.need == ww && h.have == ww - 20
    }));
    set_hook_calls(0);
    font_draw_wrapped(&mut c, FontFace::Regular, 34.0, Rect { x: 0, y: 0, w: ww, h: 200 }, two, 4, 0, true, 0); // the word fits
    check!(hook_calls() == 0);
    font_set_overflow_hook(None);
    font_draw_left(&mut c, FontFace::Regular, 44.0, Rect { x: 0, y: 0, w: 200, h: 100 }, longer, 0);
    check!(hook_calls() == 0);
    canvas_free(&mut c);
}

fn wrapping() {
    ui_set_theme(UiTheme::Gothic);
    let ls: f32 = 34.0 * 0.9;
    let w = font_measure(FontFace::Regular, ls, "Narrow, no margin");
    // Narrower than the label: two lines, broken after the comma (as the Library's tiles did).
    let mut widest = 0;
    let n = font_wrap_lines(FontFace::Regular, ls, "Wide, no margin", font_measure(FontFace::Regular, ls, "Wide, no margin") - 1, Some(&mut widest));
    check!(n == 2 && widest == font_measure(FontFace::Regular, ls, "no margin"));
    check!(font_wrap_lines(FontFace::Regular, ls, "Narrow, no margin", w - 1, Some(&mut widest)) == 2);
    check!(widest == font_measure(FontFace::Regular, ls, "no margin"));
    // Balanced: the longer line as short as it can be.
    let s = "aaaa bbbb cccc dddd eeee ffff gggg";
    let full = font_measure(FontFace::Regular, ls, s);
    check!(font_wrap_lines(FontFace::Regular, ls, s, full - 1, Some(&mut widest)) == 2);
    let after3 = font_measure(FontFace::Regular, ls, "dddd eeee ffff gggg");
    let after4 = font_measure(FontFace::Regular, ls, "aaaa bbbb cccc dddd");
    let first3 = font_measure(FontFace::Regular, ls, "aaaa bbbb cccc");
    let last3 = font_measure(FontFace::Regular, ls, "eeee ffff gggg");
    let best = if first3 > after3 { first3 } else { after3 };
    let other = if after4 > last3 { after4 } else { last3 };
    check!(widest == if best < other { best } else { other });
    check!(widest < full * 2 / 3);
    // A break after a comma wins while its longer line is at most half again as long as the
    // balanced break's: "Fino," over "sin margen", like its neighbours, not "Fino, sin" over "margen".
    let fino = "Fino, sin margen";
    let rest = font_measure(FontFace::Regular, ls, "sin margen");
    check!(font_measure(FontFace::Regular, ls, "Fino, sin") < rest); // (the balanced break is narrower)
    check!(font_wrap_lines(FontFace::Regular, ls, fino, rest + 4, Some(&mut widest)) == 2 && widest == rest);
    // A first clause much shorter than the rest stays on the line with it, though the rest would
    // fit a line of its own.
    let tip = "Tip: aaaa bbbb cccc dddd eeee ffff gggg hhhh";
    let tip_w = font_measure(FontFace::Regular, ls, tip);
    let after = font_measure(FontFace::Regular, ls, &tip[5..]);
    check!(font_wrap_lines(FontFace::Regular, ls, tip, tip_w - 1, Some(&mut widest)) == 2 && after <= tip_w - 1);
    check!(widest < after); // (after "Tip:" the longer line would be the rest)
    // Fits: one line; nothing: none; a word wider than the line: broken between its letters.
    check!(font_wrap_lines(FontFace::Regular, ls, s, full, None) == 1);
    check!(font_wrap_lines(FontFace::Regular, ls, "", 100, None) == 0);
    check!(font_wrap_lines(FontFace::Regular, ls, "Unbreakableword", font_measure(FontFace::Regular, ls, "Unbreakableword") / 2, None) >= 2);
    // One line that fits is drawn exactly as font_draw_centered / font_draw_left draw it.
    let mut a = Canvas::default();
    let mut b = Canvas::default();
    check!(canvas_init(&mut a, 600, 200) == 0 && canvas_init(&mut b, 600, 200) == 0);
    let all = Rect { x: 0, y: 0, w: 600, h: 200 };
    for center in [false, true] {
        canvas_fill(&mut a, all, 255);
        canvas_fill(&mut b, all, 255);
        let r = Rect { x: 20, y: 30, w: 560, h: 120 };
        if center {
            font_draw_centered(&mut a, FontFace::Bold, 44.0, r, "Calibration saved", 0);
        } else {
            font_draw_left(&mut a, FontFace::Bold, 44.0, r, "Calibration saved", 0);
        }
        check!(font_draw_wrapped(&mut b, FontFace::Bold, 44.0, r, "Calibration saved", 2, 0, center, 0) == 1);
        check!(same_pixels(&a, &b));
        canvas_fill(&mut b, all, 255);
        ui_text(&mut b, FontFace::Bold, 44.0, 34.0, r, "Calibration saved", 2, center, 0);
        check!(same_pixels(&a, &b));
    }
    canvas_free(&mut a);
    canvas_free(&mut b);
}

fn fit_sizes() {
    ui_set_theme(UiTheme::Gothic);
    let s = "Restablecer herramienta";
    let mut at44 = font_measure(FontFace::Caps, 44.0, s);
    let at34 = font_measure(FontFace::Caps, 34.0, s);
    check!(at34 < at44);
    check!(font_fit_px(FontFace::Caps, 44.0, 34.0, s, at44) == 44.0); // fits as it is
    // Jersey 10 has whole steps only: anything narrower than at 44 px is the 34 px step.
    let mut f = font_fit_px(FontFace::Caps, 44.0, 34.0, s, at44 - 1);
    check!(f >= 34.0 && f < 44.0 && font_measure(FontFace::Caps, f, s) == at34);
    check!(font_fit_px(FontFace::Caps, 44.0, 34.0, s, at34 - 1) == 0.0); // and nothing below that
    check!(font_fit_px(FontFace::Caps, 44.0, 40.0, s, at44 - 1) > 0.0); // a floor inside the step: the step
    // Terminess shrinks in quarter pixels: the largest size that fits.
    ui_set_theme(UiTheme::Classic);
    at44 = font_measure(FontFace::Regular, 34.0, s);
    f = font_fit_px(FontFace::Regular, 34.0, 27.0, s, at44 * 9 / 10);
    check!(f > 27.0 && f < 34.0 && font_measure(FontFace::Regular, f, s) <= at44 * 9 / 10);
    check!(font_measure(FontFace::Regular, f + 0.25, s) > at44 * 9 / 10);
    check!(font_fit_px(FontFace::Regular, 34.0, 27.0, s, 10) == 0.0);
    ui_set_theme(UiTheme::Gothic);
    check!(ui_text_box_w(FontFace::Bold, 34.0, "150%   tap to reset", 40, 380, 900) >= 380);
    check!(ui_text_box_w(FontFace::Bold, 34.0, "", 40, 380, 900) == 380);
    check!(ui_text_box_w(FontFace::Bold, 34.0, "A label far longer than any box could ever want to be", 40, 380, 500) == 500);
}

// The columns where two drawings of a button differ within rows [y0, y1): its label's ink, when
// the one is drawn with the label and the other with " " (the same layout, nothing to see).
fn ink_cols(a: &Canvas, b: &Canvas, y0: i32, y1: i32, x0: &mut i32, x1: &mut i32) {
    *x0 = a.w;
    *x1 = -1;
    for y in y0..y1 {
        for x in 0..a.w {
            if a.px[(y * a.stride + x) as usize] != b.px[(y * b.stride + x) as usize] {
                if x < *x0 {
                    *x0 = x;
                }
                if x > *x1 {
                    *x1 = x;
                }
            }
        }
    }
}

// A label in a Gothic button 90 px or more each way keeps inside the carved inner line (2 px wide,
// 10 px in, 13 px in a primary button's), 4 px clear of it: a word up to the edge's inset ran into
// the line ("Compartir" under the Share icon did). Classic has no such line. A row of buttons that
// widens for its longest word (the Library's top bar) asks ui_button_need_w.
fn button_labels_clear_the_carved_line() {
    let mut a = Canvas::default();
    let mut b = Canvas::default();
    check!(canvas_init(&mut a, 400, 150) == 0 && canvas_init(&mut b, 400, 150) == 0);
    font_set_overflow_hook(Some(hook));
    let word = "Compartir"; // 150 px under an icon in Gothic, 140 in Classic
    let whole = Rect { x: 0, y: 0, w: 400, h: 150 };
    for th in THEMES {
        ui_set_theme(th);
        for primary in [false, true] {
            let st = if primary { ButtonStyle::Primary } else { ButtonStyle::Plain };
            let inset = if th == UiTheme::Gothic { (if primary { 13 } else { 10 }) + 2 + 4 } else { 8 }; // the label's inset
            for icon in [false, true] {
                let ik = if icon { IconKind::Share } else { IconKind::None };
                let px: f32 = if icon { 34.0 } else { 44.0 };
                let need = ui_button_need_w(150, ik, word, st);
                check!(need == font_measure(FontFace::Caps, px, word) + 2 * inset);
                // As wide as it needs: whole, inside the line (or Classic's edge) with room to spare.
                let r = Rect { x: 0, y: 0, w: need, h: 150 };
                canvas_fill(&mut a, whole, 255);
                canvas_fill(&mut b, whole, 255);
                set_hook_calls(0);
                ui_button(&mut a, r, ik, word, st);
                ui_button(&mut b, r, ik, " ", st);
                check!(hook_calls() == 0);
                let (mut x0, mut x1) = (0, 0);
                ink_cols(&a, &b, if icon { 90 } else { 8 }, 142, &mut x0, &mut x1);
                if x0 < inset || x1 >= need - inset {
                    eprintln!("  ink {}..{} in {} px (inset {})", x0, x1, need, inset);
                }
                check!(x0 >= inset && x1 < need - inset && x1 > x0);
                if icon {
                    // A pixel narrower: cut short, and the fit check hears of it.
                    ui_button(&mut a, Rect { x: 0, y: 0, w: need - 1, h: 150 }, ik, word, st);
                    check!(hook_calls() == 1);
                }
            }
        }
        // The top bar holds every language's words: none needs more than its buttons can grow to.
        const BAR: [&str; 5] = ["Exit", "Settings", "Share", "Folder", "New"];
        const BAR_ICON: [IconKind; 5] = [IconKind::Exit, IconKind::Gear, IconKind::Share, IconKind::FolderPlus, IconKind::Plus];
        for l in 0..i18n_language_count() {
            i18n_set(i18n_language_code(l).unwrap());
            for i in 0..5 {
                let need = ui_button_need_w(150, BAR_ICON[i], tr(BAR[i]), if i == 4 { ButtonStyle::Primary } else { ButtonStyle::Plain });
                if need > 220 {
                    eprintln!("  {}: \"{}\" needs a {} px button", i18n_current(), tr(BAR[i]), need);
                }
                check!(need <= 220);
            }
        }
        i18n_set("en");
    }
    font_set_overflow_hook(None);
    canvas_free(&mut a);
    canvas_free(&mut b);
    ui_set_theme(UiTheme::Gothic);
}

// The rows of a box the label's ink spans (as ink_cols, by rows): how tall it is drawn.
fn ink_rows(a: &Canvas, b: &Canvas) -> i32 {
    let (mut y0, mut y1) = (a.h, -1);
    for y in 0..a.h {
        let (ra, rb) = ((y * a.stride) as usize, (y * b.stride) as usize);
        if a.px[ra..ra + a.w as usize] != b.px[rb..rb + a.w as usize] {
            if y < y0 {
                y0 = y;
            }
            y1 = y;
        }
    }
    if y1 >= y0 {
        y1 - y0 + 1
    } else {
        0
    }
}

// Buttons in a grid (Custom music's parts) draw their labels at one size, the smallest any of
// them needs: ui_button_text_px is the size ui_button gives a label of its own, and
// ui_button_text_at draws one at a given size.
fn button_grid_sizes() {
    let mut a = Canvas::default();
    let mut b = Canvas::default();
    check!(canvas_init(&mut a, 312, 103) == 0 && canvas_init(&mut b, 312, 103) == 0);
    let r = Rect { x: 0, y: 0, w: 312, h: 103 }; // a part's button, upright
    let small = "Bass staff";
    let longer = "Tablatura, 4 l\u{ed}neas";
    let far = "A name far too long for one line";
    for th in THEMES {
        ui_set_theme(th);
        let ts = ui_text_size();
        let ss = ui_small_size();
        check!(ui_button_text_px(r, small, ButtonStyle::Plain) == ts);
        let s = ui_button_text_px(r, longer, ButtonStyle::Plain);
        check!(s >= ss && s < ts);
        check!(ui_button_text_px(r, far, ButtonStyle::Plain) == ss); // two lines at the small size
        // At its own size a label is drawn as ui_button draws it.
        for label in [small, longer, far] {
            canvas_fill(&mut a, r, 255);
            canvas_fill(&mut b, r, 255);
            ui_button(&mut a, r, IconKind::None, label, ButtonStyle::Plain);
            ui_button_text_at(&mut b, r, label, ButtonStyle::Plain, ui_button_text_px(r, label, ButtonStyle::Plain));
            check!(same_pixels(&a, &b));
        }
        // At the grid's size, a short label is drawn smaller than it would be on its own.
        canvas_fill(&mut b, r, 255);
        ui_button(&mut b, r, IconKind::None, " ", ButtonStyle::Plain);
        canvas_fill(&mut a, r, 255);
        ui_button(&mut a, r, IconKind::None, small, ButtonStyle::Plain);
        let own = ink_rows(&a, &b);
        canvas_fill(&mut a, r, 255);
        ui_button_text_at(&mut a, r, small, ButtonStyle::Plain, s);
        check!(ink_rows(&a, &b) < own && ink_rows(&a, &b) > 0);
    }
    canvas_free(&mut a);
    canvas_free(&mut b);
    ui_set_theme(UiTheme::Gothic);
}

// The zoom chip: 380 px for the English at every zoom, in both styles. A longer text keeps as far
// from Gothic's pointed ends (each 50 px deep) as the widest English does from them, 43 px;
// Classic's round ends leave it room closer in.
fn chips() {
    for th in THEMES {
        ui_set_theme(th);
        i18n_set("en");
        for pct in 50..=400 {
            let z = cformat!(tr("%d%%   tap to reset"), pct);
            check!(ui_chip_w(FontFace::Bold, ui_small_size(), &z, 100, 380, 2400) == 380);
        }
        let longer = "191 %   toca para restablecer";
        let tw = font_measure(FontFace::Bold, ui_small_size(), longer);
        let pad = if th == UiTheme::Gothic { 43 } else { 24 };
        check!(tw + 2 * pad > 380);
        check!(ui_chip_w(FontFace::Bold, ui_small_size(), longer, 100, 380, 2400) == tw + 2 * pad);
        check!(ui_chip_w(FontFace::Bold, ui_small_size(), longer, 100, 380, 500) == 500);
    }
    ui_set_theme(UiTheme::Gothic);
}

// Toasts: one line where it fits, in the very box they always had (tests/scenarios/
// t_toast_over_sheets.qs checks where it goes); two, in a taller box, where one would be cut.
fn toasts() {
    let screen = Rect { x: 0, y: 0, w: 1860, h: 2480 };
    ui_set_theme(UiTheme::Gothic);
    let mut r = ui_toast_rect(screen, "Your Kindle passcode was left off. Turn it back on in Kindle Settings.");
    check!(r.x == 112 && r.y == 2250 && r.w == 1636 && r.h == 110);
    let damaged = "A damaged page was set aside, and a blank one put in its place: grimoire.log says which";
    check!(font_measure(FontFace::Bold, ui_text_size(), damaged) > ui_toast_room(screen)); // one line would cut it
    r = ui_toast_rect(screen, damaged);
    check!(r.h == 110 + font_ascent(FontFace::Bold, ui_text_size()) + font_descent(FontFace::Bold, ui_text_size()));
    check!(r.y + r.h == 2480 - 120 && r.w <= 1800);
    let mut c = Canvas::default();
    check!(canvas_init(&mut c, 1860, 2480) == 0);
    canvas_fill(&mut c, screen, 255);
    font_set_overflow_hook(Some(hook));
    set_hook_calls(0);
    ui_toast_at(&mut c, r, damaged);
    check!(hook_calls() == 0); // two lines hold it
    ui_set_theme(UiTheme::Classic);
    r = ui_toast_rect(screen, damaged);
    check!(r.h == 110 || r.h == 110 + font_ascent(FontFace::Bold, ui_text_size()) + font_descent(FontFace::Bold, ui_text_size()));
    ui_toast_at(&mut c, r, damaged);
    check!(hook_calls() == 0);
    // Three lines' worth: two, the second cut short, and the hook hears of it.
    let three = format!("{} {} {}", damaged, damaged, damaged);
    r = ui_toast_rect(screen, &three);
    check!(r.w == 1800);
    ui_toast_at(&mut c, r, &three);
    check!(hook_calls() == 1);
    font_set_overflow_hook(None);
    canvas_free(&mut c);
    ui_set_theme(UiTheme::Gothic);
}

#[test]
fn test_i18n() {
    let root = format!("{}/i18n-{}", tl_tmp(), std::process::id());
    let _ = std::fs::remove_dir_all(&root);
    check!(store_init(&root) == 0);
    ui_init(300.0);
    if font_init() != 0 {
        panic!("font_init failed");
    }
    run!(format_reader);
    run!(format_compatibility);
    run!(catalogs_hold_together);
    run!(lookups);
    run!(plurals);
    run!(damaged_catalogs);
    run!(context_plurals);
    run!(labels_and_identifiers);
    run!(locale_names);
    run!(detection_order);
    run!(applying_a_setting);
    run!(language_setting);
    run!(dates);
    run!(rare_notices_fit);
    run!(copies_cut_between_characters);
    run!(raw_labels_fit);
    run!(paper_is_the_same_in_every_language);
    run!(overflow_hook);
    run!(wrapping);
    run!(fit_sizes);
    run!(button_labels_clear_the_carved_line);
    run!(button_grid_sizes);
    run!(chips);
    run!(toasts);
    // (the C removes the root whatever tl_done says)
    let _ = std::fs::remove_dir_all(&root);
    done();
}
