// i18n.c: the language in use and the lookups (i18n.h). A catalog is a GNU .mo file as msgfmt
// writes it (po/<lang>.mo, embedded): a header, then two tables of n (length, offset) pairs, the
// English strings sorted byte by byte and their translations in the same order. A string with a
// context is stored as "context\004English"; a plural as "English\0English plural", its
// translation as its forms one after another, NUL-separated. Lookups are a binary search; each
// language's plural rule is written here by hand (the catalog's Plural-Forms line is only read by
// tests/test_i18n.c, to hold the two to each other). English is no catalog at all: the text as
// the code has it.
#include "i18n.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "common.h"

const int i18n_ng = 1;

// Which plural form a count takes, per language (an index into the catalog's forms).
static int plural_one_other(long n) { return n != 1; }

static const struct {
    const char *code, *name;  // the name in the language itself (UTF-8)
    int nforms;
    int (*plural)(long n);
} k_langs[] = {
    {"en", "English", 2, plural_one_other},
    {"es", "Espa\xc3\xb1ol", 2, plural_one_other},
};
enum { NLANGS = sizeof k_langs / sizeof k_langs[0] };

int i18n_language_count(void) { return NLANGS; }
const char *i18n_language_code(int i) { return i >= 0 && i < NLANGS ? k_langs[i].code : NULL; }
const char *i18n_language_name(int i) { return i >= 0 && i < NLANGS ? k_langs[i].name : NULL; }

// ---- catalogs ----

typedef struct {
    const unsigned char *d;
    uint32_t size, n, orig, trans;  // strings; where the two tables start
    bool be;                        // written big-endian
} Catalog;

static uint32_t rd32(const Catalog *c, uint32_t off) {
    const unsigned char *p = c->d + off;
    if (c->be) return (uint32_t)p[0] << 24 | (uint32_t)p[1] << 16 | (uint32_t)p[2] << 8 | p[3];
    return (uint32_t)p[3] << 24 | (uint32_t)p[2] << 16 | (uint32_t)p[1] << 8 | p[0];
}

// String i of a table, with its length; NULL unless it lies inside the file and ends in a NUL.
static const char *str_at(const Catalog *c, uint32_t table, uint32_t i, uint32_t *len) {
    uint32_t len_ = rd32(c, table + i * 8), off = rd32(c, table + i * 8 + 4);
    if (off >= c->size || len_ >= c->size - off || c->d[off + len_]) return NULL;
    if (len) *len = len_;
    return (const char *)c->d + off;
}

// Reads a catalog, checking all of it once: the tables and every string inside the file, the
// English in order (or the binary search would miss). False for anything else.
static bool catalog_open(Catalog *c, const unsigned char *d, uint32_t size) {
    *c = (Catalog){.d = d, .size = size};
    if (!d || size < 28) return false;
    uint32_t magic = (uint32_t)d[0] | (uint32_t)d[1] << 8 | (uint32_t)d[2] << 16 | (uint32_t)d[3] << 24;
    if (magic == 0xde120495u) c->be = true;
    else if (magic != 0x950412deu) return false;
    if (rd32(c, 4) >> 16 > 1) return false;  // major revision 0, or 1 (system-dependent strings, unused)
    c->n = rd32(c, 8);
    c->orig = rd32(c, 12);
    c->trans = rd32(c, 16);
    if (c->n > size / 16 || c->orig > size - c->n * 8 || c->trans > size - c->n * 8) return false;
    const char *prev = NULL;
    for (uint32_t i = 0; i < c->n; i++) {
        const char *o = str_at(c, c->orig, i, NULL);
        if (!o || !str_at(c, c->trans, i, NULL) || (prev && strcmp(prev, o) >= 0)) return false;
        prev = o;
    }
    return true;
}

