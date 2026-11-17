**English** | [日本語](../japanese/Fonts-and-Language-ja.md) | [Polski](../polish/Fonts-and-Language-pl.md)

# Fonts and Language

## Language

Grimoire speaks English, Spanish, Japanese and Polish. **Settings > Language** offers **Automatic**,
**English**, **Español**, **日本語** and **Polski**. Automatic, the default, follows the language set
in the Kindle's own Settings, and gives English for a language Grimoire does not speak yet; its row
names the language it gives ("Automatic (English)"). So if your Kindle is set to Spanish, Japanese
or Polish, Grimoire is in that language from the first start after you update; **Ajustes > Idioma >
English** (Settings > Language, in Spanish), **設定 > 言語 > English** (in Japanese) or
**Ustawienia > Język > English** (in Polish) switches it back. A choice applies at once and stays
chosen.

All of Grimoire follows it: every screen, the share pages and the Effects studio, and the name a
new notebook gets ("Notebook Oct 20", "Cuaderno 20 oct", "ノート 10月20日", "Notatnik 20 paź").
Your pages never change with the language: paper is drawn the same everywhere (Cornell notes keep
their English headings), and the names you give notebooks and folders stay as you typed them. The
KOReader plugin follows KOReader's own language.

Corrections and new languages are welcome; see [Translating](Translating). Translations are under
Grimoire's license, GPL-3.0-or-later.

## Fonts (Classic)

Settings' Display category has a **Font** row in the Classic style, naming the font in use. It
opens the Font sheet:

- **Terminess** (the default)
- **Atkinson Hyperlegible Next** (made for readers with low vision: I, l and 1, O and 0 cannot be
  mistaken)
- **Noto Sans**
- **Literata** (a serif made for reading on screens)

Each is named in itself, with a sample of the one in use at the top. A tap on a font applies it to
every screen at once and saves it; the sheet's back arrow returns to Settings and its X closes
Settings, both keeping the font you tapped (to go back, tap the font you had).

Your pages, paper labels, the clock, PDFs and pictures never change with the font, and neither do
the icons; the share pages in your browser use the same font (a page already open takes it when it
loads again). Gothic keeps its own pixel type, so the Font row is not there. Greek, Cyrillic and
other letters a font lacks come from Terminess, and Japanese from DotGothic16, as in every style.

## Your own fonts (Classic)

Copy `.ttf` or `.otf` files into `grimoire/fonts` over USB (a family unzipped into a folder of its
own is fine, one folder deep; up to 200 files, 24 MB each), then open **Settings > Font**. Under
**Your fonts** each file shows its name drawn in itself once Grimoire has checked it in the
background ("Checking 3 fonts..." stays on the sheet meanwhile, and a file still being copied
waits until it is whole), or its row is greyed and says why not:

- "Unreadable font." (not a font, damaged, or one whose letters draw nothing)
- "Too big (over 24 MB)."
- "No Latin letters." (a symbol font)
- "Use .ttf or .otf." (a web font)
- "Took too long to read."

Tap a font to use it: it applies at once and is saved. Bold comes from the font's own weight axis
if it has one, else from the Bold file of the same family beside it, else Grimoire thickens the
regular. A letter your font lacks comes from Terminess (its row says "Some letters come from
Terminess." when your language needs some), and kana and kanji from DotGothic16 unless your font
has them. Text in your font may be a little smaller or larger than in Terminess: Grimoire sizes it
so its capitals are about as tall and its words no wider, and a label that still does not fit ends
in "...".

### Whose fonts

Your font draws Grimoire's own words only (menus, names, the keyboard). Pages, paper labels, the
clock, icons, PDFs and pictures never use it, and neither do the share pages (your browser gets
Terminess): Grimoire never copies your font into a page, a PDF or anywhere else, and never sends
it anywhere. It stays on this Kindle. Use fonts you have the right to use.

### Back to Terminess

Tap Terminess in the Font sheet (while a font of yours is in use the sheet's own words are in
Terminess, so it can always be read; a tap on Terminess also takes your font away after a start in
Terminess that a crash made, so the next start does not try it again), or delete the font's file
over USB: Grimoire is in Terminess from its next start (a running Grimoire keeps the copy it
loaded), and the row says "File missing." until you pick another font or put the file back, which
brings the font back.

If a font ever stops Grimoire before its first screen is drawn, the next start is in Terminess by
itself, the start after tries the font once more, and after a second failure Grimoire leaves it
alone until the file changes.
