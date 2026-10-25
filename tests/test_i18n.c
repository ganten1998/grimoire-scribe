// test_i18n.c: the languages. Lookups, contexts and plurals in the embedded catalogs; every
// translation checked against its English (the same printf arguments, glyphs every face has);
// the choice of language (settings.ini, the Kindle's locale file, the environment); dates; and
// the widths of text the every-screen scenario (tests/scenarios/y_all_screens.qs, which make
// l10n-check runs in every language) cannot reach: rare notices, and text drawn without a fit.
#include <stdint.h>
#include <time.h>
#include <unistd.h>

#include "app.h"
#include "store.h"
#include "ui.h"
#include "testlib.h"

static char root[600];

// ---- printf formats ----

// The arguments a printf format reads, by position (1-based): 'i' an int (d i u o x X c, and
// the shorter ones it promotes), 'l' long, 'q' long long, 'z' size_t, 'j' intmax_t, 't' ptrdiff_t,
// 'f' double, 'F' long double, 's' a string, 'p' a pointer. Returns how many, or -1 for a format
// printf cannot use safely: %n, a broken directive, numbered and unnumbered ones mixed, the same
// argument read as two types, or a gap among numbered ones (musl's printf then prints nothing).
static int fmt_args(const char *f, char types[17]) {
    memset(types, 0, 17);
    int seq = 0, max = 0;
    bool numbered = false, plain = false;
    for (const char *p = f; *p; p++) {
        if (*p != '%') continue;
        if (*++p == '%') continue;
        int num = 0;
        const char *q = p;
        while (*q >= '0' && *q <= '9') num = num * 10 + (*q++ - '0');
        int pos = 0;
        if (*q == '$' && num > 0) {
            pos = num;
            p = q + 1;
            numbered = true;
        } else {
            plain = true;
        }
        while (*p && strchr("-+ #0'", *p)) p++;
        for (int part = 0; part < 2; part++) {  // the width, then the precision
            if (part == 1) {
                if (*p != '.') break;
                p++;
            }
            if (*p == '*') {  // an int argument holds it
                p++;
                int sn = 0;
                const char *r = p;
                while (*r >= '0' && *r <= '9') sn = sn * 10 + (*r++ - '0');
                int at = *r == '$' && sn > 0 ? sn : ++seq;
                if (*r == '$' && sn > 0) p = r + 1;
                if (at > 16 || (types[at] && types[at] != 'i')) return -1;
                types[at] = 'i';
                if (at > max) max = at;
            } else {
                while (*p >= '0' && *p <= '9') p++;
            }
        }
        char len = 0;
        if (*p == 'h') {
            p += p[1] == 'h' ? 2 : 1;
        } else if (*p == 'l') {
            len = p[1] == 'l' ? 'q' : 'l';
            p += p[1] == 'l' ? 2 : 1;
        } else if (*p && strchr("zjtL", *p)) {
            len = *p++;
        }
        char t;
        if (*p && strchr("diuoxXc", *p)) t = len && len != 'L' ? len : 'i';
        else if (*p && strchr("fFeEgGaA", *p)) t = len == 'L' ? 'F' : 'f';
        else if (*p == 's') t = 's';
        else if (*p == 'p') t = 'p';
        else return -1;
        int at = pos ? pos : ++seq;
        if (at > 16 || (types[at] && types[at] != t)) return -1;
        types[at] = t;
        if (at > max) max = at;
    }
    if (numbered && plain) return -1;
    for (int i = 1; i <= max; i++)
        if (!types[i]) return -1;
    return max;
}

TEST(format_reader) {
    char t[17];
    CHECK(fmt_args("Saved", t) == 0);
    CHECK(fmt_args("%d pages, %s", t) == 2 && t[1] == 'i' && t[2] == 's');
    CHECK(fmt_args("%2$d %1$s, %3$02d:%4$02d", t) == 4 && t[1] == 's' && t[2] == 'i' && t[4] == 'i');
    CHECK(fmt_args("%d%%   tap to reset", t) == 1 && t[1] == 'i');
    CHECK(fmt_args("+%.0f%%", t) == 1 && t[1] == 'f');
    CHECK(fmt_args("%.*s and %zu, %lld", t) == 4 && t[1] == 'i' && t[2] == 's' && t[3] == 'z' && t[4] == 'q');
    CHECK(fmt_args("%2$s", t) == -1);         // argument 1 never read
    CHECK(fmt_args("%1$d %s", t) == -1);      // numbered and not
    CHECK(fmt_args("%n", t) == -1);
    CHECK(fmt_args("100%", t) == -1);         // a lone percent sign
    CHECK(fmt_args("%1$d %1$s", t) == -1);    // one argument, two types
}

// Whether a translation reads exactly the arguments its English does, with the same types.
static bool fmt_compatible(const char *english, const char *translated) {
    char a[17], b[17];
    int na = fmt_args(english, a), nb = fmt_args(translated, b);
    return na >= 0 && nb == na && !memcmp(a, b, sizeof a);
}

TEST(format_compatibility) {
    CHECK(fmt_compatible("%s %d, %02d:%02d", "%2$d %1$s, %3$02d:%4$02d"));
    CHECK(fmt_compatible("%d pages", "%d p\xc3\xa1ginas"));
    CHECK(!fmt_compatible("%d pages", "%s p\xc3\xa1ginas"));
    CHECK(!fmt_compatible("%d pages, %s", "%d p\xc3\xa1ginas"));      // drops an argument
    CHECK(!fmt_compatible("Saved", "Guardado %s"));                    // reads one the code never passes
    CHECK(!fmt_compatible("%s %d", "%2$d %1$d"));
}

// ---- the catalogs ----

static int g_bad;
static const char *g_code;

static void each_entry_formats(const I18nEntry *e, void *arg) {
    (void)arg;
    const char *s = e->str;
    for (int k = 0; k < e->nforms; k++, s += strlen(s) + 1) {
        const char *english = k == 0 || !e->id_plural ? e->id : e->id_plural;
        if (!fmt_compatible(english, s)) {
            fprintf(stderr, "  %s: \"%s\" does not read the arguments of \"%s\"\n", g_code, s, english);
            g_bad++;
        }
    }
    if (e->id_plural && !fmt_compatible(e->id, e->id_plural)) {
        fprintf(stderr, "  %s: \"%s\" and its plural \"%s\" read different arguments\n", g_code, e->id, e->id_plural);
        g_bad++;
    }
}

// The next code point of valid UTF-8 (overlong forms, surrogates and stray bytes are not); 0 at
// the end, -1 for broken text.
static long next_cp(const char **s) {
    const unsigned char *p = (const unsigned char *)*s;
    if (!*p) return 0;
    int len = p[0] < 0x80 ? 1 : p[0] >= 0xC2 && p[0] < 0xE0 ? 2 : p[0] >= 0xE0 && p[0] < 0xF0 ? 3 : p[0] >= 0xF0 && p[0] < 0xF5 ? 4 : 0;
    if (!len) return -1;
    long c = len == 1 ? p[0] : p[0] & (0x7F >> len);
    for (int i = 1; i < len; i++) {
        if ((p[i] & 0xC0) != 0x80) return -1;
        c = c << 6 | (p[i] & 0x3F);
    }
    static const long least[5] = {0, 0, 0x80, 0x800, 0x10000};
    if (c < least[len] || (c >= 0xD800 && c <= 0xDFFF) || c > 0x10FFFF) return -1;
    *s += len;
    return c;
}

