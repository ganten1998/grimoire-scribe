**English** | [日本語](../japanese/Translating-ja.md) | [Polski](../polish/Translating-pl.md)

# Translating Grimoire

Grimoire speaks English, Spanish, Japanese and Polish. This page says how to improve a translation
and how to add a language. The full rules are in `docs/TRANSLATING.md` in the repository.

## What there is

```
po/grimoire.pot      every string to translate, made from the sources by make l10n
po/<lang>.po         one language's translations (po/es.po), the file translators edit
po/<lang>.mo         that file compiled by msgfmt: committed, and embedded in the binary
po/es-glossary.md    the words the Spanish chose, with the alternatives weighed
po/ja-glossary.md    the words the Japanese chose, and the font and date notes
```

- The English is in the code (`src/*.c` and `web/studio.js`). A string a catalog lacks, or leaves
  empty, shows in English.
- `po/grimoire.pot` holds every string the screens show and the Effects studio's, each with its
  `TRANSLATORS` comment and the file it comes from.
- Building needs no gettext: the binary embeds the committed `.mo` files. `make l10n`, which needs
  GNU gettext, makes `po/grimoire.pot` again from the sources, merges it into every
  `po/<lang>.po` and compiles each to its `.mo`. `make l10n-check`, part of `make check`, checks
  every language on every screen.
- KOReader's Grimoire plugin has a few words of its own, not in `po/`: the `STRINGS` table of
  `kindle/koreader/grimoire.koplugin/main.lua` and the description in `_meta.lua`. It picks them by
  KOReader's own language setting.
- Translations are under Grimoire's license, GPL-3.0-or-later, as the header of `po/es.po` says.

## Improving the Spanish

The Spanish is one neutral `es` for Spain and Latin America, speaking to the reader as *tú*.
`po/es-glossary.md` lists every choice of words, with the alternatives weighed: start there.

## Improving the Japanese

The Japanese is one `ja`, speaking to the reader in the polite です・ます form for sentences and
noun labels for buttons. `po/ja-glossary.md` lists the main choices and the date and counter
conventions: start there. Its kana and kanji come from DotGothic16, a pixel font embedded as the
Japanese fallback (subset to the kana and the common kanji), so the Gothic style survives.

## Improving the Polish

The Polish is one `pl`, speaking to the reader in the second person singular. It has three plural
forms: the first for 1, the second for 2–4 (but not 12–14), and the third for the rest. The
interface's terms are in `po/pl.po`; new strings should match them.

## The rules of a translation

- **Room.** Most strings sit in a widget of a fixed width, and their `TRANSLATORS` comment gives
  the room as pixels at a text size: "284 px at 34 px" is a text drawn 34 px tall with 284 px of
  width. `make l10n-check` lists every text cut short. A word that still does not fit needs a
  shorter one.
- **The top bar's taps.** A top-bar word wider than 153 px in Gothic (159 px for all but New) or
  175 px in Classic widens the buttons enough to move New away from the scenario's taps. Keep
  those words that narrow.
- **Arguments.** Keep every `%s`, `%d` and the like, of the same kind and as many as the English
  has. To put them in another order, number them all: `%2$d %1$s`. Never mix numbered and plain
  ones, use every argument, and write a percent sign as `%%` where the English does.
- **Whole sentences.** Each string is a whole sentence or label, with values coming in through
  `%s` and `%d`. A row a tap cycles through is one sentence per value: translate each whole.
- **Plurals.** A string with a `msgid_plural` needs every form the language has
  (`msgstr[0]`, `msgstr[1]`...), in the order of its Plural-Forms rule.
- **Contexts.** A `msgctxt` tells two meanings of the same English apart (the month "May", a
  "Blank" paper): translate each in its own sense. The contexts are `paper`, `tool`, `music part`,
  `month`, `date`, `key`, `keyboard`, `share link`, `folder`, `sleep after` and `studio`.
- **Characters.** Every character must be one that a face can draw: Jersey 10 and Jacquard 24
  (Gothic), Terminess regular and bold (Classic), or, for Japanese, DotGothic16. The Classic fonts
  of Settings > Font must have every character but Japanese's own too.
- **Font names** (Terminess, Atkinson Hyperlegible Next, Noto Sans, Literata) are never
  translated.
- **What stays as it is**: Grimoire, Kindle, Kindle Scribe, KOReader, Wi-Fi, PDF, PNG, ZIP,
  ImageMagick; the file and folder names on the Kindle (`grimoire/PDFs`, `grimoire/trash`,
  `grimoire.log`); the command `tools/grimoire-pull --http`. The headings printed on Cornell paper
  belong to the page and are not in the catalog.

## Adding a language

Take French, `fr`:

1. Make its catalog from the template:

   ```
   msginit --input=po/grimoire.pot --locale=fr --output-file=po/fr.po --no-translator
   ```

   `msginit` fills in the header, Plural-Forms included (for French `nplurals=2; plural=(n > 1);`).
   Give its header comment the license line `po/es.po` has.
2. Add its line to `k_langs` in `src/i18n.c`, after the others: its code, its name in itself
   ("Français"), its number of plural forms, and its plural rule, a function from a count to the
   index of that count's form, written to match Plural-Forms.
3. Add its Plural-Forms line to `k_rules` in `tests/test_i18n.c`, as the catalog's header has it,
   and checks of the function to the `plurals` test there: `NG_` at 0, 1 and 2 with the forms the
   catalog gives.
4. Translate, then `make l10n`: it merges the template into `po/fr.po` and compiles `po/fr.mo`.
   `make l10n-check` checks every screen; `make test` runs `tests/test_i18n.c`.

## Trying a change without building

To try a change in the desktop simulator without building it again, compile the catalog by hand
and point the simulator at it:

```sh
mkdir -p /tmp/l10n && msgfmt --check -o /tmp/l10n/es.mo po/es.po
GRIMOIRE_L10N_DIR=/tmp/l10n GRIMOIRE_LANG=es ./build/host/grimoire-sim
```

Its log then says `i18n: (simulator) the catalog for es comes from /tmp/l10n/es.mo`.

Without gettext you can still edit `po/es.po` or `po/ja.po` and send it; whoever builds runs
`make l10n`. Commit `po/<lang>.po` and `po/<lang>.mo` together, with `po/grimoire.pot` if it
changed: `make l10n-check` fails when a `.mo` is not what msgfmt makes of its `.po`.
