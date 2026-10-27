//! i18n.h: user-visible text goes through these, so it can be translated. Wrap whole sentences,
//! with %s for paths and values; never wrap identifiers (settings keys, file names, log lines, API
//! fields). English stays in the code; each language is po/<lang>.po (for translators), compiled
//! to po/<lang>.mo, which is committed (building needs no gettext) and embedded in the binary like
//! the fonts (build/gen/l10n.c). docs/FORMAT.md has the language setting; the Makefile's l10n and
//! l10n-check rules have the rest.
//!   _("Saved")                       a string shown to the user, looked up now
//!   N_("Pencil")                     marks a string for translation in a static table (no lookup:
//!                                    whoever draws it looks it up with _())
//!   C_("paper", "Blank")             like _(), with a context telling two meanings apart
//!   NC_("paper", "Blank")            marks it with that context in a static table (look it up
//!                                    with C_() and the same context)
//!   NG_("%d page", "%d pages", n)    the plural form for n, looked up now
//! A lookup the language has no translation for gives the English. _(), C_() and NG_() are calls
//! that return a const char *, so they cannot initialize a static table: mark its strings with
//! N_() or NC_() and look each up where it is drawn. The lookups are declared format_arg, so the
//! compiler checks every printf against the English text (NG_ against both forms); msgfmt --check
//! (make l10n, make l10n-check) and tests/test_i18n.c check each translation against its English.
//! A translation can be longer than the English a buffer was sized for: copy one with i18n_copy.

use crate::prelude::*;

// The C's macros are functions here, under names xgettext can be given as keywords (the l10n rule
// in ../Makefile; PORTING.md, Translations):
//   _("Saved")                 tr("Saved")
//   N_("Pencil")               n_("Pencil")
//   C_("paper", "Blank")       c_("paper", "Blank")
//   NC_("paper", "Blank")      nc_("paper", "Blank")
//   NG_("%d page", "%d pages", n)   ng_("%d page", "%d pages", n)
// A lookup returns the translation, or the English (msgid) itself: both live as long as the program
// (the catalogs are embedded), hence the 'static. A translation is a printf format when its English
// is: format it with cformat! (cfmt.rs), which takes positional conversions ("%2$d %1$s") too.
// (The C's i18n_ng and the ?: in NG_ only make clang check printf against both English forms; Rust
// has no such check, so tests/test_i18n.c's port and msgfmt --check carry it.)

/// _(): a string shown to the user, looked up now.
pub fn tr(msgid: &'static str) -> &'static str {
    i18n_get(msgid)
}
/// N_(): marks a string for translation in a static table; whoever draws it looks it up with tr().
pub const fn n_(s: &'static str) -> &'static str {
    s
}
/// C_(): like tr(), with a context telling two meanings apart.
pub fn c_(ctx: &str, msgid: &'static str) -> &'static str {
    i18n_getc(ctx, msgid)
}
/// NC_(): marks a string with its context in a static table (look it up with c_() and the same
/// context).
pub const fn nc_(_ctx: &str, s: &'static str) -> &'static str {
    s
}
/// NG_(): the plural form for n, looked up now.
pub fn ng_(one: &'static str, many: &'static str, n: i64) -> &'static str {
    i18n_getn(one, many, n)
}


// i18n.c: the language in use and the lookups (i18n.h). A catalog is a GNU .mo file as msgfmt
// writes it (po/<lang>.mo, embedded): a header, then two tables of n (length, offset) pairs, the
// English strings sorted byte by byte and their translations in the same order. A string with a
// context is stored as "context\004English"; a plural as "English\0English plural", its
// translation as its forms one after another, NUL-separated. Lookups are a binary search; each
// language's plural rule is written here by hand (the catalog's Plural-Forms line is only read by
// tests/test_i18n.c, to hold the two to each other). English is no catalog at all: the text as
// the code has it.

use std::cell::RefCell;
use std::cmp::Ordering;

// Which plural form a count takes, per language (an index into the catalog's forms).
fn plural_one_other(n: i64) -> i32 {
    (n != 1) as i32
}

struct Lang {
    code: &'static str,
    /// the name in the language itself (UTF-8)
    name: &'static str,
    nforms: i32,
    plural: fn(i64) -> i32,
}
const K_LANGS: [Lang; 2] = [
    Lang { code: "en", name: "English", nforms: 2, plural: plural_one_other },
    Lang { code: "es", name: "Espa\u{f1}ol", nforms: 2, plural: plural_one_other },
];
const NLANGS: usize = K_LANGS.len();

pub fn i18n_language_count() -> i32 {
    NLANGS as i32
}
pub fn i18n_language_code(i: i32) -> Option<&'static str> {
    if i >= 0 && (i as usize) < NLANGS {
        Some(K_LANGS[i as usize].code)
    } else {
        None
    }
}
pub fn i18n_language_name(i: i32) -> Option<&'static str> {
    if i >= 0 && (i as usize) < NLANGS {
        Some(K_LANGS[i as usize].name)
    } else {
        None
    }
}

// ---- catalogs ----

// The bytes of a catalog and where its parts are. 'a is 'static for the catalogs the lookups use
// (embedded, or read once and kept); i18n_mo_valid checks bytes it only borrows.
#[derive(Clone, Copy)]
struct Catalog<'a> {
    d: &'a [u8],
    /// strings; where the two tables start
    size: u32,
    n: u32,
    orig: u32,
    trans: u32,
    /// written big-endian
    be: bool,
}