// Every face text can be drawn in: Gothic's Jersey 10 (text) and Jacquard 24 (titles), Classic's
// Terminess and its bold. A glyph a pixel face lacks would come from Terminess, smaller and in
// another style: a translation must not need that.
static int missing_glyphs(const char *s) {
    int missing = 0;
    for (;;) {
        long cp = next_cp(&s);
        if (cp == 0) return missing;
        if (cp < 0 || cp < 0x20) return missing + 1000;  // broken UTF-8, or a control character
        ui_set_theme(THEME_GOTHIC);
        bool ok = font_has_glyph(FONT_REGULAR, (uint32_t)cp) && font_has_glyph(FONT_TITLE, (uint32_t)cp);
        ui_set_theme(THEME_CLASSIC);
        ok = ok && font_has_glyph(FONT_REGULAR, (uint32_t)cp) && font_has_glyph(FONT_BOLD, (uint32_t)cp);
        ui_set_theme(THEME_GOTHIC);
        if (!ok) {
            fprintf(stderr, "  U+%04lX is missing from a face\n", cp);
            missing++;
        }
    }
}

static void each_entry_glyphs(const I18nEntry *e, void *arg) {
    (void)arg;
    const char *s = e->str;
    for (int k = 0; k < e->nforms; k++, s += strlen(s) + 1)
        if (missing_glyphs(s)) {
            fprintf(stderr, "  %s: \"%s\" cannot be drawn in every face\n", g_code, s);
            g_bad++;
        }
}

static void count_entry(const I18nEntry *e, void *arg) {
    int *forms = arg;
    if (e->id_plural && e->nforms != forms[0]) {
        fprintf(stderr, "  %s: \"%s\" has %d plural forms, the language %d\n", g_code, e->id, e->nforms, forms[0]);
        g_bad++;
    }
}

// The Plural-Forms line each language's catalog must carry: src/i18n.c has the same rule by hand.
static const struct {
    const char *code, *plural_forms;
} k_rules[] = {
    {"es", "Plural-Forms: nplurals=2; plural=(n != 1);"},
};

TEST(catalogs_hold_together) {
    // Every catalog is for a language of the list (English needs none), and reads.
    int ncat = 0;
    for (const I18nCatalog *k = i18n_catalogs; k->code; k++, ncat++) {
        CHECK(i18n_find(k->code) > 0 && !strcmp(i18n_language_code(i18n_find(k->code)), k->code));
        CHECK(i18n_mo_valid(k->mo, k->size));
    }
    CHECK(ncat == i18n_language_count() - 1);
    CHECK(!strcmp(i18n_language_code(0), "en") && i18n_language_code(i18n_language_count()) == NULL);
    for (int i = 1; i < i18n_language_count(); i++) {
        g_code = i18n_language_code(i);
        g_bad = 0;
        int n = i18n_walk(g_code, each_entry_formats, NULL);
        CHECK(n > 0);
        i18n_walk(g_code, each_entry_glyphs, NULL);
        int forms[1] = {i18n_plural_forms(g_code)};
        i18n_walk(g_code, count_entry, forms);
        CHECK(g_bad == 0);
        // Its plural rule is the one written in src/i18n.c.
        const char *rule = NULL;
        for (size_t r = 0; r < sizeof k_rules / sizeof k_rules[0]; r++)
            if (!strcmp(k_rules[r].code, g_code)) rule = k_rules[r].plural_forms;
        CHECK(rule && strstr(i18n_header(g_code), rule));
        if (!rule) fprintf(stderr, "  %s: add its Plural-Forms to k_rules here, and its rule to src/i18n.c\n", g_code);
        // The language's own name is in every face too.
        CHECK(missing_glyphs(i18n_language_name(i)) == 0);
    }
}

TEST(lookups) {
    CHECK(i18n_set("en") && !strcmp(i18n_current(), "en"));
    const char *lang = "Language";
    CHECK(_("Language") == lang || !strcmp(_("Language"), "Language"));
    CHECK(i18n_set("es") && !strcmp(i18n_current(), "es"));
    CHECK(!strcmp(_("Language"), "Idioma"));
    CHECK(!strcmp(_("Language: %s"), "Idioma: %s"));
    // English for what a catalog lacks: the very string the code passed.
    const char *none = "A sentence no catalog has";
    CHECK(_(none) == none);
    CHECK(_("") != NULL && !strcmp(_(""), ""));  // never the catalog's header
    // Contexts: the same English in two contexts are two strings; a context it lacks is English.
    CHECK(!strcmp(C_("month", "May"), "may"));
    CHECK(!strcmp(C_("month", "Sep"), "sept"));
    CHECK(!strcmp(C_("nonesuch", "May"), "May"));
    CHECK(!strcmp(_("May"), "May"));
    CHECK(!strcmp(C_("date", "%s %d"), "%2$d %1$s"));
    // A locale name picks its language by the language part; one it lacks is English.
    CHECK(i18n_set("es_MX.UTF-8") && !strcmp(i18n_current(), "es"));
    CHECK(!i18n_set("fr_FR.UTF-8") && !strcmp(i18n_current(), "en") && !strcmp(_("Language"), "Language"));
    i18n_set("en");
}

TEST(plurals) {
    i18n_set("es");
    // Spanish: 1 the one form, 0 and 2 (and every other count) the other.
    CHECK(!strcmp(NG_("%d notebook", "%d notebooks", 0), "%d cuadernos"));
    CHECK(!strcmp(NG_("%d notebook", "%d notebooks", 1), "%d cuaderno"));
    CHECK(!strcmp(NG_("%d notebook", "%d notebooks", 2), "%d cuadernos"));
    CHECK(!strcmp(NG_("%d page, %s", "%d pages, %s", 21), "%d p\xc3\xa1ginas, %s"));
    // Missing: the English form for the count.
    CHECK(!strcmp(NG_("%d frog", "%d frogs", 1), "%d frog") && !strcmp(NG_("%d frog", "%d frogs", 3), "%d frogs"));
    i18n_set("en");
    CHECK(!strcmp(NG_("%d notebook", "%d notebooks", 1), "%d notebook"));
    CHECK(!strcmp(NG_("%d notebook", "%d notebooks", 0), "%d notebooks"));
    CHECK(i18n_plural_forms("en") == 2 && i18n_plural_forms("es") == 2 && i18n_plural_forms("xx") == 0);
}