// strcmp of "ctx\004id" (or of "id", ctx NULL) against s, without building the key.
static int key_cmp(const char *ctx, const char *id, const char *s) {
    if (ctx) {
        for (; *ctx; ctx++, s++)
            if (*ctx != *s) return (unsigned char)*ctx - (unsigned char)*s;
        if (*s != '\004') return '\004' - (unsigned char)*s;
        s++;
    }
    return strcmp(id, s);
}

// The translation of id (in ctx), with its length; NULL if the catalog has none or it is empty.
static const char *catalog_find(const Catalog *c, const char *ctx, const char *id, uint32_t *len) {
    uint32_t lo = 0, hi = c->n;
    while (lo < hi) {
        uint32_t mid = lo + (hi - lo) / 2;
        int cmp = key_cmp(ctx, id, str_at(c, c->orig, mid, NULL));
        if (!cmp) {
            const char *t = str_at(c, c->trans, mid, len);
            return *t ? t : NULL;
        }
        if (cmp < 0) hi = mid;
        else lo = mid + 1;
    }
    return NULL;
}

#ifdef GRIMOIRE_SIM
// The simulator's GRIMOIRE_L10N_DIR (a test hook, and a translator's shortcut): a language's
// catalog is read from <dir>/<code>.mo instead of the binary, once, and kept for the run. NULL
// when the variable is not set (the embedded catalogs, as on the Kindle) or that file is missing.
static const unsigned char *sim_catalog(const char *code, uint32_t *size) {
    static struct {
        char code[8];
        unsigned char *d;
        uint32_t size;
    } k_read[8];
    const char *dir = getenv("GRIMOIRE_L10N_DIR");
    if (!dir || !*dir || strlen(code) >= sizeof k_read[0].code) return NULL;
    for (size_t i = 0; i < sizeof k_read / sizeof k_read[0]; i++) {
        if (k_read[i].code[0] && strcmp(k_read[i].code, code)) continue;
        if (!k_read[i].code[0]) {
            char path[1024];
            size_t len = 0;
            snprintf(path, sizeof path, "%s/%s.mo", dir, code);
            k_read[i].d = read_file(path, &len);
            k_read[i].size = len < UINT32_MAX ? (uint32_t)len : 0;
            snprintf(k_read[i].code, sizeof k_read[i].code, "%s", code);
            qlog("i18n: (simulator) the catalog for %s %s %s", code, k_read[i].d ? "comes from" : "is not in", path);
        }
        *size = k_read[i].size;
        return k_read[i].d;
    }
    return NULL;
}
#endif

// The embedded catalog of a language: false if there is none (English) or it does not read.
static bool catalog_for(const char *code, Catalog *out) {
#ifdef GRIMOIRE_SIM
    uint32_t sim_size;
    const unsigned char *sim = sim_catalog(code, &sim_size);
    if (sim) {
        if (catalog_open(out, sim, sim_size)) return true;
        qlog("i18n: the catalog for %s is damaged; using English", code);
        return false;
    }
#endif
    for (const I18nCatalog *k = i18n_catalogs; k->code; k++)
        if (!strcmp(k->code, code)) {
            if (catalog_open(out, k->mo, k->size)) return true;
            qlog("i18n: the catalog for %s is damaged; using English", code);
            return false;
        }
    return false;
}

static int g_lang;           // index into k_langs
static Catalog g_cat;        // its catalog
static bool g_have_cat;

const char *i18n_get(const char *msgid) {
    if (!g_have_cat || !msgid || !*msgid) return msgid;
    const char *t = catalog_find(&g_cat, NULL, msgid, NULL);
    return t ? t : msgid;
}

const char *i18n_getc(const char *ctx, const char *msgid) {
    if (!g_have_cat || !msgid || !*msgid) return msgid;
    const char *t = catalog_find(&g_cat, ctx, msgid, NULL);
    return t ? t : msgid;
}

