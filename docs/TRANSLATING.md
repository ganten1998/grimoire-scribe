# Translating Grimoire

Grimoire speaks English and Spanish. This page says how to improve a translation and how to add a
language. How the code does it is in [DESIGN.md](DESIGN.md) (Languages).

## What there is

```
po/grimoire.pot      every string to translate, made from the sources by make l10n
po/<lang>.po         one language's translations (po/es.po), the file translators edit
po/<lang>.mo         that file compiled by msgfmt: committed, and embedded in the binary
po/es-glossary.md    the words the Spanish chose, with the alternatives weighed
```

- The English is in the code (src/*.c and web/studio.js). A string a catalog lacks, or leaves
  empty, shows in English.
- `po/grimoire.pot` holds every string the screens show (from src/*.c) and the Effects studio's
  (from web/studio.js, under the context `studio`), each with its `TRANSLATORS` comment and the
  file it comes from.
- Building needs no gettext: the binary embeds the committed `.mo` files. `make l10n`, which needs
  GNU gettext, makes `po/grimoire.pot` again from the sources, merges it into every `po/<lang>.po`
  and compiles each to its `.mo`. `make l10n-check`, part of `make check`, checks every language
  on every screen.
- KOReader's Grimoire plugin has a few words of its own, not in po/: the `STRINGS` table of
  `kindle/koreader/grimoire.koplugin/main.lua` and the description in `_meta.lua`. It picks them
  by KOReader's own language setting.
- Translations are under Grimoire's license, GPL-3.0-or-later, as the header of `po/es.po` says.

## Improving the Spanish

The Spanish is one neutral `es` for Spain and Latin America, speaking to the reader as *tú*, and a
draft until a native speaker reviews it. `po/es-glossary.md` lists every choice of words to
confirm, with the alternatives weighed: start there.

1. Edit `po/es.po` in a .po editor or a text editor. Change a `msgstr`, never a `msgid`: that is
   the English, and the code finds the translation by it. Take off a `#, fuzzy` flag once you have
   checked its entry: msgfmt leaves fuzzy entries out, so they show in English.
2. `make l10n` compiles `po/es.mo` (and updates `po/grimoire.pot` from the sources). It stops at a
   translation whose printf conversions differ from its English.
3. `make l10n-check` runs every screen in Spanish, in both styles, and lists each text that does
   not fit as `fit[es]: "..." needs W px, has R`. Its shots show every screen in context:
   `build/shots/l10n/es-gothic/` and `es-classic/`, `hosted-es-gothic/` and
   `hosted-es-classic/` for the Library opened from KOReader, and `templates-es-gothic/` and
   `templates-es-classic/` for the Templates sheet with templates in it.
4. `make test` runs tests/test_i18n.c, which checks the catalog as well (What the checks catch,
   below).
5. Commit `po/es.po` and `po/es.mo` together, with `po/grimoire.pot` if it changed: make
   l10n-check fails when a `.mo` is not what msgfmt makes of its `.po`. Update the glossary when a
   choice changes.

Without gettext you can still edit `po/es.po` and send it; whoever builds runs `make l10n`.

Merge with `make l10n`, not with msgmerge by hand. It keeps `po/grimoire.pot` and each
`po/<lang>.po` in gettext's own line wrapping (a long entry broken into lines of about 80
columns), as they are committed, so its diff shows only the entries that changed.
`msgmerge --no-wrap` puts every long entry on one line, and the next `make l10n` breaks them all
again.

To try a change in the desktop simulator (docs/SIM.md) without building it again, compile the
catalog by hand and point the simulator at it:

```
mkdir -p /tmp/l10n && msgfmt --check -o /tmp/l10n/es.mo po/es.po
GRIMOIRE_L10N_DIR=/tmp/l10n GRIMOIRE_LANG=es ./build/host/grimoire-sim
```

Its log then says `i18n: (simulator) the catalog for es comes from /tmp/l10n/es.mo`.

## The rules of a translation

- **Room.** Most strings sit in a widget of a fixed width, and their `TRANSLATORS` comment gives
  the room as pixels at a text size: "284 px at 34 px" is a text drawn 34 px tall with 284 px of
  width. You need not measure: `make l10n-check` lists every text cut short. The layout helps
  where it can (a long notice takes two lines, a long label a smaller size or two lines, the
  Library's top-bar buttons widen together), but a word that still does not fit needs a shorter
  one. The glossary records several such choices: *Ajustes* rather than *Configuración* in the top
  bar, *Estilográfica* in the tool picker.
- **The top bar's taps.** tests/scenarios/y_all_screens.qs taps the Library's New, Folder and
  Share where English and Spanish put them. A top-bar word wider than 153 px in Gothic (159 px for
  all but New) or 175 px in Classic, though within its comment's room, widens the buttons enough
  to move New away from that tap (Folder and Share follow a little later), and make l10n-check then
  fails on `expect` lines, not with a `fit` line. Keep those words that narrow, or the scenario's
  taps and docs/SIM.md's coordinates have to change with them.
- **Arguments.** Keep every `%s`, `%d` and the like, of the same kind and as many as the English
  has. To put them in another order, number them all: `%2$d %1$s`. The date does so: its English
  `%s %d, %02d:%02d` (month, day, hours, minutes) is `%2$d %1$s, %3$02d:%4$02d` in Spanish. Never
  mix numbered and plain ones, use every argument, and write a percent sign as `%%` where the
  English does. msgfmt --check and tests/test_i18n.c refuse a translation that reads other
  arguments than its English.
- **Whole sentences.** Each string is a whole sentence or label, with values coming in through
  `%s` and `%d`. A row a tap cycles through is one sentence per value ("Holding the pen button
  erases (tap to change)"): translate each whole.
- **Plurals.** A string with a `msgid_plural` needs every form the language has (`msgstr[0]`,
  `msgstr[1]`...), in the order of its Plural-Forms rule.
- **Contexts.** A `msgctxt` tells two meanings of the same English apart (the month "May", a
  "Blank" paper): translate each in its own sense. The contexts are `paper`, `tool`, `music part`,
  `month`, `date`, `key`, `keyboard`, `share link`, `folder`, `sleep after` and `studio`. The
  months are short names (Spanish *oct*, *sept*).
- **Characters.** Every character must be one that all of Grimoire's faces have: Jersey 10 and
  Jacquard 24 (Gothic), Terminess regular and bold (Classic). tests/test_i18n.c fails on any
  other. The Gothic faces draw º and ª as a plain o and a, so the Spanish does without them.
- **What stays as it is**: Grimoire, Kindle, Kindle Scribe, KOReader, Wi-Fi, PDF, PNG, ZIP,
  ImageMagick; the file and folder names on the Kindle (`grimoire/PDFs`, `grimoire/trash`,
  `grimoire.log`); the command `tools/grimoire-pull --http`. The headings printed on Cornell paper
  belong to the page and are not in the catalog, so a page looks the same in every language.
- **The studio's strings** (context `studio`) are shown in a browser, under the same rules. They
  reach the page escaped as text, so markup in a translation shows as it is written.

## Adding a language

Take French, `fr`:

1. Make its catalog from the template:
   ```
   msginit --input=po/grimoire.pot --locale=fr --output-file=po/fr.po --no-translator
   ```
   msginit fills in the header, Plural-Forms included (for French `nplurals=2; plural=(n > 1);`).
   Give its header comment the license line `po/es.po` has.
2. Add its line to `k_langs` in src/i18n.c, after the others: its code, its name in itself
   ("Français"), its number of plural forms, and its plural rule, a function from a count to the
   index of that count's form, written to match Plural-Forms. English and Spanish share
   `plural_one_other` (`n != 1`); French needs one of its own (`n > 1`).
3. Add its Plural-Forms line to `k_rules` in tests/test_i18n.c, as the catalog's header has it.
   The test holds the catalog's header to that line, a copy kept by hand, not to the function in
   src/i18n.c, so add checks of the function too, to the `plurals` test there: `NG_` at 0, 1 and 2
   with the forms the catalog gives (in French 0 and 1 take the first form, 2 the second).
4. Translate, then `make l10n`: it merges the template into `po/fr.po` and compiles `po/fr.mo`,
   which the build embeds by itself (every `po/*.mo`). Settings > Language then lists the language
   by its own name, and Automatic picks it on a Kindle set to it (`fr`, `fr_CA.UTF-8`...).
5. `make test`, then `make l10n-check`, which now runs every screen in French too (shots in
   `build/shots/l10n/fr-gothic/` and `fr-classic/`); fix every `fit[fr]` line it lists. Commit
   `po/fr.po`, `po/fr.mo` and `po/grimoire.pot` with the code.
6. A glossary like `po/es-glossary.md` helps whoever reviews it next. The KOReader plugin learns
   the language in its `STRINGS` table (main.lua) and `_meta.lua`. A language whose alphabet has
   letters past a to z can have them on the keyboard's letters page in src/keyboard.c, as Spanish
   has ñ; until then they are on the accents page.

Every language of `k_langs` but English must have its `.mo`, and every `.mo` its line there:
tests/test_i18n.c checks both, and that the language's own name can be drawn in every face.

## What the checks catch

- msgfmt --check (make l10n and make l10n-check): a `.po` that does not compile, and a translation
  whose printf conversions differ from its English.
- make l10n-check: a `.mo` that is not what msgfmt makes of its `.po` now; text cut short on any
  screen or sheet, in either style, upright or sideways (`fit[<lang>]: "..." needs W px, has R`);
  the Settings sheet from a deep data root, whose path is cut short as names are and does not
  count; and the Library's top bar opened from KOReader. A top bar widened past the every-screen
  scenario's taps fails here too, on its `expect` lines (The top bar's taps, above).
- tests/test_i18n.c: the same printf arguments, by type, in every translation and plural form;
  the number of plural forms; the Plural-Forms line against `k_rules`, and the plural rule itself
  only at the counts its `plurals` test checks; characters every face has; a damaged catalog
  refused whole; and the widths no screen of the every-screen scenario shows: rare notices, which
  must take at most two lines in both styles, the Share screen without an address, and Pen feel's
  labels.
- tests/share_check.sh: the share pages and the studio in Spanish, and that no translation can add
  markup to them.