// A catalog that does not read is refused whole (its language is then English), whatever is wrong.
TEST(damaged_catalogs) {
    const I18nCatalog *es = NULL;
    for (const I18nCatalog *k = i18n_catalogs; k->code; k++)
        if (!strcmp(k->code, "es")) es = k;
    CHECK(es != NULL);
    if (!es) return;
    unsigned n = es->size;
    unsigned char *m = malloc(n);
    memcpy(m, es->mo, n);
    CHECK(i18n_mo_valid(m, n));
    CHECK(!i18n_mo_valid(m, 27));          // shorter than the header
    CHECK(!i18n_mo_valid(m, n - 1));       // the last string runs off the end
    CHECK(!i18n_mo_valid(NULL, 0));
    uint32_t count, orig, trans;
    memcpy(&count, m + 8, 4);
    memcpy(&orig, m + 12, 4);
    memcpy(&trans, m + 16, 4);
    m[0] ^= 1;  // the magic
    CHECK(!i18n_mo_valid(m, n));
    m[0] ^= 1;
    uint32_t v = 0x7FFFFFF0u;  // a table past the end
    memcpy(m + 12, &v, 4);
    CHECK(!i18n_mo_valid(m, n));
    memcpy(m + 12, &orig, 4);
    v = count + 1000000;  // more strings than the file holds
    memcpy(m + 8, &v, 4);
    CHECK(!i18n_mo_valid(m, n));
    memcpy(m + 8, &count, 4);
    // Two English strings swapped: out of order, so a search could miss: refused.
    uint8_t e1[8], e2[8];
    memcpy(e1, m + orig + 8, 8);
    memcpy(e2, m + orig + 16, 8);
    memcpy(m + orig + 8, e2, 8);
    memcpy(m + orig + 16, e1, 8);
    CHECK(!i18n_mo_valid(m, n));
    memcpy(m + orig + 8, e1, 8);
    memcpy(m + orig + 16, e2, 8);
    CHECK(i18n_mo_valid(m, n));
    // A string without its terminating NUL.
    uint32_t len0, off0;
    memcpy(&len0, m + trans + 8, 4);
    memcpy(&off0, m + trans + 12, 4);
    m[off0 + len0] = 'x';
    CHECK(!i18n_mo_valid(m, n));
    m[off0 + len0] = 0;
    // The same catalog written big-endian reads too (msgfmt on such a machine).
    unsigned char *be = malloc(n);
    memcpy(be, m, n);
    for (unsigned at = 0; at < 28; at += 4) {
        unsigned char t[4] = {be[at + 3], be[at + 2], be[at + 1], be[at]};
        memcpy(be + at, t, 4);
    }
    for (unsigned at = orig; at < orig + count * 8; at += 4) {
        unsigned char t[4] = {m[at + 3], m[at + 2], m[at + 1], m[at]};
        memcpy(be + at, t, 4);
    }
    for (unsigned at = trans; at < trans + count * 8; at += 4) {
        unsigned char t[4] = {m[at + 3], m[at + 2], m[at + 1], m[at]};
        memcpy(be + at, t, 4);
    }
    CHECK(i18n_mo_valid(be, n));
    free(be);
    free(m);
}

// A context with plurals (httpd.c sends the studio its words so): the form for the count, English
// for a language without the string.
TEST(context_plurals) {
    i18n_set("es");
    const char *one = "PNGs, all %d page (zip)", *many = "PNGs, all %d pages (zip)";
    CHECK(!strcmp(i18n_getnc("studio", one, many, 1), "PNG de %d p\xc3\xa1gina (zip)"));
    CHECK(!strcmp(i18n_getnc("studio", one, many, 12), "PNG de las %d p\xc3\xa1ginas (zip)"));
    CHECK(i18n_getnc(NULL, one, many, 12) == many);       // no such string outside its context
    CHECK(i18n_getnc("nonesuch", one, many, 1) == one);
    i18n_set("en");
    CHECK(i18n_getnc("studio", one, many, 1) == one && i18n_getnc("studio", one, many, 2) == many);
}

// The names of papers, tools and music parts are identifiers (/api/list.json, grimoire.log): English
// in every language. Their labels, which the screens show, are in the language in use, and every
// language has each of them.
TEST(labels_and_identifiers) {
    for (int l = 0; l < i18n_language_count(); l++) {
        i18n_set(i18n_language_code(l));
        bool en = l == 0;
        CHECK(!strcmp(paper_name(PAPER_NARROW), "Narrow rule") && !strcmp(tool_name(TOOL_PENCIL), "Pencil"));
        CHECK(!strcmp(music_part_name(PART_TREBLE), "Treble staff"));
        for (int i = 0; i < PAPER_COUNT; i++) {
            PaperKind k = paper_at(i);
            CHECK(en ? paper_label(k) == paper_name(k) : paper_label(k) != paper_name(k));
            CHECK(missing_glyphs(paper_label(k)) == 0);
        }
        PaperKind custom = music_default(), newer = (PaperKind)0x50000001u;
        CHECK(en ? paper_label(custom) == paper_name(custom) : paper_label(custom) != paper_name(custom));
        CHECK(en ? paper_label(newer) == paper_name(newer) : paper_label(newer) != paper_name(newer));
        for (int t = 0; t < TOOL_COUNT; t++) CHECK(en ? tool_label((ToolKind)t) == tool_name((ToolKind)t)
                                                      : tool_label((ToolKind)t) != tool_name((ToolKind)t));
        for (int p = PART_TREBLE; p < PART_COUNT; p++)
            CHECK(en ? part_label((MusicPart)p) == music_part_name((MusicPart)p)
                     : part_label((MusicPart)p) != music_part_name((MusicPart)p));
        CHECK(!strcmp(part_label(PART_NONE), ""));
    }
    i18n_set("es");
    CHECK(!strcmp(paper_label(PAPER_NARROW), "Rayado fino") && !strcmp(tool_label(TOOL_PENCIL), "Grafito"));
    CHECK(!strcmp(part_label(PART_TREBLE), "Clave de sol"));
    i18n_set("en");
}

// ---- choosing the language ----

static void clear_env(void) {
    unsetenv("GRIMOIRE_LANG");
    unsetenv("LC_ALL");
    unsetenv("LC_MESSAGES");
    unsetenv("LANG");
}

TEST(locale_names) {
    CHECK(i18n_find("es") == 1 && i18n_find("ES") == 1 && i18n_find("es-419") == 1);
    CHECK(i18n_find("es_MX.utf8") == 1 && i18n_find("es_ES.UTF-8") == 1 && i18n_find("es@euro") == 1);
    CHECK(i18n_find("en_US.UTF-8") == 0 && i18n_find("en") == 0);
    CHECK(i18n_find("") == -1 && i18n_find("C") == -1 && i18n_find("POSIX") == -1 && i18n_find("C.UTF-8") == -1);
    CHECK(i18n_find("fr_FR") == -1 && i18n_find("esperanto") == -1 && i18n_find("e") == -1 && i18n_find(NULL) == -1);
    char v[32];
    i18n_locale_file_lang("LANG=es_ES.UTF-8\nLC_ALL=es_ES.UTF-8\n", v, sizeof v);
    CHECK(!strcmp(v, "es_ES.UTF-8"));
    i18n_locale_file_lang("LC_ALL=ru_RU.utf8\r\nLANG=ru_RU.utf8\r\n", v, sizeof v);
    CHECK(!strcmp(v, "ru_RU.utf8"));
    i18n_locale_file_lang("# langpicker\nexport LANG=\"es_MX.utf8\"\n", v, sizeof v);
    CHECK(!strcmp(v, "es_MX.utf8"));
    i18n_locale_file_lang("LC_ALL=es_AR.UTF-8\n", v, sizeof v);  // no LANG= line: LC_ALL=
    CHECK(!strcmp(v, "es_AR.UTF-8"));
    i18n_locale_file_lang("LANGUAGE=es\nLANG=\n", v, sizeof v);
    CHECK(!strcmp(v, ""));
    i18n_locale_file_lang(NULL, v, sizeof v);
    CHECK(!strcmp(v, ""));
}