// A C string's bytes: up to the first NUL (a plural's English holds two, its translation all its
// forms: the C reads each as a string and steps over the NUL itself).
fn cstr(s: &[u8]) -> &[u8] {
    &s[..s.iter().position(|&b| b == 0).unwrap_or(s.len())]
}

// A string out of a catalog, as the text the lookups return. Catalogs are UTF-8 (msgfmt writes
// them so); bytes that are not read as no string, the lookups' English.
fn utf8(b: &'static [u8]) -> Option<&'static str> {
    std::str::from_utf8(b).ok()
}

// The longest prefix of b that is UTF-8 (i18n_walk's entries, for a catalog that is not).
fn utf8_prefix(b: &'static [u8]) -> &'static str {
    match std::str::from_utf8(b) {
        Ok(s) => s,
        Err(e) => std::str::from_utf8(&b[..e.valid_up_to()]).unwrap_or(""),
    }
}

impl<'a> Catalog<'a> {
    fn rd32(&self, off: u32) -> u32 {
        let p = &self.d[off as usize..off as usize + 4];
        if self.be {
            (p[0] as u32) << 24 | (p[1] as u32) << 16 | (p[2] as u32) << 8 | p[3] as u32
        } else {
            (p[3] as u32) << 24 | (p[2] as u32) << 16 | (p[1] as u32) << 8 | p[0] as u32
        }
    }

    // String i of a table (its bytes, without the NUL); None unless it lies inside the file and
    // ends in a NUL.
    fn str_at(&self, table: u32, i: u32) -> Option<&'a [u8]> {
        let len = self.rd32(table + i * 8);
        let off = self.rd32(table + i * 8 + 4);
        if off >= self.size || len >= self.size - off || self.d[(off + len) as usize] != 0 {
            return None;
        }
        Some(&self.d[off as usize..(off + len) as usize])
    }
}

// Reads a catalog, checking all of it once: the tables and every string inside the file, the
// English in order (or the binary search would miss). None for anything else.
fn catalog_open(d: &[u8]) -> Option<Catalog<'_>> {
    let size = u32::try_from(d.len()).unwrap_or(0);
    let mut c = Catalog { d, size, n: 0, orig: 0, trans: 0, be: false };
    if size < 28 {
        return None;
    }
    let magic = d[0] as u32 | (d[1] as u32) << 8 | (d[2] as u32) << 16 | (d[3] as u32) << 24;
    if magic == 0xde120495 {
        c.be = true;
    } else if magic != 0x950412de {
        return None;
    }
    if c.rd32(4) >> 16 > 1 {
        return None; // major revision 0, or 1 (system-dependent strings, unused)
    }
    c.n = c.rd32(8);
    c.orig = c.rd32(12);
    c.trans = c.rd32(16);
    if c.n > size / 16 || c.orig > size - c.n * 8 || c.trans > size - c.n * 8 {
        return None;
    }
    let mut prev: Option<&[u8]> = None;
    for i in 0..c.n {
        let o = c.str_at(c.orig, i)?;
        c.str_at(c.trans, i)?;
        if let Some(p) = prev {
            if cstr(p) >= cstr(o) {
                return None;
            }
        }
        prev = Some(o);
    }
    Some(c)
}

// strcmp of "ctx\004id" (or of "id", no ctx) against s, without building the key.
fn key_cmp(ctx: Option<&str>, id: &str, s: &[u8]) -> Ordering {
    let mut s = cstr(s);
    if let Some(ctx) = ctx {
        for &ch in cstr(ctx.as_bytes()) {
            let sc = s.first().copied().unwrap_or(0);
            if ch != sc {
                return ch.cmp(&sc);
            }
            s = &s[1..];
        }
        let sc = s.first().copied().unwrap_or(0);
        if sc != 4 {
            return 4u8.cmp(&sc);
        }
        s = &s[1..];
    }
    cstr(id.as_bytes()).cmp(s)
}