const char *i18n_getnc(const char *ctx, const char *one, const char *many, long n) {
    uint32_t len;
    const char *t = g_have_cat && one && *one ? catalog_find(&g_cat, ctx, one, &len) : NULL;
    if (t) {
        // Form k: past k NULs, if the translation has that many forms.
        const char *end = t + len;
        for (int k = k_langs[g_lang].plural(n); k > 0 && t <= end; k--) t += strlen(t) + 1;
        if (t < end && *t) return t;
    }
    return n == 1 ? one : many;
}

const char *i18n_getn(const char *one, const char *many, long n) { return i18n_getnc(NULL, one, many, n); }
const char *i18n_getn_many(const char *one, const char *many, long n) { return i18n_getnc(NULL, one, many, n); }

size_t i18n_copy(char *dst, const char *src, size_t cap) {
    size_t n = strlcpy_q(dst, src, cap);
    // The last character copied, if it is not whole (cut here, or before: by snprintf).
    size_t lead = n;
    while (lead > 0 && ((unsigned char)dst[lead - 1] & 0xC0) == 0x80) lead--;
    if (lead > 0) {
        unsigned char c = (unsigned char)dst[lead - 1];
        size_t len = c >= 0xF0 ? 4 : c >= 0xE0 ? 3 : c >= 0xC0 ? 2 : 1;
        if (n - (lead - 1) < len) n = lead - 1;
    }
    dst[n] = 0;
    return n;
}

// ---- choosing ----

int i18n_find(const char *locale) {
    if (!locale) return -1;
    // The language part: the letters before a region, an encoding or a modifier.
    char part[8];
    size_t n = 0;
    while (locale[n] && !strchr("_-.@", locale[n]) && n + 1 < sizeof part) {
        char ch = locale[n];
        part[n++] = (char)(ch >= 'A' && ch <= 'Z' ? ch - 'A' + 'a' : ch);
    }
    part[n] = 0;
    for (int i = 0; i < NLANGS; i++)
        if (!strcmp(part, k_langs[i].code)) return i;
    return -1;
}

bool i18n_set(const char *code) {
    int i = i18n_find(code);
    g_lang = i < 0 ? 0 : i;
    g_have_cat = g_lang != 0 && catalog_for(k_langs[g_lang].code, &g_cat);
    return i >= 0;
}

const char *i18n_current(void) { return k_langs[g_lang].code; }

bool i18n_setting_known(const char *value) {
    if (!value) return false;
    if (!strcmp(value, "auto")) return true;
    for (int i = 0; i < NLANGS; i++)
        if (!strcmp(value, k_langs[i].code)) return true;
    return false;
}

// A value from the environment or the locale file as the log shows it: printable ASCII, short.
static void loggable(const char *v, char *out, size_t cap) {
    size_t n = 0;
    for (; *v && n + 1 < cap && n < 40; v++) out[n++] = *v > ' ' && *v < 0x7F && *v != '"' ? *v : '?';
    out[n] = 0;
}

void i18n_locale_file_lang(const char *text, char *out, size_t cap) {
    if (cap) out[0] = 0;
    if (!text || !cap) return;
    static const char *const keys[] = {"LANG=", "LC_ALL="};
    for (size_t k = 0; k < sizeof keys / sizeof keys[0]; k++)
        for (const char *line = text; line && *line; line = strchr(line, '\n') ? strchr(line, '\n') + 1 : NULL) {
            const char *s = line;
            while (*s == ' ' || *s == '\t') s++;
            if (!strncmp(s, "export ", 7)) s += 7;
            if (strncmp(s, keys[k], strlen(keys[k]))) continue;
            s += strlen(keys[k]);
            if (*s == '"' || *s == '\'') s++;
            size_t n = strcspn(s, "\"'\r\n \t");
            if (!n) continue;
            if (n >= cap) n = cap - 1;
            memcpy(out, s, n);
            out[n] = 0;
            return;
        }
}