TEST(detection_order) {
    char why[128];
    clear_env();
    CHECK(i18n_detect_from(NULL, why, sizeof why) == 0 && strstr(why, "no language"));
    setenv("LANG", "es_ES.UTF-8", 1);
    CHECK(i18n_detect_from(NULL, why, sizeof why) == 1 && strstr(why, "LANG=es_ES.UTF-8"));
    setenv("LC_MESSAGES", "en_GB.UTF-8", 1);  // LC_MESSAGES before LANG
    CHECK(i18n_detect_from(NULL, why, sizeof why) == 0);
    setenv("LC_ALL", "es_MX.UTF-8", 1);       // LC_ALL before both
    CHECK(i18n_detect_from(NULL, why, sizeof why) == 1 && strstr(why, "LC_ALL"));
    // On the Kindle its own Settings' language ranks above what Grimoire inherits: KOReader's
    // launcher exports LC_ALL=en_US.UTF-8 to everything it starts.
    setenv("LC_ALL", "en_US.UTF-8", 1);
    CHECK(i18n_detect_from("LANG=es_ES.UTF-8\nLC_ALL=es_ES.UTF-8\n", why, sizeof why) == 1);
    CHECK(strstr(why, "Kindle's locale es_ES.UTF-8") != NULL);
    // GRIMOIRE_LANG ranks first, even there.
    setenv("GRIMOIRE_LANG", "en", 1);
    CHECK(i18n_detect_from("LANG=es_ES.UTF-8\n", why, sizeof why) == 0 && strstr(why, "GRIMOIRE_LANG=en"));
    setenv("GRIMOIRE_LANG", "", 1);  // set but empty: not set
    CHECK(i18n_detect_from("LANG=es_ES.UTF-8\n", why, sizeof why) == 1);
    // The first one set decides, also for a language Grimoire does not speak: English, said so.
    clear_env();
    setenv("LANG", "es_ES.UTF-8", 1);
    CHECK(i18n_detect_from("LANG=ru_RU.utf8\n", why, sizeof why) == 0 && strstr(why, "not a language"));
    setenv("LANG", "C", 1);
    CHECK(i18n_detect_from(NULL, why, sizeof why) == 0);
    // An empty or unreadable locale file counts as none.
    setenv("LANG", "es_CO.UTF-8", 1);
    CHECK(i18n_detect_from("", why, sizeof why) == 1);
    // The log line names no more than the locale: printable ASCII, cut short.
    setenv("LANG", "es_ES\"\x01\xc3\xb1xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx", 1);
    CHECK(i18n_detect_from(NULL, why, sizeof why) == 1 && !strchr(why, '"') && !strchr(why, '\x01') && strlen(why) < 80);
    clear_env();
}

TEST(applying_a_setting) {
    clear_env();
    setenv("GRIMOIRE_LANG", "es", 1);
    i18n_detect();
    CHECK(!strcmp(i18n_auto(), "es"));
    i18n_apply("auto");
    CHECK(!strcmp(i18n_current(), "es"));
    i18n_apply("en");
    CHECK(!strcmp(i18n_current(), "en"));
    i18n_apply("fr");  // a language a newer version speaks: auto
    CHECK(!strcmp(i18n_current(), "es"));
    i18n_apply("");
    CHECK(!strcmp(i18n_current(), "es"));
    CHECK(i18n_setting_known("auto") && i18n_setting_known("en") && i18n_setting_known("es"));
    CHECK(!i18n_setting_known("fr") && !i18n_setting_known("es_ES") && !i18n_setting_known("") && !i18n_setting_known(NULL));
    clear_env();
    i18n_detect();
    CHECK(!strcmp(i18n_auto(), "en"));
    i18n_apply("auto");
}

static char *file_text(const char *path) { return read_file(path, NULL); }
static bool has_line(const char *text, const char *line) {
    size_t n = strlen(line);
    for (const char *p = text; p && *p; p = strchr(p, '\n') ? strchr(p, '\n') + 1 : NULL)
        if (!strncmp(p, line, n) && (p[n] == '\n' || !p[n])) return true;
    return false;
}

// settings.ini language=: auto by default; a language this version lacks acts as auto and goes
// back into the file as it was, until the language is picked; settings_language reads it alone.
TEST(language_setting) {
    char path[700], v[16];
    snprintf(path, sizeof path, "%s/settings.ini", store_root());
    unlink(path);
    Settings s;
    settings_load(&s);
    CHECK(!strcmp(s.language, "auto"));
    settings_language(v, sizeof v);
    CHECK(!strcmp(v, ""));
    CHECK(settings_save(&s) == 0);
    char *t = file_text(path);
    CHECK(t && has_line(t, "language=auto"));
    free(t);
    const char *f = "theme=classic\nlanguage=es\n";
    CHECK(write_file_atomic(path, f, strlen(f)) == 0);
    CHECK(settings_load(&s) == 0 && !strcmp(s.language, "es"));
    settings_language(v, sizeof v);
    CHECK(!strcmp(v, "es"));
    f = "language=fr\r\ntheme=gothic\n";
    CHECK(write_file_atomic(path, f, strlen(f)) == 0);
    CHECK(settings_load(&s) == 0 && !strcmp(s.language, "auto"));
    settings_language(v, sizeof v);
    CHECK(!strcmp(v, "fr"));
    for (int round = 0; round < 2; round++) {  // kept on every save while unchanged
        CHECK(settings_save(&s) == 0);
        t = file_text(path);
        CHECK(t && has_line(t, "language=fr") && !strstr(t, "language=auto"));
        free(t);
        CHECK(settings_load(&s) == 0);
    }
    strlcpy_q(s.language, "es", sizeof s.language);  // picked in Settings
    CHECK(settings_save(&s) == 0);
    strlcpy_q(s.language, "auto", sizeof s.language);  // and then Automatic: saved as set
    CHECK(settings_save(&s) == 0);
    t = file_text(path);
    CHECK(t && has_line(t, "language=auto") && !strstr(t, "language=fr"));
    free(t);
    // The last line counts.
    f = "language=fr\nlanguage=en\n";
    CHECK(write_file_atomic(path, f, strlen(f)) == 0);
    CHECK(settings_load(&s) == 0 && !strcmp(s.language, "en") && settings_save(&s) == 0);
    t = file_text(path);
    CHECK(t && has_line(t, "language=en") && !strstr(t, "fr"));
    free(t);
    settings_language(v, sizeof v);
    CHECK(!strcmp(v, "en"));
    unlink(path);
}

// ---- dates ----

TEST(dates) {
    setenv("TZ", "UTC", 1);
    tzset();
    int64_t t = 1760106720;  // 2025-10-10 14:32 UTC
    char b[64];
    i18n_set("en");
    format_time_short(t, b, sizeof b);
    CHECK(!strcmp(b, "Oct 10, 14:32"));
    format_date_short(t, b, sizeof b);
    CHECK(!strcmp(b, "Oct 10"));
    i18n_set("es");
    format_time_short(t, b, sizeof b);
    CHECK(!strcmp(b, "10 oct, 14:32"));
    format_date_short(t, b, sizeof b);
    CHECK(!strcmp(b, "10 oct"));
    format_time_short(t - 30 * 86400 + 7 * 3600, b, sizeof b);  // 2025-09-10 21:32: the long month
    CHECK(!strcmp(b, "10 sept, 21:32"));
    format_time_short(1736208000, b, sizeof b);  // 2025-01-07 00:00
    CHECK(!strcmp(b, "7 ene, 00:00"));
    i18n_set("en");
    unsetenv("TZ");
    tzset();
}

// ---- text that must fit ----