// The translation of id (in ctx), all its bytes (a plural's forms are NUL-separated); None if the
// catalog has none or it is empty.
fn catalog_find<'a>(c: &Catalog<'a>, ctx: Option<&str>, id: &str) -> Option<&'a [u8]> {
    let (mut lo, mut hi) = (0u32, c.n);
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        let o = c.str_at(c.orig, mid)?;
        match key_cmp(ctx, id, o) {
            Ordering::Equal => {
                let t = c.str_at(c.trans, mid)?;
                return if t.first().copied().unwrap_or(0) != 0 { Some(t) } else { None };
            }
            Ordering::Less => hi = mid,
            Ordering::Greater => lo = mid + 1,
        }
    }
    None
}

// The simulator's GRIMOIRE_L10N_DIR (a test hook, and a translator's shortcut): a language's
// catalog is read from <dir>/<code>.mo instead of the binary, once, and kept for the run. None
// when the variable is not set (the embedded catalogs, as on the Kindle) or that file is missing.
#[cfg(feature = "sim")]
type KRead = (String, Option<&'static [u8]>);
#[cfg(feature = "sim")]
thread_local! {
    // The C's k_read[8]: the code, and the file's bytes (kept for the run, so leaked) if it was there.
    static K_READ: RefCell<Vec<KRead>> = const { RefCell::new(Vec::new()) };
}
#[cfg(feature = "sim")]
fn sim_catalog(code: &str) -> Option<&'static [u8]> {
    let dir = std::env::var_os("GRIMOIRE_L10N_DIR")?;
    if dir.is_empty() || code.len() >= 8 {
        return None;
    }
    if let Some(d) = K_READ.with(|k| k.borrow().iter().find(|e| e.0 == code).map(|e| e.1)) {
        return d;
    }
    if K_READ.with(|k| k.borrow().len()) >= 8 {
        return None;
    }
    let mut path = String::new();
    csnprintf!(&mut path, 1024, "%s/%s.mo", &*dir.to_string_lossy(), code);
    let d: Option<&'static [u8]> = read_file(&path).map(|v| {
        // (a file of 4 GB or more reads as empty, as the C's uint32_t size does)
        let v: &'static [u8] = Box::leak(v.into_boxed_slice());
        if v.len() < u32::MAX as usize {
            v
        } else {
            &v[..0]
        }
    });
    K_READ.with(|k| k.borrow_mut().push((code.to_string(), d)));
    qlog!("i18n: (simulator) the catalog for %s %s %s", code, if d.is_some() { "comes from" } else { "is not in" }, &path);
    d
}

// The embedded catalog of a language: None if there is none (English) or it does not read.
fn catalog_for(code: &str) -> Option<Catalog<'static>> {
    #[cfg(feature = "sim")]
    if let Some(sim) = sim_catalog(code) {
        let c = catalog_open(sim);
        if c.is_none() {
            qlog!("i18n: the catalog for %s is damaged; using English", code);
        }
        return c;
    }
    for k in i18n_catalogs.iter() {
        if k.code == code {
            let c = catalog_open(k.mo);
            if c.is_none() {
                qlog!("i18n: the catalog for %s is damaged; using English", code);
            }
            return c;
        }
    }
    None
}

// The C's file-level statics (g_lang, g_cat and g_have_cat, g_auto and g_auto_why).
struct State {
    /// index into K_LANGS
    lang: usize,
    /// its catalog (g_have_cat: it is Some)
    cat: Option<Catalog<'static>>,
    auto: usize,
    auto_why: String,
}
impl Default for State {
    fn default() -> State {
        State { lang: 0, cat: None, auto: 0, auto_why: "not detected".to_string() }
    }
}
thread_local! {
    static S: RefCell<State> = RefCell::new(State::default());
}
fn with<R>(f: impl FnOnce(&mut State) -> R) -> R {
    S.with(|s| f(&mut s.borrow_mut()))
}

