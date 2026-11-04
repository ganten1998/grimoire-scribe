The font licenses are stored beside each font.

`DotGothic16-Regular.ttf` is a subset of the upstream face (SIL Open Font License 1.1, see
`DotGothic16-OFL-1.1.txt`), made with fontTools' `pyftsubset` under the OFL's permission for
modified versions. It keeps the kana, the CJK punctuation and fullwidth forms, every kanji
`po/ja.po` and `po/ja-glossary.md` use, and JIS X 0208 level 1's kanji (常用漢字 and the rest of
the level 1 set), which covers ordinary names. That is 3,340 of the upstream face's glyphs and
about 0.65 MB instead of 2 MB in every build. Nothing in the app needs the glyphs left out: text
outside the subset falls back to Terminess, or is left out of a name as any missing glyph is.

To make it again after `po/ja.po` gains kanji, with `pip install fonttools`:

```
python3 - <<'EOF'
import subprocess
chars = set()
for p in ["po/ja.po", "po/ja-glossary.md"]:
    chars |= set(open(p, encoding="utf-8").read())
kanji = {c for c in chars if 0x3400 <= ord(c) <= 0x9FFF or 0xF900 <= ord(c) <= 0xFAFF}
for row in range(16, 48):  # JIS X 0208 level 1
    for cell in range(1, 95):
        try:
            ch = bytes([0xA0 + row, 0xA0 + cell]).decode("euc_jp")
        except Exception:
            continue
        if 0x4E00 <= ord(ch) <= 0x9FFF:
            kanji.add(ch)
unicodes = ",".join(["U+3000-303F", "U+3040-30FF", "U+FF00-FFEF"] +
                    ["U+%04X" % ord(c) for c in sorted(kanji)])
subprocess.run(["pyftsubset", "assets/DotGothic16-Regular.ttf",
                "--output-file=assets/DotGothic16-Regular.ttf", "--unicodes=" + unicodes,
                "--layout-features=", "--no-hinting", "--name-IDs=1,2,3,4,5,6",
                "--notdef-outline"], check=True)
EOF
```

`TerminessNerdFontMono-Regular.ttf` and `-Bold.ttf` are a subset of Nerd Fonts' Terminess (Nerd
Fonts 3.4.0, its Terminess Nerd Font Mono, a patched build of Terminus TTF 4.49.3; SIL Open Font
License 1.1, see `Terminess-OFL-1.1.txt`), cut down to Terminus's text glyphs and the Material
Design icons Grimoire draws or plans: exactly the code points `terminess-keep.txt` lists, 1,197
text and 84 icons. That is about 0.43 MB a weight instead of 2.67 MB. The icons are Material Design
Icons by Pictogrammers under the Apache License 2.0 (`MaterialDesignIcons-LICENSE.txt`, which says
where they come from, and `MaterialDesignIcons-Apache-2.0.txt`); the release ships both notices
beside the OFL text, and the share pages serve all three, with the Gothic fonts' OFL texts, as
`/font/licenses.txt` beside the fonts (each font's answer names it as its license). Left out: every
other icon set Nerd Fonts patched in (Font Awesome and Codicons, CC BY 4.0; Font Logos, brand logos
Nerd Fonts lists as unlicensed; Octicons, Devicons, Seti-UI and the rest), and the Nerd Fonts glyphs
at ordinary code points (its box drawing and block elements, drawn over Terminus's, the Octicons
heart and lightning, the power symbols, the menu sign and the heavy angle brackets). Those
characters are no longer in Terminess: Grimoire's own screens and pages use none of them, a name
that has one shows without it, and KOReader takes them from its fallback fonts. The kept glyphs are
the source's own bytes: their outlines and metrics, the hinting that decides how FreeType draws
them (no hinting would turn on its auto-hinter), the embedded bitmaps it uses at 12 to 32 px, and
the name table whole (the family name is what FreeType reports and what KOReader lists and stores
when the font is chosen; KOReader's patch finds the files by their names, and the share pages load
them under a name of their own). Drawn through stb_truetype as the app draws them and through
FreeType (hinting on and off) at 12 to 100 px, every kept glyph is byte for byte what the full font
draws.

A new icon goes into `terminess-keep.txt` (rust/tests/test_terminess_keep.rs fails while the code
draws one the font lacks), then the fonts are made again. Never an icon of Material Design Icons'
"Brand / Logo" category: Pictogrammers' license does not cover those, and
`MaterialDesignIcons-LICENSE.txt` calls every icon in Terminess Apache-2.0. For the maintainer, with
fontTools 4.68.0 (the version that made the committed fonts) in a venv outside the repo
(`python3 -m venv ~/venvs/fonttools && ~/venvs/fonttools/bin/pip install fonttools==4.68.0`):

```
make fonts-subset FONTS_PYTHON=~/venvs/fonttools/bin/python
```

It downloads, once, into `~/.cache/grimoire-fonts` (`FONTS_CACHE`): Nerd Fonts' release
(https://github.com/ryanoasis/nerd-fonts/releases/download/v3.4.0/Terminus.zip, SHA-256
`b52a95517b920038396a64ea5ae9e4006541a63831c4a7397d5ce648a8d56d8e`; its two Mono fonts are the
files Grimoire shipped whole until 2.0.0-beta.1) and upstream Terminus TTF
(https://files.ax86.net/terminus-ttf/files/4.49.3/terminus-ttf-4.49.3.zip, SHA-256
`0ead921d98d99a4590ffe6cd66dc037fc0a2ceea1c735d866ba73fe058257577`). Then `tools/fonts_subset.py`
checks the list's text against upstream Terminus (a text code point must hold Terminus's own
outline in Nerd Fonts' file, and each that does must be listed), checks that every icon is a
Material Design icon of the release, subsets both weights with fontTools and writes them here.
`FONTS_SUBSET_FLAGS=--verify` writes nothing and checks that the committed fonts are what the list
makes; with fontTools 4.68.0 they are byte for byte. `make check` never runs it.

`eraser-sample.qpg` contains the maintainer's handwritten “erase me!” practice ink, contributed
to Grimoire under the project's GPL-3.0 license. The app fits a private in-memory copy to the
eraser pad. It does not read or modify a notebook to provide this sample.