// Notices no scenario shows (a full store, a damaged page, a missing pen, the passcode left off),
// in every language and style, on the narrow (upright) screen: a toast has two lines. Each is
// formatted as the code formats it (keep this list in step with the messages); out holds them.
enum { RARE_MAX = 32, RARE_LEN = 256 };
static int rare_notices(char out[RARE_MAX][RARE_LEN]) {
    int n = 0;
#define PUT(...) snprintf(out[n++], RARE_LEN, __VA_ARGS__)
    PUT("%s", _("Your Kindle passcode was left off. Turn it back on in Kindle Settings."));
    PUT(_("No pen found. %s is made for the Kindle Scribe; writing needs its pen."), APP_NAME);
    PUT(_("Your Kindle passcode is paused while %s is open."), APP_NAME);
    PUT("%s", _("Could not save this page. Hold power 4 s again to leave anyway."));
    PUT(_("Saved. Leaving %s."), APP_NAME);
    PUT(_("Leaving %s without this page."), APP_NAME);
    PUT(_("PDF saved to %s"), "grimoire/PDFs");
    PUT("%s", _("Could not save the PDF"));
    PUT(_("A damaged page was set aside; a blank one took its place (see %s)"), "grimoire.log");
    PUT("%s", _("Could not create the notebook (storage full?)"));
    PUT("%s", _("Could not save this page. Is the storage full?"));
    PUT("%s", _("This page was made by a newer Grimoire. It is read-only here."));
    PUT("%s", _("Could not open that page"));
    PUT("%s", _("Could not open that notebook"));
    PUT("%s", _("This page is already empty"));
    PUT("%s", _("Could not delete those pages"));
    PUT("%s", _("Could not add a page"));
    PUT("%s", _("Could not copy that page"));
    PUT(_("This page is a copy of page %d."), 2000);
    PUT("%s", _("Could not save the sleep screen"));
    PUT("%s", _("Calibration saved"));
    PUT(_("Moved to %s"), "grimoire/trash");
    PUT(NG_("Deleted %d page (kept in %s)", "Deleted %d pages (kept in %s)", 1), 1, "grimoire/trash");
    PUT(NG_("Deleted %d page (kept in %s)", "Deleted %d pages (kept in %s)", 2000), 2000, "grimoire/trash");
#undef PUT
    return n;
}

TEST(rare_notices_fit) {
    Rect screen = {0, 0, 1860, 2480};
    static char toasts[RARE_MAX][RARE_LEN];
    for (int l = 0; l < i18n_language_count(); l++) {
        i18n_set(i18n_language_code(l));
        int nt = rare_notices(toasts);
        for (int th = THEME_GOTHIC; th <= THEME_CLASSIC; th++) {
            ui_set_theme((UiTheme)th);
            for (int i = 0; i < nt; i++) {
                const char *s = toasts[i];
                int lines = font_wrap_lines(FONT_BOLD, ui_text_size(), s, ui_toast_room(screen), NULL);
                if (lines < 1 || lines > 2) fprintf(stderr, "  %s: \"%s\" needs %d lines\n", i18n_current(), s, lines);
                CHECK(lines >= 1 && lines <= 2);
                CHECK(strlen(s) < sizeof ((App *)0)->toast);  // whole in the toast's buffer
            }
            // The Display sheet's rows for a Kindle without the light (its whole width).
            int room = 1500 - (th == THEME_GOTHIC ? 80 : 72);
            CHECK(font_measure(FONT_REGULAR, ui_text_size(), _("Front light controls are not available here.")) <= room);
            CHECK(font_measure(FONT_REGULAR, ui_text_size(), _("This Kindle has no warmth control.")) <= room);
            // The Share screen without an address (the scenario has one), and asleep: lines across
            // the sheet's text room, the first in bold at the title size.
            int share = 1860 - 2 * 70 - (th == THEME_GOTHIC ? 80 : 72);
            const char *heads[] = {_("Turn on Wi-Fi on the Kindle first"), _("Could not start the server"), _("Asleep")};
            const char *lines[] = {_("Then tap Retry."), _("Is another app using the port? Try again."),
                                   _("Wake the Kindle to share again.")};
            for (int i = 0; i < 3; i++) {
                if (font_measure(FONT_BOLD, ui_title_size(), heads[i]) > share) fprintf(stderr, "  too wide: %s\n", heads[i]);
                CHECK(font_measure(FONT_BOLD, ui_title_size(), heads[i]) <= share);
                CHECK(font_measure(FONT_REGULAR, ui_text_size(), lines[i]) <= share);
            }
            // Its Retry button, under no icon (404 px, at 44 px or 34 px).
            CHECK(font_fit_px(FONT_CAPS, ui_text_size(), ui_small_size(), _("Retry"), 420 - 16) > 0.f);
            // A second address, on its own line in small type (sideways the column is narrower).
            char also[160];
            snprintf(also, sizeof also, _("Also: %s (%s)"), "http://192.168.15.244:8080", "usb0");
            CHECK(font_measure(FONT_REGULAR, ui_small_size(), also) <= 1400);
            // Calibration's notes under what to do (the scenario sees them fit), as calibrate.c
            // formats them: whole in their buffer, which is a toast's size.
            char off[RARE_LEN];
            snprintf(off, sizeof off, _("The center was off by %.1f mm. Let's try again."), 88.8);
            const char *notes[] = {_("Those taps did not make sense. Let's try again."), off, _("Pen done.")};
            for (int i = 0; i < 3; i++) CHECK(strlen(notes[i]) < sizeof ((App *)0)->toast);
            // Settings' Data line with the Kindle's own path: the label fits the sheet (the path is
            // a name, which a longer one is cut short as, unreported: tests/l10n/long_root.qs).
            char data[160];
            snprintf(data, sizeof data, _("Data: %s"), "/mnt/us/grimoire");
            CHECK(font_measure(FONT_REGULAR, ui_small_size(), data) <= (th == THEME_GOTHIC ? 1640 : 1648));
        }
    }
    ui_set_theme(THEME_GOTHIC);
    i18n_set("en");
}

// A translation can be longer than the English a buffer was sized for: i18n_copy cuts it between
// two characters, and takes off one that snprintf already cut.
TEST(copies_cut_between_characters) {
    char b[8];
    CHECK(i18n_copy(b, "a\xc3\xb1o", sizeof b) == 4 && !strcmp(b, "a\xc3\xb1o"));  // "año", whole
    CHECK(i18n_copy(b, "a\xc3\xb1o", 4) == 3 && !strcmp(b, "a\xc3\xb1"));
    CHECK(i18n_copy(b, "a\xc3\xb1o", 3) == 1 && !strcmp(b, "a"));  // not "a" and half an n-tilde
    CHECK(i18n_copy(b, "\xe2\x80\xa6!", 3) == 0 && !strcmp(b, ""));  // a three-byte character
    CHECK(i18n_copy(b, "\xf0\x9f\x98\x80!", 5) == 4 && !strcmp(b, "\xf0\x9f\x98\x80"));
    CHECK(i18n_copy(b, "\xf0\x9f\x98\x80!", 4) == 0 && !strcmp(b, ""));
    CHECK(i18n_copy(b, "ab\xc3", sizeof b) == 2 && !strcmp(b, "ab"));  // cut by snprintf before
    CHECK(i18n_copy(b, "abcdefghij", sizeof b) == 7 && !strcmp(b, "abcdefg"));
    CHECK(i18n_copy(b, "", sizeof b) == 0 && !strcmp(b, ""));
    CHECK(i18n_copy(b, "x", 1) == 0 && !strcmp(b, ""));
    b[0] = 'z';
    CHECK(i18n_copy(b, "x", 0) == 0 && b[0] == 'z');  // no room, not even for the NUL: untouched
}

// Pen feel draws its graph's and pad's labels as they are (font_draw: never cut): under the
// graph and over it, 570 px to the test pad; over the pad, the pad's width (1180 px upright).
TEST(raw_labels_fit) {
    for (int l = 0; l < i18n_language_count(); l++) {
        i18n_set(i18n_language_code(l));
        for (int th = THEME_GOTHIC; th <= THEME_CLASSIC; th++) {
            ui_set_theme((UiTheme)th);
            float ss = ui_small_size();
            CHECK(font_measure(FONT_REGULAR, ss, _("pen pressure")) <= 570);
            CHECK(font_measure(FONT_REGULAR, ss, _("ink")) <= 570);
            CHECK(font_measure(FONT_REGULAR, ss, _("test pad")) <= 1180);
            CHECK(font_measure(FONT_REGULAR, ss, _("test pad (the eraser clears it)")) <= 1180);
            CHECK(font_measure(FONT_REGULAR, ss, _("test pad (white ink: try it on the black)")) <= 1180);
        }
    }
    ui_set_theme(THEME_GOTHIC);
    i18n_set("en");
}