pub fn i18n_get(msgid: &'static str) -> &'static str {
    let cat = with(|s| s.cat);
    let Some(c) = cat else { return msgid };
    if msgid.is_empty() {
        return msgid;
    }
    catalog_find(&c, None, msgid).and_then(|t| utf8(cstr(t))).unwrap_or(msgid)
}

pub fn i18n_getc(ctx: &str, msgid: &'static str) -> &'static str {
    let cat = with(|s| s.cat);
    let Some(c) = cat else { return msgid };
    if msgid.is_empty() {
        return msgid;
    }
    catalog_find(&c, Some(ctx), msgid).and_then(|t| utf8(cstr(t))).unwrap_or(msgid)
}

// getnc, where the context may be none (i18n_getn's: "id" is not "\004id").
fn getnc(ctx: Option<&str>, one: &'static str, many: &'static str, n: i64) -> &'static str {
    let (cat, lang) = with(|s| (s.cat, s.lang));
    let t = match cat {
        Some(c) if !one.is_empty() => catalog_find(&c, ctx, one),
        _ => None,
    };
    if let Some(t) = t {
        // Form k: past k NULs, if the translation has that many forms. (pos == len is the NUL that
        // ends the translation, one past it is the end.)
        let len = t.len();
        let mut pos = 0usize;
        let mut k = (K_LANGS[lang].plural)(n);
        while k > 0 && pos <= len {
            pos += cstr(&t[pos..]).len() + 1;
            k -= 1;
        }
        if pos < len && t[pos] != 0 {
            if let Some(s) = utf8(cstr(&t[pos..])) {
                return s;
            }
        }
    }
    if n == 1 {
        one
    } else {
        many
    }
}

pub fn i18n_getn(one: &'static str, many: &'static str, n: i64) -> &'static str {
    getnc(None, one, many, n)
}
pub fn i18n_getn_many(one: &'static str, many: &'static str, n: i64) -> &'static str {
    getnc(None, one, many, n)
}
/// The plural form for n of a string with a context (C_ and NG_ at once), looked up now; for
/// strings the code gets from a table (the share studio's, httpd.c), so it has no macro.
pub fn i18n_getnc(ctx: &str, one: &'static str, many: &'static str, n: i64) -> &'static str {
    getnc(Some(ctx), one, many, n)
}

/// Copies src into dst (cap bytes, with the NUL) as strlcpy_q does, but text too long for dst is
/// cut between two characters, never inside one, and so is text snprintf already cut inside one.
/// Returns the bytes copied.
pub fn i18n_copy(dst: &mut String, src: &str, cap: usize) -> usize {
    if cap == 0 {
        return 0;
    }
    // (a &str is whole characters, so strlcpy_q's cut is the only one to make: it drops a
    // character it would split)
    strlcpy_q(dst, src, cap)
}

// ---- choosing ----

// i18n_find on bytes: the environment and the Kindle's locale file need not be UTF-8.
fn find_bytes(locale: &[u8]) -> i32 {
    // The language part: the letters before a region, an encoding or a modifier.
    let mut part = [0u8; 8];
    let mut n = 0usize;
    while n < locale.len() && locale[n] != 0 && !b"_-.@".contains(&locale[n]) && n + 1 < part.len() {
        let ch = locale[n];
        part[n] = if ch.is_ascii_uppercase() { ch - b'A' + b'a' } else { ch };
        n += 1;
    }
    for (i, l) in K_LANGS.iter().enumerate() {
        if &part[..n] == l.code.as_bytes() {
            return i as i32;
        }
    }
    -1
}

/// The language a locale name is in, by its language part ("es_MX.UTF-8", "es-419", "ES": es):
/// its index, or -1 for one Grimoire does not speak (and for "", "C" and "POSIX").
pub fn i18n_find(locale: &str) -> i32 {
    find_bytes(locale.as_bytes())
}

/// Every lookup in that language from now on (a code or a locale name); false, and English, for a
/// language Grimoire does not speak.
pub fn i18n_set(code: &str) -> bool {
    let i = i18n_find(code);
    let lang = if i < 0 { 0 } else { i as usize };
    // (the catalog is read, and logged about, before the state is touched)
    let cat = if lang != 0 { catalog_for(K_LANGS[lang].code) } else { None };
    with(|s| {
        s.lang = lang;
        s.cat = cat;
    });
    i >= 0
}

/// the code of the language in use
pub fn i18n_current() -> &'static str {
    K_LANGS[with(|s| s.lang)].code
}

/// settings.ini's language=: "auto", or a code from the list. Any other value (a language a newer
/// Grimoire speaks) acts as auto, and store.c keeps it in the file.
pub fn i18n_setting_known(value: &str) -> bool {
    value == "auto" || K_LANGS.iter().any(|l| value == l.code)
}

// A value from the environment or the locale file as the log shows it: printable ASCII, short.
fn loggable(v: &[u8], cap: usize) -> String {
    let mut out = String::new();
    for &b in v {
        if b == 0 || !(out.len() + 1 < cap && out.len() < 40) {
            break;
        }
        out.push(if b > b' ' && b < 0x7F && b != b'"' { b as char } else { '?' });
    }
    out
}

