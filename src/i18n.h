// i18n.h: user-visible text goes through these, so it can be translated. Until the catalogs land
// (the i18n work), every string passes through untranslated. Wrap whole sentences, with %s for
// paths and values; never wrap identifiers (settings keys, file names, log lines, API fields).
//   _("Saved")                       a string shown to the user
//   N_("Pencil")                     marks a string for translation in a static table (no lookup)
//   C_("paper", "Blank")             the same, with a context telling two meanings apart
//   NG_("%d page", "%d pages", n)    the plural form for n
#pragma once
#define _(s) (s)
#define N_(s) (s)
#define C_(ctx, s) (s)
#define NG_(one, many, n) ((n) == 1 ? (one) : (many))