// Paper never changes with the language: the Cornell headings are part of the page, drawn the
// same for everyone (so are PDFs, thumbnails and grimoire-export's renders).
TEST(paper_is_the_same_in_every_language) {
    Canvas a, b;
    CHECK(canvas_init(&a, 1860, 2480) == 0 && canvas_init(&b, 1860, 2480) == 0);
    i18n_set("en");
    paper_render(&a, PAPER_CORNELL, (Rect){0, 0, 1860, 2480}, 300.f);
    for (int l = 1; l < i18n_language_count(); l++) {
        i18n_set(i18n_language_code(l));
        paper_render(&b, PAPER_CORNELL, (Rect){0, 0, 1860, 2480}, 300.f);
        bool same = true;
        for (int y = 0; y < a.h && same; y++) same = !memcmp(a.px + (size_t)y * a.stride, b.px + (size_t)y * b.stride, (size_t)a.w);
        CHECK(same);
    }
    canvas_free(&a);
    canvas_free(&b);
    i18n_set("en");
}

// ---- the layout helpers ----

static int g_hook_calls, g_hook_need, g_hook_have;
static char g_hook_text[256];
static void hook(const char *text, int need, int have) {
    g_hook_calls++;
    g_hook_need = need;
    g_hook_have = have;
    strlcpy_q(g_hook_text, text, sizeof g_hook_text);
}

static bool same_pixels(const Canvas *a, const Canvas *b) {
    for (int y = 0; y < a->h; y++)
        if (memcmp(a->px + (size_t)y * a->stride, b->px + (size_t)y * b->stride, (size_t)a->w)) return false;
    return true;
}

TEST(overflow_hook) {
    Canvas c;
    CHECK(canvas_init(&c, 800, 200) == 0);
    font_set_overflow_hook(hook);
    g_hook_calls = 0;
    font_draw_left(&c, FONT_REGULAR, 44.f, (Rect){0, 0, 800, 100}, "Fits", 0);
    font_draw_centered(&c, FONT_REGULAR, 44.f, (Rect){0, 0, 800, 100}, "Fits", 0);
    CHECK(g_hook_calls == 0);
    const char *longer = "A sentence much too long for a box two hundred pixels wide";
    font_draw_left(&c, FONT_REGULAR, 44.f, (Rect){0, 0, 200, 100}, longer, 0);
    CHECK(g_hook_calls == 1 && !strcmp(g_hook_text, longer) && g_hook_have == 200);
    CHECK(g_hook_need == font_measure(FONT_REGULAR, 44.f, longer));
    font_draw_centered(&c, FONT_BOLD, 44.f, (Rect){0, 0, 200, 100}, longer, 0);
    CHECK(g_hook_calls == 2);
    // A name, which "..." shortens by design: not reported.
    font_draw_left_name(&c, FONT_REGULAR, 44.f, (Rect){0, 0, 200, 100}, longer, 0);
    CHECK(g_hook_calls == 2);
    // Wrapped text that fits its lines is not reported; text that does not is, whole.
    font_draw_wrapped(&c, FONT_REGULAR, 34.f, (Rect){0, 0, 500, 200}, "Two short lines of text here", 2, 0, true, 0);
    CHECK(g_hook_calls == 2);
    font_draw_wrapped(&c, FONT_REGULAR, 34.f, (Rect){0, 0, 300, 200}, longer, 2, 0, true, 0);
    CHECK(g_hook_calls == 3 && !strcmp(g_hook_text, longer) && g_hook_have == 300);
    // A word wider than its line, broken between two of its letters (with no hyphen): reported,
    // with the word's width, though every line fits.
    const char *word = "Notenlinienpapier", *two = "Notenlinienpapier kariert";
    int ww = font_measure(FONT_REGULAR, 34.f, word);
    g_hook_calls = 0;
    font_draw_wrapped(&c, FONT_REGULAR, 34.f, (Rect){0, 0, ww - 20, 200}, two, 4, 0, true, 0);
    CHECK(g_hook_calls == 1 && !strcmp(g_hook_text, two) && g_hook_need == ww && g_hook_have == ww - 20);
    g_hook_calls = 0;
    font_draw_wrapped(&c, FONT_REGULAR, 34.f, (Rect){0, 0, ww, 200}, two, 4, 0, true, 0);  // the word fits
    CHECK(g_hook_calls == 0);
    font_set_overflow_hook(NULL);
    font_draw_left(&c, FONT_REGULAR, 44.f, (Rect){0, 0, 200, 100}, longer, 0);
    CHECK(g_hook_calls == 0);
    canvas_free(&c);
}

TEST(wrapping) {
    ui_set_theme(THEME_GOTHIC);
    float ls = 34.f * 0.9f;
    int w = font_measure(FONT_REGULAR, ls, "Narrow, no margin");
    // Narrower than the label: two lines, broken after the comma (as the Library's tiles did).
    int widest, n = font_wrap_lines(FONT_REGULAR, ls, "Wide, no margin", font_measure(FONT_REGULAR, ls, "Wide, no margin") - 1,
                                    &widest);
    CHECK(n == 2 && widest == font_measure(FONT_REGULAR, ls, "no margin"));
    CHECK(font_wrap_lines(FONT_REGULAR, ls, "Narrow, no margin", w - 1, &widest) == 2);
    CHECK(widest == font_measure(FONT_REGULAR, ls, "no margin"));
    // Balanced: the longer line as short as it can be.
    const char *s = "aaaa bbbb cccc dddd eeee ffff gggg";
    int full = font_measure(FONT_REGULAR, ls, s);
    CHECK(font_wrap_lines(FONT_REGULAR, ls, s, full - 1, &widest) == 2);
    int after3 = font_measure(FONT_REGULAR, ls, "dddd eeee ffff gggg"), after4 = font_measure(FONT_REGULAR, ls, "aaaa bbbb cccc dddd");
    int first3 = font_measure(FONT_REGULAR, ls, "aaaa bbbb cccc"), last3 = font_measure(FONT_REGULAR, ls, "eeee ffff gggg");
    int best = first3 > after3 ? first3 : after3, other = after4 > last3 ? after4 : last3;
    CHECK(widest == (best < other ? best : other));
    CHECK(widest < full * 2 / 3);
    // A break after a comma wins while its longer line is at most half again as long as the
    // balanced break's: "Fino," over "sin margen", like its neighbours, not "Fino, sin" over "margen".
    const char *fino = "Fino, sin margen";
    int rest = font_measure(FONT_REGULAR, ls, "sin margen");
    CHECK(font_measure(FONT_REGULAR, ls, "Fino, sin") < rest);  // (the balanced break is narrower)
    CHECK(font_wrap_lines(FONT_REGULAR, ls, fino, rest + 4, &widest) == 2 && widest == rest);
    // A first clause much shorter than the rest stays on the line with it, though the rest would
    // fit a line of its own.
    const char *tip = "Tip: aaaa bbbb cccc dddd eeee ffff gggg hhhh";
    int tip_w = font_measure(FONT_REGULAR, ls, tip), after = font_measure(FONT_REGULAR, ls, tip + 5);
    CHECK(font_wrap_lines(FONT_REGULAR, ls, tip, tip_w - 1, &widest) == 2 && after <= tip_w - 1);
    CHECK(widest < after);  // (after "Tip:" the longer line would be the rest)
    // Fits: one line; nothing: none; a word wider than the line: broken between its letters.
    CHECK(font_wrap_lines(FONT_REGULAR, ls, s, full, NULL) == 1);
    CHECK(font_wrap_lines(FONT_REGULAR, ls, "", 100, NULL) == 0);
    CHECK(font_wrap_lines(FONT_REGULAR, ls, "Unbreakableword", font_measure(FONT_REGULAR, ls, "Unbreakableword") / 2, NULL) >= 2);
    // One line that fits is drawn exactly as font_draw_centered / font_draw_left draw it.
    Canvas a, b;
    CHECK(canvas_init(&a, 600, 200) == 0 && canvas_init(&b, 600, 200) == 0);
    for (int center = 0; center < 2; center++) {
        canvas_fill(&a, (Rect){0, 0, 600, 200}, 255);
        canvas_fill(&b, (Rect){0, 0, 600, 200}, 255);
        Rect r = {20, 30, 560, 120};
        if (center) font_draw_centered(&a, FONT_BOLD, 44.f, r, "Calibration saved", 0);
        else font_draw_left(&a, FONT_BOLD, 44.f, r, "Calibration saved", 0);
        CHECK(font_draw_wrapped(&b, FONT_BOLD, 44.f, r, "Calibration saved", 2, 0, center, 0) == 1);
        CHECK(same_pixels(&a, &b));
        canvas_fill(&b, (Rect){0, 0, 600, 200}, 255);
        ui_text(&b, FONT_BOLD, 44.f, 34.f, r, "Calibration saved", 2, center, 0);
        CHECK(same_pixels(&a, &b));
    }
    canvas_free(&a);
    canvas_free(&b);
}