// i18n_locale_file_lang on bytes: the value, at most cap - 1 of them ("" if none).
fn locale_file_lang(text: &[u8], cap: usize) -> Vec<u8> {
    let text = cstr(text);
    for key in [&b"LANG="[..], &b"LC_ALL="[..]] {
        let mut at = 0usize;
        while at < text.len() {
            let line = &text[at..];
            let mut s = line;
            while s.first() == Some(&b' ') || s.first() == Some(&b'\t') {
                s = &s[1..];
            }
            if s.starts_with(b"export ") {
                s = &s[7..];
            }
            if s.starts_with(key) {
                s = &s[key.len()..];
                if s.first() == Some(&b'"') || s.first() == Some(&b'\'') {
                    s = &s[1..];
                }
                // strcspn(s, "\"'\r\n \t")
                let mut n = s.iter().position(|b| b"\"'\r\n \t".contains(b)).unwrap_or(s.len());
                if n > 0 {
                    if n >= cap {
                        n = cap - 1;
                    }
                    return s[..n].to_vec();
                }
            }
            match line.iter().position(|&b| b == b'\n') {
                Some(nl) => at += nl + 1,
                None => break,
            }
        }
    }
    Vec::new()
}

/// The value of the LANG= line (else the LC_ALL= line) of a locale file's text ("" if none).
pub fn i18n_locale_file_lang(text: &str, out: &mut String, cap: usize) {
    if cap == 0 {
        return;
    }
    out.clear();
    let v = locale_file_lang(text.as_bytes(), cap);
    // (whole characters but for a cut at cap - 1 bytes, which drops the one it would split)
    match std::str::from_utf8(&v) {
        Ok(s) => out.push_str(s),
        Err(e) => out.push_str(std::str::from_utf8(&v[..e.valid_up_to()]).unwrap_or("")),
    }
}

// The environment variable as bytes (the C's getenv).
fn getenv(name: &str) -> Option<Vec<u8>> {
    use std::os::unix::ffi::OsStrExt;
    std::env::var_os(name).map(|v| v.as_bytes().to_vec())
}

// i18n_detect_from on the locale file's bytes, which need not be UTF-8.
fn detect_from(kindle_locale: Option<&[u8]>, why: &mut String, cap: usize) -> i32 {
    let file_lang = kindle_locale.map_or_else(Vec::new, |t| locale_file_lang(t, 64));
    let src: [(Option<&str>, Option<Vec<u8>>); 5] = [
        (Some("GRIMOIRE_LANG"), getenv("GRIMOIRE_LANG")),
        (None, Some(file_lang)), // the Kindle's locale
        (Some("LC_ALL"), getenv("LC_ALL")),
        (Some("LC_MESSAGES"), getenv("LC_MESSAGES")),
        (Some("LANG"), getenv("LANG")),
    ];
    for (name, value) in &src {
        let Some(value) = value else { continue };
        // (a value is a C string: what follows a NUL is not there)
        let value = cstr(value);
        if value.is_empty() {
            continue;
        }
        let v = loggable(value, 48);
        let lang = find_bytes(value);
        let not_spoken = if lang < 0 { ", not a language Grimoire speaks" } else { "" };
        match name {
            Some(name) => csnprintf!(why, cap, "from %s=%s%s", *name, &v, not_spoken),
            None => csnprintf!(why, cap, "from the Kindle's locale %s%s", &v, not_spoken),
        };
        return if lang < 0 { 0 } else { lang };
    }
    csnprintf!(why, cap, "no language set");
    0
}

/// The same choice from what was read: the text of the Kindle's locale file (NULL where there is
/// none) and the environment. The language's index; why: where it came from, for the log.
pub fn i18n_detect_from(kindle_locale: Option<&str>, why: &mut String, cap: usize) -> i32 {
    detect_from(kindle_locale.map(|t| t.as_bytes()), why, cap)
}

/// What auto stands for, found once at startup: GRIMOIRE_LANG if it is set (nothing on the Kindle
/// sets it; a launcher may), then on the Kindle the language of its own Settings (the LANG= line
/// of /var/local/system/locale), ranked above any LC_ALL or LANG Grimoire inherits (KOReader
/// exports LC_ALL=en_US.UTF-8 to what it starts), then LC_ALL, LC_MESSAGES, LANG. The first one
/// set decides; a language Grimoire does not speak means English.
pub fn i18n_detect() {
    #[cfg(feature = "kindle")]
    let (text, missing) = {
        let text = read_file("/var/local/system/locale");
        let missing = if text.is_none() { "; /var/local/system/locale cannot be read" } else { "" };
        (text, missing)
    };
    #[cfg(not(feature = "kindle"))]
    let (text, missing): (Option<Vec<u8>>, &str) = (None, "");
    let mut why = String::new();
    let auto = detect_from(text.as_deref(), &mut why, 112);
    let mut auto_why = String::new();
    csnprintf!(&mut auto_why, 160, "%s%s", &why, missing);
    with(|s| {
        s.auto = auto as usize;
        s.auto_why = auto_why;
    });
}

