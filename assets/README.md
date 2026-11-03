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

`eraser-sample.qpg` contains the maintainer's handwritten “erase me!” practice ink, contributed
to Grimoire under the project's GPL-3.0 license. The app fits a private in-memory copy to the
eraser pad. It does not read or modify a notebook to provide this sample.