TEST(fit_sizes) {
    ui_set_theme(THEME_GOTHIC);
    const char *s = "Restablecer herramienta";
    int at44 = font_measure(FONT_CAPS, 44.f, s), at34 = font_measure(FONT_CAPS, 34.f, s);
    CHECK(at34 < at44);
    CHECK(font_fit_px(FONT_CAPS, 44.f, 34.f, s, at44) == 44.f);  // fits as it is
    // Jersey 10 has whole steps only: anything narrower than at 44 px is the 34 px step.
    float f = font_fit_px(FONT_CAPS, 44.f, 34.f, s, at44 - 1);
    CHECK(f >= 34.f && f < 44.f && font_measure(FONT_CAPS, f, s) == at34);
    CHECK(font_fit_px(FONT_CAPS, 44.f, 34.f, s, at34 - 1) == 0.f);  // and nothing below that
    CHECK(font_fit_px(FONT_CAPS, 44.f, 40.f, s, at44 - 1) > 0.f);   // a floor inside the step: the step
    // Terminess shrinks in quarter pixels: the largest size that fits.
    ui_set_theme(THEME_CLASSIC);
    at44 = font_measure(FONT_REGULAR, 34.f, s);
    f = font_fit_px(FONT_REGULAR, 34.f, 27.f, s, at44 * 9 / 10);
    CHECK(f > 27.f && f < 34.f && font_measure(FONT_REGULAR, f, s) <= at44 * 9 / 10);
    CHECK(font_measure(FONT_REGULAR, f + 0.25f, s) > at44 * 9 / 10);
    CHECK(font_fit_px(FONT_REGULAR, 34.f, 27.f, s, 10) == 0.f);
    ui_set_theme(THEME_GOTHIC);
    CHECK(ui_text_box_w(FONT_BOLD, 34.f, "150%   tap to reset", 40, 380, 900) >= 380);
    CHECK(ui_text_box_w(FONT_BOLD, 34.f, "", 40, 380, 900) == 380);
    CHECK(ui_text_box_w(FONT_BOLD, 34.f, "A label far longer than any box could ever want to be", 40, 380, 500) == 500);
}

// The columns where two drawings of a button differ within rows [y0, y1): its label's ink, when
// the one is drawn with the label and the other with " " (the same layout, nothing to see).
static void ink_cols(const Canvas *a, const Canvas *b, int y0, int y1, int *x0, int *x1) {
    *x0 = a->w;
    *x1 = -1;
    for (int y = y0; y < y1; y++)
        for (int x = 0; x < a->w; x++)
            if (a->px[(size_t)y * a->stride + x] != b->px[(size_t)y * b->stride + x]) {
                if (x < *x0) *x0 = x;
                if (x > *x1) *x1 = x;
            }
}

// A label in a Gothic button 90 px or more each way keeps inside the carved inner line (2 px wide,
// 10 px in, 13 px in a primary button's), 4 px clear of it: a word up to the edge's inset ran into
// the line ("Compartir" under the Share icon did). Classic has no such line. A row of buttons that
// widens for its longest word (the Library's top bar) asks ui_button_need_w.
TEST(button_labels_clear_the_carved_line) {
    Canvas a, b;
    CHECK(canvas_init(&a, 400, 150) == 0 && canvas_init(&b, 400, 150) == 0);
    font_set_overflow_hook(hook);
    const char *word = "Compartir";  // 150 px under an icon in Gothic, 140 in Classic
    for (int th = THEME_GOTHIC; th <= THEME_CLASSIC; th++) {
        ui_set_theme((UiTheme)th);
        for (int primary = 0; primary < 2; primary++) {
            ButtonStyle st = primary ? BTN_PRIMARY : BTN_PLAIN;
            int in = th == THEME_GOTHIC ? (primary ? 13 : 10) + 2 + 4 : 8;  // the label's inset
            for (int icon = 0; icon < 2; icon++) {
                IconKind ik = icon ? ICON_SHARE : ICON_NONE;
                float px = icon ? 34.f : 44.f;
                int need = ui_button_need_w(150, ik, word, st);
                CHECK(need == font_measure(FONT_CAPS, px, word) + 2 * in);
                // As wide as it needs: whole, inside the line (or Classic's edge) with room to spare.
                Rect r = {0, 0, need, 150};
                canvas_fill(&a, (Rect){0, 0, 400, 150}, 255);
                canvas_fill(&b, (Rect){0, 0, 400, 150}, 255);
                g_hook_calls = 0;
                ui_button(&a, r, ik, word, st);
                ui_button(&b, r, ik, " ", st);
                CHECK(g_hook_calls == 0);
                int x0, x1;
                ink_cols(&a, &b, icon ? 90 : 8, 142, &x0, &x1);
                if (x0 < in || x1 >= need - in) fprintf(stderr, "  ink %d..%d in %d px (inset %d)\n", x0, x1, need, in);
                CHECK(x0 >= in && x1 < need - in && x1 > x0);
                if (icon) {
                    // A pixel narrower: cut short, and the fit check hears of it.
                    ui_button(&a, (Rect){0, 0, need - 1, 150}, ik, word, st);
                    CHECK(g_hook_calls == 1);
                }
            }
        }
        // The top bar holds every language's words: none needs more than its buttons can grow to.
        static const char *const bar[] = {"Exit", "Settings", "Share", "Folder", "New"};
        static const IconKind bar_icon[] = {ICON_EXIT, ICON_GEAR, ICON_SHARE, ICON_FOLDER_PLUS, ICON_PLUS};
        for (int l = 0; l < i18n_language_count(); l++) {
            i18n_set(i18n_language_code(l));
            for (int i = 0; i < 5; i++) {
                int need = ui_button_need_w(150, bar_icon[i], _(bar[i]), i == 4 ? BTN_PRIMARY : BTN_PLAIN);
                if (need > 220) fprintf(stderr, "  %s: \"%s\" needs a %d px button\n", i18n_current(), _(bar[i]), need);
                CHECK(need <= 220);
            }
        }
        i18n_set("en");
    }
    font_set_overflow_hook(NULL);
    canvas_free(&a);
    canvas_free(&b);
    ui_set_theme(THEME_GOTHIC);
}