/// the code auto stands for (English before i18n_detect)
pub fn i18n_auto() -> &'static str {
    K_LANGS[with(|s| s.auto)].code
}

/// Sets the language for a language= value, and logs which one and why.
pub fn i18n_apply(setting: &str) {
    if setting != "auto" && i18n_setting_known(setting) {
        i18n_set(setting);
        qlog!("i18n: language %s (set in Settings)", i18n_current());
        return;
    }
    let (auto, auto_why) = with(|s| (s.auto, s.auto_why.clone()));
    i18n_set(K_LANGS[auto].code);
    let mut v = String::new();
    if !setting.is_empty() && setting != "auto" {
        v = loggable(setting.as_bytes(), 48);
    }
    qlog!(
        "i18n: language %s (auto, %s)%s%s%s",
        i18n_current(),
        &auto_why,
        if !v.is_empty() { "; settings.ini says language=" } else { "" },
        &v,
        if !v.is_empty() { ", a language this version does not speak" } else { "" }
    );
}

/// Dates in the language in use (util.c): "Oct 10, 14:32" (format_time_short, common.h), and the
/// date alone, "Oct 10", as in a new notebook's name. (Both are util.rs's, as util.c has them.)
pub use crate::util::format_date_short;

/// Each entry of the catalog for code, in catalog order (sorted by context and English): for
/// tests, and for the share studio, which gets the words of its context (httpd.c).
#[derive(Clone, Debug, Default)]
pub struct I18nEntry {
    /// "" for none
    pub ctx: FixedStr<64>,
    /// the English, and its plural form (None for none)
    pub id: &'static str,
    pub id_plural: Option<&'static str>,
    /// the translation; a plural's forms follow one another, each ending in its NUL: nforms of them
    /// (1 for no plural)
    pub str: &'static str,
    pub nforms: i32,
}

// ---- for tests ----

/// Calls f for every entry but the header; returns how many, or -1 if code has no valid catalog.
pub fn i18n_walk(code: &str, f: &mut dyn FnMut(&I18nEntry)) -> i32 {
    let Some(c) = catalog_for(code) else { return -1 };
    let mut count = 0;
    for i in 0..c.n {
        // (catalog_open has seen both tables' strings are there)
        let (Some(o), Some(t)) = (c.str_at(c.orig, i), c.str_at(c.trans, i)) else { continue };
        if o.first().copied().unwrap_or(0) == 0 {
            continue; // the header
        }
        let mut e = I18nEntry { str: utf8_prefix(t), nforms: 1, ..Default::default() };
        let mut o_id: &'static [u8] = o;
        if let Some(eot) = cstr(o).iter().position(|&b| b == 4) {
            e.ctx.set_bytes(&o[..eot]);
            o_id = &o[eot + 1..];
        }
        let id = cstr(o_id);
        e.id = utf8_prefix(id);
        if id.len() < o_id.len() {
            e.id_plural = Some(utf8_prefix(cstr(&o_id[id.len() + 1..])));
        }
        e.nforms += t.iter().filter(|&&b| b == 0).count() as i32;
        count += 1;
        f(&e);
    }
    count
}

/// The catalog's header (msgid ""), "" if none; and how many plural forms the language has.
pub fn i18n_header(code: &str) -> &'static str {
    let Some(c) = catalog_for(code) else { return "" };
    catalog_find(&c, None, "").and_then(|t| utf8(cstr(t))).unwrap_or("")
}
pub fn i18n_plural_forms(code: &str) -> i32 {
    let i = i18n_find(code);
    if i < 0 {
        0
    } else {
        K_LANGS[i as usize].nforms
    }
}
/// Whether these bytes read as a catalog: every embedded one must, or its language is English.
pub fn i18n_mo_valid(mo: &[u8]) -> bool {
    catalog_open(mo).is_some()
}

/// The embedded catalogs, one per po/<lang>.mo (build/gen/l10n.c; here assets::I18N_CATALOGS, which
/// build.rs writes: a slice, so no NULL entry ends it).
#[derive(Clone, Copy, Debug)]
pub struct I18nCatalog {
    /// the file's name: "es" for po/es.mo
    pub code: &'static str,
    /// the file's bytes (the C's mo and size)
    pub mo: &'static [u8],
}
pub use crate::assets::I18N_CATALOGS as i18n_catalogs;

#[cfg(test)]
mod tests {
    use super::*;

    // (the language is thread state: every test names the one it wants and ends in English)
    fn english() {
        i18n_set("en");
    }