int i18n_detect_from(const char *kindle_locale, char *why, size_t cap) {
    char file_lang[64] = "";
    i18n_locale_file_lang(kindle_locale, file_lang, sizeof file_lang);
    const struct {
        const char *name, *value;
    } src[] = {
        {"GRIMOIRE_LANG", getenv("GRIMOIRE_LANG")},
        {NULL, file_lang},  // the Kindle's locale
        {"LC_ALL", getenv("LC_ALL")},
        {"LC_MESSAGES", getenv("LC_MESSAGES")},
        {"LANG", getenv("LANG")},
    };
    for (size_t i = 0; i < sizeof src / sizeof src[0]; i++) {
        if (!src[i].value || !*src[i].value) continue;
        char v[48];
        loggable(src[i].value, v, sizeof v);
        int lang = i18n_find(src[i].value);
        if (src[i].name) snprintf(why, cap, "from %s=%s%s", src[i].name, v, lang < 0 ? ", not a language Grimoire speaks" : "");
        else snprintf(why, cap, "from the Kindle's locale %s%s", v, lang < 0 ? ", not a language Grimoire speaks" : "");
        return lang < 0 ? 0 : lang;
    }
    snprintf(why, cap, "no language set");
    return 0;
}

static int g_auto;
static char g_auto_why[160] = "not detected";

void i18n_detect(void) {
    char *text = NULL;
    const char *missing = "";
#ifdef GRIMOIRE_KINDLE
    text = read_file("/var/local/system/locale", NULL);
    if (!text) missing = "; /var/local/system/locale cannot be read";
#endif
    char why[112];
    g_auto = i18n_detect_from(text, why, sizeof why);
    snprintf(g_auto_why, sizeof g_auto_why, "%s%s", why, missing);
    free(text);
}

const char *i18n_auto(void) { return k_langs[g_auto].code; }

void i18n_apply(const char *setting) {
    if (setting && strcmp(setting, "auto") && i18n_setting_known(setting)) {
        i18n_set(setting);
        qlog("i18n: language %s (set in Settings)", i18n_current());
        return;
    }
    i18n_set(k_langs[g_auto].code);
    char v[48] = "";
    if (setting && *setting && strcmp(setting, "auto")) loggable(setting, v, sizeof v);
    qlog("i18n: language %s (auto, %s)%s%s%s", i18n_current(), g_auto_why, v[0] ? "; settings.ini says language=" : "", v,
         v[0] ? ", a language this version does not speak" : "");
}

// ---- for tests ----

int i18n_walk(const char *code, void (*f)(const I18nEntry *e, void *arg), void *arg) {
    Catalog c;
    if (!code || !catalog_for(code, &c)) return -1;
    int count = 0;
    for (uint32_t i = 0; i < c.n; i++) {
        uint32_t olen, tlen;
        const char *o = str_at(&c, c.orig, i, &olen), *t = str_at(&c, c.trans, i, &tlen);
        if (!*o) continue;  // the header
        const char *o_end = o + olen;
        I18nEntry e = {.str = t, .nforms = 1};
        const char *eot = strchr(o, '\004');
        if (eot) {
            snprintf(e.ctx, sizeof e.ctx, "%.*s", (int)(eot - o), o);
            o = eot + 1;
        }
        e.id = o;
        if (o + strlen(o) < o_end) e.id_plural = o + strlen(o) + 1;
        for (uint32_t k = 0; k < tlen; k++) e.nforms += !t[k];
        count++;
        if (f) f(&e, arg);
    }
    return count;
}

const char *i18n_header(const char *code) {
    Catalog c;
    if (!code || !catalog_for(code, &c)) return "";
    const char *t = catalog_find(&c, NULL, "", NULL);
    return t ? t : "";
}

bool i18n_mo_valid(const unsigned char *mo, unsigned int size) {
    Catalog c;
    return catalog_open(&c, mo, size);
}

int i18n_plural_forms(const char *code) {
    int i = i18n_find(code);
    return i < 0 ? 0 : k_langs[i].nforms;
}