// The rows of a box the label's ink spans (as ink_cols, by rows): how tall it is drawn.
static int ink_rows(const Canvas *a, const Canvas *b) {
    int y0 = a->h, y1 = -1;
    for (int y = 0; y < a->h; y++)
        if (memcmp(a->px + (size_t)y * a->stride, b->px + (size_t)y * b->stride, (size_t)a->w)) {
            if (y < y0) y0 = y;
            y1 = y;
        }
    return y1 >= y0 ? y1 - y0 + 1 : 0;
}

// Buttons in a grid (Custom music's parts) draw their labels at one size, the smallest any of
// them needs: ui_button_text_px is the size ui_button gives a label of its own, and
// ui_button_text_at draws one at a given size.
TEST(button_grid_sizes) {
    Canvas a, b;
    CHECK(canvas_init(&a, 312, 103) == 0 && canvas_init(&b, 312, 103) == 0);
    Rect r = {0, 0, 312, 103};  // a part's button, upright
    const char *small = "Bass staff", *longer = "Tablatura, 4 l\xc3\xadneas", *far = "A name far too long for one line";
    for (int th = THEME_GOTHIC; th <= THEME_CLASSIC; th++) {
        ui_set_theme((UiTheme)th);
        float ts = ui_text_size(), ss = ui_small_size();
        CHECK(ui_button_text_px(r, small, BTN_PLAIN) == ts);
        float s = ui_button_text_px(r, longer, BTN_PLAIN);
        CHECK(s >= ss && s < ts);
        CHECK(ui_button_text_px(r, far, BTN_PLAIN) == ss);  // two lines at the small size
        // At its own size a label is drawn as ui_button draws it.
        const char *labels[] = {small, longer, far};
        for (int i = 0; i < 3; i++) {
            canvas_fill(&a, r, 255);
            canvas_fill(&b, r, 255);
            ui_button(&a, r, ICON_NONE, labels[i], BTN_PLAIN);
            ui_button_text_at(&b, r, labels[i], BTN_PLAIN, ui_button_text_px(r, labels[i], BTN_PLAIN));
            CHECK(same_pixels(&a, &b));
        }
        // At the grid's size, a short label is drawn smaller than it would be on its own.
        canvas_fill(&b, r, 255);
        ui_button(&b, r, ICON_NONE, " ", BTN_PLAIN);
        canvas_fill(&a, r, 255);
        ui_button(&a, r, ICON_NONE, small, BTN_PLAIN);
        int own = ink_rows(&a, &b);
        canvas_fill(&a, r, 255);
        ui_button_text_at(&a, r, small, BTN_PLAIN, s);
        CHECK(ink_rows(&a, &b) < own && ink_rows(&a, &b) > 0);
    }
    canvas_free(&a);
    canvas_free(&b);
    ui_set_theme(THEME_GOTHIC);
}

// The zoom chip: 380 px for the English at every zoom, in both styles. A longer text keeps as far
// from Gothic's pointed ends (each 50 px deep) as the widest English does from them, 43 px;
// Classic's round ends leave it room closer in.
TEST(chips) {
    char z[96];
    for (int th = THEME_GOTHIC; th <= THEME_CLASSIC; th++) {
        ui_set_theme((UiTheme)th);
        i18n_set("en");
        for (int pct = 50; pct <= 400; pct++) {
            snprintf(z, sizeof z, _("%d%%   tap to reset"), pct);
            CHECK(ui_chip_w(FONT_BOLD, ui_small_size(), z, 100, 380, 2400) == 380);
        }
        const char *longer = "191 %   toca para restablecer";
        int tw = font_measure(FONT_BOLD, ui_small_size(), longer), pad = th == THEME_GOTHIC ? 43 : 24;
        CHECK(tw + 2 * pad > 380);
        CHECK(ui_chip_w(FONT_BOLD, ui_small_size(), longer, 100, 380, 2400) == tw + 2 * pad);
        CHECK(ui_chip_w(FONT_BOLD, ui_small_size(), longer, 100, 380, 500) == 500);
    }
    ui_set_theme(THEME_GOTHIC);
}

// Toasts: one line where it fits, in the very box they always had (tests/scenarios/
// t_toast_over_sheets.qs checks where it goes); two, in a taller box, where one would be cut.
TEST(toasts) {
    Rect screen = {0, 0, 1860, 2480};
    ui_set_theme(THEME_GOTHIC);
    Rect r = ui_toast_rect(screen, "Your Kindle passcode was left off. Turn it back on in Kindle Settings.");
    CHECK(r.x == 112 && r.y == 2250 && r.w == 1636 && r.h == 110);
    const char *damaged = "A damaged page was set aside, and a blank one put in its place: grimoire.log says which";
    CHECK(font_measure(FONT_BOLD, ui_text_size(), damaged) > ui_toast_room(screen));  // one line would cut it
    r = ui_toast_rect(screen, damaged);
    CHECK(r.h == 110 + font_ascent(FONT_BOLD, ui_text_size()) + font_descent(FONT_BOLD, ui_text_size()));
    CHECK(r.y + r.h == 2480 - 120 && r.w <= 1800);
    Canvas c;
    CHECK(canvas_init(&c, 1860, 2480) == 0);
    canvas_fill(&c, screen, 255);
    font_set_overflow_hook(hook);
    g_hook_calls = 0;
    ui_toast_at(&c, r, damaged);
    CHECK(g_hook_calls == 0);  // two lines hold it
    ui_set_theme(THEME_CLASSIC);
    r = ui_toast_rect(screen, damaged);
    CHECK(r.h == 110 || r.h == 110 + font_ascent(FONT_BOLD, ui_text_size()) + font_descent(FONT_BOLD, ui_text_size()));
    ui_toast_at(&c, r, damaged);
    CHECK(g_hook_calls == 0);
    // Three lines' worth: two, the second cut short, and the hook hears of it.
    char three[600];
    snprintf(three, sizeof three, "%s %s %s", damaged, damaged, damaged);
    r = ui_toast_rect(screen, three);
    CHECK(r.w == 1800);
    ui_toast_at(&c, r, three);
    CHECK(g_hook_calls == 1);
    font_set_overflow_hook(NULL);
    canvas_free(&c);
    ui_set_theme(THEME_GOTHIC);
}

int main(void) {
    snprintf(root, sizeof root, "%s/i18n-%d", tl_tmp(), (int)getpid());
    char cmd[700];
    snprintf(cmd, sizeof cmd, "rm -rf '%s'", root);
    system(cmd);
    CHECK(store_init(root) == 0);
    ui_init(300.f);
    if (font_init() != 0) return 1;
    RUN(format_reader);
    RUN(format_compatibility);
    RUN(catalogs_hold_together);
    RUN(lookups);
    RUN(plurals);
    RUN(damaged_catalogs);
    RUN(context_plurals);
    RUN(labels_and_identifiers);
    RUN(locale_names);
    RUN(detection_order);
    RUN(applying_a_setting);
    RUN(language_setting);
    RUN(dates);
    RUN(rare_notices_fit);
    RUN(copies_cut_between_characters);
    RUN(raw_labels_fit);
    RUN(paper_is_the_same_in_every_language);
    RUN(overflow_hook);
    RUN(wrapping);
    RUN(fit_sizes);
    RUN(button_labels_clear_the_carved_line);
    RUN(button_grid_sizes);
    RUN(chips);
    RUN(toasts);
    int rc = tl_done();
    system(cmd);
    return rc;
}