    #[test]
    fn lookups_contexts_and_plurals() {
        assert!(i18n_set("es") && i18n_current() == "es");
        assert_eq!(tr("Done"), "Listo");
        assert_eq!(tr("Cancel"), "Cancelar");
        assert_eq!(tr("This is not in any catalog"), "This is not in any catalog");
        assert_eq!(tr(""), "");
        // A context tells two meanings apart: "key" is not the plain lookup.
        assert_eq!(c_("key", "space"), "espacio");
        assert_eq!(c_("nope", "Done"), "Done");
        assert_eq!(c_("paper", "Blank"), "En blanco");
        // The plural form of a count, in the language's rule.
        assert_eq!(ng_("%d page", "%d pages", 1), "%d p\u{e1}gina");
        assert_eq!(ng_("%d page", "%d pages", 0), "%d p\u{e1}ginas");
        assert_eq!(ng_("%d page", "%d pages", 2), "%d p\u{e1}ginas");
        assert_eq!(ng_("%d thing", "%d things", 1), "%d thing");
        assert_eq!(ng_("%d thing", "%d things", 3), "%d things");
        assert_eq!(i18n_getn_many("%d page", "%d pages", 1), ng_("%d page", "%d pages", 1));
        english();
        assert_eq!(tr("Done"), "Done");
        assert_eq!(ng_("%d page", "%d pages", 1), "%d page");
        assert_eq!(ng_("%d page", "%d pages", 2), "%d pages");
    }

    #[test]
    fn setting_the_language() {
        assert!(i18n_set("es_MX.UTF-8") && i18n_current() == "es");
        assert!(!i18n_set("fr_FR.UTF-8") && i18n_current() == "en" && tr("Done") == "Done");
        assert_eq!(i18n_language_count(), 2);
        assert_eq!((i18n_language_code(1), i18n_language_name(1)), (Some("es"), Some("Espa\u{f1}ol")));
        assert_eq!((i18n_language_code(2), i18n_language_name(-1)), (None, None));
        assert!(i18n_setting_known("auto") && i18n_setting_known("es") && !i18n_setting_known("fr") && !i18n_setting_known(""));
        assert_eq!((i18n_plural_forms("es"), i18n_plural_forms("fr")), (2, 0));
        english();
    }

    #[test]
    fn every_embedded_catalog_reads() {
        for k in i18n_catalogs.iter() {
            assert!(i18n_mo_valid(k.mo), "{}", k.code);
            let mut n = 0;
            let mut prev: Option<Vec<u8>> = None;
            let count = i18n_walk(k.code, &mut |e| {
                // Sorted byte by byte as "context\004English", as the binary search needs.
                let mut key = Vec::new();
                if !e.ctx.is_empty() {
                    key.extend_from_slice(e.ctx.as_bytes());
                    key.push(4);
                }
                key.extend_from_slice(e.id.as_bytes());
                assert!(prev.as_ref().is_none_or(|p| *p < key), "{}: {:?}", k.code, e.id);
                prev = Some(key);
                assert_eq!(e.nforms as usize, e.str.split('\0').count());
                n += 1;
            });
            assert_eq!(count, n);
            assert!(count > 100);
            assert!(i18n_header(k.code).contains("Plural-Forms"), "{}", k.code);
        }
        assert_eq!(i18n_walk("fr", &mut |_| {}), -1);
        assert_eq!(i18n_header("fr"), "");
    }

    #[test]
    fn a_damaged_catalog_is_not_one() {
        let es = i18n_catalogs.iter().find(|k| k.code == "es").unwrap().mo;
        assert!(!i18n_mo_valid(&[]));
        assert!(!i18n_mo_valid(&es[..27])); // shorter than the header
        assert!(!i18n_mo_valid(&es[..es.len() - 1])); // the last string runs off the end
        let mut m = es.to_vec();
        m[0] ^= 1; // not the magic number
        assert!(!i18n_mo_valid(&m));
        let mut m = es.to_vec();
        m[6] = 2; // major revision 2
        assert!(!i18n_mo_valid(&m));
        // Written big-endian, it reads the same.
        let mut be = es.to_vec();
        let (n, orig, trans) = {
            let rd = |o: usize| u32::from_le_bytes(es[o..o + 4].try_into().unwrap()) as usize;
            (rd(8), rd(12), rd(16))
        };
        for o in (0..28).step_by(4) {
            be[o..o + 4].reverse();
        }
        for table in [orig, trans] {
            for i in 0..n * 2 {
                let o = table + i * 4;
                be[o..o + 4].reverse();
            }
        }
        assert!(i18n_mo_valid(&be));
    }

