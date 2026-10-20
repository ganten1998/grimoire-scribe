// i18n.h: user-visible text goes through these, so it can be translated. Wrap whole sentences,
// with %s for paths and values; never wrap identifiers (settings keys, file names, log lines, API
// fields). English stays in the code; each language is po/<lang>.po (for translators), compiled
// to po/<lang>.mo, which is committed (building needs no gettext) and embedded in the binary like
// the fonts (build/gen/l10n.c). docs/FORMAT.md has the language setting; the Makefile's l10n and
// l10n-check rules have the rest.
//   _("Saved")                       a string shown to the user, looked up now
//   N_("Pencil")                     marks a string for translation in a static table (no lookup:
//                                    whoever draws it looks it up with _())
//   C_("paper", "Blank")             like _(), with a context telling two meanings apart
//   NC_("paper", "Blank")            marks it with that context in a static table (look it up
//                                    with C_() and the same context)
//   NG_("%d page", "%d pages", n)    the plural form for n, looked up now
// A lookup the language has no translation for gives the English. _(), C_() and NG_() are calls
// that return a const char *, so they cannot initialize a static table: mark its strings with
// N_() or NC_() and look each up where it is drawn. The lookups are declared format_arg, so the
// compiler checks every printf against the English text (NG_ against both forms); msgfmt --check
// (make l10n, make l10n-check) and tests/test_i18n.c check each translation against its English.
// A translation can be longer than the English a buffer was sized for: copy one with i18n_copy.
#pragma once
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#define _(s) i18n_get(s)
#define N_(s) (s)
#define C_(ctx, s) i18n_getc(ctx, s)
#define NC_(ctx, s) (s)
// Both branches make the same lookup. The conditional is for the compiler: clang heeds one
// format_arg per function, but checks a printf against both branches of a ?:, so both English
// forms are checked. (i18n_ng is never 0; being extern, it keeps the ?: from being folded away.
// n is evaluated once.)
#define NG_(one, many, n) (i18n_ng ? i18n_getn(one, many, n) : i18n_getn_many(one, many, n))

const char *i18n_get(const char *msgid) __attribute__((format_arg(1)));
const char *i18n_getc(const char *ctx, const char *msgid) __attribute__((format_arg(2)));
const char *i18n_getn(const char *one, const char *many, long n) __attribute__((format_arg(1)));
const char *i18n_getn_many(const char *one, const char *many, long n) __attribute__((format_arg(2)));
extern const int i18n_ng;
// The plural form for n of a string with a context (C_ and NG_ at once), looked up now; for
// strings the code gets from a table (the share studio's, httpd.c), so it has no macro.
const char *i18n_getnc(const char *ctx, const char *one, const char *many, long n) __attribute__((format_arg(2)));
// Copies src into dst (cap bytes, with the NUL) as strlcpy_q does, but text too long for dst is
// cut between two characters, never inside one, and so is text snprintf already cut inside one.
// Returns the bytes copied.
size_t i18n_copy(char *dst, const char *src, size_t cap);

// The languages Grimoire speaks, English first: each one's code ("en", "es") and its name in
// itself ("English", "Español"), which is never translated.
int i18n_language_count(void);
const char *i18n_language_code(int i);   // NULL past the list
const char *i18n_language_name(int i);
// The language a locale name is in, by its language part ("es_MX.UTF-8", "es-419", "ES": es):
// its index, or -1 for one Grimoire does not speak (and for "", "C" and "POSIX").
int i18n_find(const char *locale);
// Every lookup in that language from now on (a code or a locale name); false, and English, for a
// language Grimoire does not speak.
bool i18n_set(const char *code);
const char *i18n_current(void);          // the code of the language in use

// settings.ini's language=: "auto", or a code from the list. Any other value (a language a newer
// Grimoire speaks) acts as auto, and store.c keeps it in the file.
bool i18n_setting_known(const char *value);
// What auto stands for, found once at startup: GRIMOIRE_LANG if it is set (nothing on the Kindle
// sets it; a launcher may), then on the Kindle the language of its own Settings (the LANG= line
// of /var/local/system/locale), ranked above any LC_ALL or LANG Grimoire inherits (KOReader
// exports LC_ALL=en_US.UTF-8 to what it starts), then LC_ALL, LC_MESSAGES, LANG. The first one
// set decides; a language Grimoire does not speak means English.
void i18n_detect(void);
// The same choice from what was read: the text of the Kindle's locale file (NULL where there is
// none) and the environment. The language's index; why: where it came from, for the log.
int i18n_detect_from(const char *kindle_locale, char *why, size_t cap);
// The value of the LANG= line (else the LC_ALL= line) of a locale file's text ("" if none).
void i18n_locale_file_lang(const char *text, char *out, size_t cap);
const char *i18n_auto(void);             // the code auto stands for (English before i18n_detect)
// Sets the language for a language= value, and logs which one and why.
void i18n_apply(const char *setting);

// Dates in the language in use (util.c): "Oct 10, 14:32" (format_time_short, common.h), and the
// date alone, "Oct 10", as in a new notebook's name.
void format_date_short(int64_t t, char *buf, size_t cap);

// Each entry of the catalog for code, in catalog order (sorted by context and English): for
// tests, and for the share studio, which gets the words of its context (httpd.c).
typedef struct {
    char ctx[64];                 // "" for none
    const char *id, *id_plural;   // the English, and its plural form (NULL for none)
    const char *str;              // the translation; a plural's forms follow one another, each
    int nforms;                   // ending in its NUL: nforms of them (1 for no plural)
} I18nEntry;
// Calls f for every entry but the header; returns how many, or -1 if code has no valid catalog.
int i18n_walk(const char *code, void (*f)(const I18nEntry *e, void *arg), void *arg);
// The catalog's header (msgid ""), "" if none; and how many plural forms the language has.
const char *i18n_header(const char *code);
int i18n_plural_forms(const char *code);
// Whether these bytes read as a catalog: every embedded one must, or its language is English.
bool i18n_mo_valid(const unsigned char *mo, unsigned int size);

// The embedded catalogs, one per po/<lang>.mo, ended by a NULL code (build/gen/l10n.c).
typedef struct {
    const char *code;             // the file's name: "es" for po/es.mo
    const unsigned char *mo;
    unsigned int size;
} I18nCatalog;
extern const I18nCatalog i18n_catalogs[];
