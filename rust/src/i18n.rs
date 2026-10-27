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
pub const fn nc_(ctx: &str, s: &'static str) -> &'static str {
    s
}
/// NG_(): the plural form for n, looked up now.
pub fn ng_(one: &'static str, many: &'static str, n: i64) -> &'static str {
    i18n_getn(one, many, n)
}

pub fn i18n_get(msgid: &'static str) -> &'static str {
    todo!()
}
pub fn i18n_getc(ctx: &str, msgid: &'static str) -> &'static str {
    todo!()
}
pub fn i18n_getn(one: &'static str, many: &'static str, n: i64) -> &'static str {
    todo!()
}
pub fn i18n_getn_many(one: &'static str, many: &'static str, n: i64) -> &'static str {
    todo!()
}
/// The plural form for n of a string with a context (C_ and NG_ at once), looked up now; for
/// strings the code gets from a table (the share studio's, httpd.c), so it has no macro.
pub fn i18n_getnc(ctx: &str, one: &'static str, many: &'static str, n: i64) -> &'static str {
    todo!()
}
/// Copies src into dst (cap bytes, with the NUL) as strlcpy_q does, but text too long for dst is
/// cut between two characters, never inside one, and so is text snprintf already cut inside one.
/// Returns the bytes copied.
pub fn i18n_copy(dst: &mut String, src: &str, cap: usize) -> usize {
    todo!()
}

/// The languages Grimoire speaks, English first: each one's code ("en", "es") and its name in
/// itself ("English", "Español"), which is never translated.
pub fn i18n_language_count() -> i32 {
    todo!()
}
/// None past the list
pub fn i18n_language_code(i: i32) -> Option<&'static str> {
    todo!()
}
pub fn i18n_language_name(i: i32) -> Option<&'static str> {
    todo!()
}
/// The language a locale name is in, by its language part ("es_MX.UTF-8", "es-419", "ES": es):
/// its index, or -1 for one Grimoire does not speak (and for "", "C" and "POSIX").
pub fn i18n_find(locale: &str) -> i32 {
    todo!()
}
/// Every lookup in that language from now on (a code or a locale name); false, and English, for a
/// language Grimoire does not speak.
pub fn i18n_set(code: &str) -> bool {
    todo!()
}
/// the code of the language in use
pub fn i18n_current() -> &'static str {
    todo!()
}

/// settings.ini's language=: "auto", or a code from the list. Any other value (a language a newer
/// Grimoire speaks) acts as auto, and store.c keeps it in the file.
pub fn i18n_setting_known(value: &str) -> bool {
    todo!()
}
/// What auto stands for, found once at startup: GRIMOIRE_LANG if it is set (nothing on the Kindle
/// sets it; a launcher may), then on the Kindle the language of its own Settings (the LANG= line
/// of /var/local/system/locale), ranked above any LC_ALL or LANG Grimoire inherits (KOReader
/// exports LC_ALL=en_US.UTF-8 to what it starts), then LC_ALL, LC_MESSAGES, LANG. The first one
/// set decides; a language Grimoire does not speak means English.
pub fn i18n_detect() {
    todo!()
}
/// The same choice from what was read: the text of the Kindle's locale file (NULL where there is
/// none) and the environment. The language's index; why: where it came from, for the log.
pub fn i18n_detect_from(kindle_locale: Option<&str>, why: &mut String, cap: usize) -> i32 {
    todo!()
}
/// The value of the LANG= line (else the LC_ALL= line) of a locale file's text ("" if none).
pub fn i18n_locale_file_lang(text: &str, out: &mut String, cap: usize) {
    todo!()
}
/// the code auto stands for (English before i18n_detect)
pub fn i18n_auto() -> &'static str {
    todo!()
}
/// Sets the language for a language= value, and logs which one and why.
pub fn i18n_apply(setting: &str) {
    todo!()
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
/// Calls f for every entry but the header; returns how many, or -1 if code has no valid catalog.
pub fn i18n_walk(code: &str, f: &mut dyn FnMut(&I18nEntry)) -> i32 {
    todo!()
}
/// The catalog's header (msgid ""), "" if none; and how many plural forms the language has.
pub fn i18n_header(code: &str) -> &'static str {
    todo!()
}
pub fn i18n_plural_forms(code: &str) -> i32 {
    todo!()
}
/// Whether these bytes read as a catalog: every embedded one must, or its language is English.
pub fn i18n_mo_valid(mo: &[u8]) -> bool {
    todo!()
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