    #[test]
    fn finding_a_language() {
        assert_eq!(i18n_find("es_MX.utf8"), 1);
        assert_eq!(i18n_find("es_ES.UTF-8"), 1);
        assert_eq!(i18n_find("es@euro"), 1);
        assert_eq!(i18n_find("es-419"), 1);
        assert_eq!(i18n_find("ES"), 1);
        assert_eq!(i18n_find("en_US.UTF-8"), 0);
        assert_eq!(i18n_find("en"), 0);
        assert_eq!((i18n_find(""), i18n_find("C"), i18n_find("POSIX"), i18n_find("C.UTF-8")), (-1, -1, -1, -1));
        assert_eq!(i18n_find("fr_FR"), -1);
    }

    #[test]
    fn the_kindles_locale_file() {
        let lang = |text: &str| {
            let mut v = String::new();
            i18n_locale_file_lang(text, &mut v, 64);
            v
        };
        assert_eq!(lang("LANG=es_ES.UTF-8\nLC_ALL=en_US.UTF-8\n"), "es_ES.UTF-8");
        assert_eq!(lang("LC_ALL=ru_RU.utf8\r\nLANG=ru_RU.utf8\r\n"), "ru_RU.utf8");
        assert_eq!(lang("# langpicker\nexport LANG=\"es_MX.utf8\"\n"), "es_MX.utf8");
        assert_eq!(lang("LC_ALL=es_AR.UTF-8\n"), "es_AR.UTF-8"); // no LANG= line: LC_ALL=
        assert_eq!(lang("nothing here"), "");
        assert_eq!(lang(""), "");
        let mut v = String::from("old");
        i18n_locale_file_lang("LANG=es\n", &mut v, 0);
        assert_eq!(v, "old"); // no room: untouched
        i18n_locale_file_lang("LANG=espanol\n", &mut v, 4);
        assert_eq!(v, "esp");
    }

    #[test]
    fn choosing_from_the_environment() {
        for k in ["GRIMOIRE_LANG", "LC_ALL", "LC_MESSAGES", "LANG"] {
            std::env::remove_var(k);
        }
        let mut why = String::new();
        assert_eq!(i18n_detect_from(None, &mut why, 200), 0);
        assert_eq!(why, "no language set");
        std::env::set_var("LANG", "es_ES.UTF-8");
        assert_eq!(i18n_detect_from(None, &mut why, 200), 1);
        assert_eq!(why, "from LANG=es_ES.UTF-8");
        std::env::set_var("LC_MESSAGES", "en_GB.UTF-8"); // LC_MESSAGES before LANG
        assert_eq!(i18n_detect_from(None, &mut why, 200), 0);
        std::env::set_var("LC_ALL", "es_MX.UTF-8"); // LC_ALL before both
        assert_eq!(i18n_detect_from(None, &mut why, 200), 1);
        assert_eq!(why, "from LC_ALL=es_MX.UTF-8");
        // The Kindle's own language ranks above what a launcher exports (KOReader sets
        // LC_ALL=en_US.UTF-8 to what it starts).
        std::env::set_var("LC_ALL", "en_US.UTF-8");
        assert_eq!(i18n_detect_from(Some("LANG=es_ES.UTF-8\nLC_ALL=es_ES.UTF-8\n"), &mut why, 200), 1);
        assert!(why.contains("Kindle's locale es_ES.UTF-8"), "{why}");
        // GRIMOIRE_LANG above all.
        std::env::set_var("GRIMOIRE_LANG", "en");
        assert_eq!(i18n_detect_from(Some("LANG=es_ES.UTF-8\n"), &mut why, 200), 0);
        assert!(why.contains("GRIMOIRE_LANG=en"), "{why}");
        // A language Grimoire does not speak decides too, and means English.
        std::env::set_var("GRIMOIRE_LANG", "fr_FR");
        assert_eq!(i18n_detect_from(None, &mut why, 200), 0);
        assert_eq!(why, "from GRIMOIRE_LANG=fr_FR, not a language Grimoire speaks");
        for k in ["GRIMOIRE_LANG", "LC_ALL", "LC_MESSAGES", "LANG"] {
            std::env::remove_var(k);
        }
    }

    #[test]
    fn copying_between_characters() {
        let mut d = String::new();
        assert_eq!(i18n_copy(&mut d, "hello", 8), 5);
        assert_eq!(d, "hello");
        assert_eq!(i18n_copy(&mut d, "h\u{e9}llo", 3), 1); // the e acute would be cut: left out
        assert_eq!(d, "h");
        assert_eq!(i18n_copy(&mut d, "\u{20ac}\u{20ac}", 5), 3);
        assert_eq!(d, "\u{20ac}");
        let mut d = String::from("untouched");
        assert_eq!(i18n_copy(&mut d, "x", 0), 0);
        assert_eq!(d, "untouched");
    }
}
