# Japanese support and a real input method (beta.2 Lane D study)

Scoping only: nothing here is merged code. Written 2026-11-02 against `e691c43` (main) and the
nebuff fork's `JS-Translation` at **`e623163`**, which is one commit newer than the brief's
`5ef0ae9`: on 2026-11-02 the contributor (AsumiChan) added a hiragana flick keyboard, "not tested
on real hardware yet". `git ls-remote` on the fork confirms `e623163` is its tip. The fork's
content was read as data; nothing in it tried to direct this study.

Sizes are measured, not guessed: the font and dictionary numbers come from fontTools subsets and a
packing prototype run on the real files, the code numbers from the contributor's diff and the
modules it touches. "Zip" means gzip -9 of the file, which tracks what the deflate in
`Grimoire.zip` adds.

## Summary

| Stage | What | Binary / zip cost | New code (from the code) | Verdict |
| --- | --- | --- | --- | --- |
| 1 | Take `ja.po`, DotGothic16 and the flick pad, with fixes | +1.56 MB / +0.61 MB (subset font + ja.mo) | about 150 to 200 lines on top of the contributor's 769 | Take it for beta.2, after a native read |
| 2 | Kana input: Bluetooth keyboards type at all, romaji to kana, katakana, Find folding | about +10 KB | about 700 lines with tests and the romaji table | Next; half of it helps every language |
| 3 | Kanji conversion: SKK dictionaries, a candidate bar, learning | +1.30 MB / +0.64 MB (M + ML + jinmei) | about 1,100 to 1,300 lines with tests | After stage 2; GPL-2.0-or-later data, compatible |
| - | Amazon's own Japanese keyboard | - | larger than stages 2 and 3 together | No (reasons below) |

Today's Kindle build is 8,839,304 bytes (`build/kindle/grimoire`, beta.1) and `Grimoire.zip` is
7,117,158 bytes, so all three stages together grow the zip by about 1.3 MB (18 percent) and the
installed app by about 2.9 MB (the dictionary as its own file). The Scribe (MediaTek MT8113, about 1 GHz, 1 GB RAM, 16 GB or more flash,
32-bit armhf userland) has room for that many times over.

## 1. Taking the translation and the font

### What the fork brings

| Part | Files | Size |
| --- | --- | --- |
| Translation | `po/ja.po` (880/880, compiles with `msgfmt --check`, `Plural-Forms: nplurals=1; plural=0;`), `po/ja.mo`, `po/ja-glossary.md`, TRANSLATING.md | ja.mo 72,736 B (25.9 KB zipped; es.mo is 64,039) |
| KOReader plugin | a `ja` table in `grimoire.koplugin/main.lua` (31 strings) | KOReader draws them with its own CJK fonts |
| Font | `assets/DotGothic16-Regular.ttf` + OFL text, a third fallback in `font.rs`/`font.c` after Terminess | 2,069,236 B |
| Language list | `i18n.rs`/`i18n.c`: `ja`, 日本語, one plural form | |
| Wrapping | `font.rs`: a CJK run is broken between characters and not reported as a word too wide | |
| Names | `ui_name_label` and `annotate_name` now keep kana and kanji (a Japanese book title stays) | |
| Flick pad | `keyboard.rs`/`keyboard.c` (+227/+164 lines), tests in both trees | |
| Tests | test_i18n, test_ui, test_font, test_keyboard, three scenario taps moved for the longer language list | |

The `po` header says `Last-Translator: draft`: the translation reads as a careful draft and
should be treated as one until a native speaker reads it.

### The font: measured

DotGothic16 1.100 (Fontworks, OFL 1.1, no Reserved Font Name, so a subset may keep its name)
has 9,362 glyphs, 8,231 code points: all of JIS X 0208 (2,965 level-1 kanji, 3,390 level-2 kanji,
the 524 non-kanji), 90 hiragana, 94 katakana, 164 fullwidth forms. Every one of the 515 non-ASCII
characters `ja.po` uses (371 kanji, all in level 1) is in it.

Subsets made with `fontTools.subset`, no hinting, layout and vertical tables dropped (Grimoire's
stb_truetype path uses neither GSUB/GPOS nor vmtx):

| Subset | Characters | TTF | Zipped |
| --- | ---: | ---: | ---: |
| Full file as the fork ships it | 8,231 | 2,069,236 | 807,744 |
| JIS X 0208 (levels 1 and 2 + non-kanji) + kana + fullwidth + UI | 6,895 | **1,485,220** | **587,848** |
| JIS level 1 + kana + fullwidth + UI | 3,505 | 688,284 | 277,860 |
| Kana + UI only | 672 | 105,236 | 44,441 |
| UI only | 515 | 89,044 | 38,321 |

Recommendation: **the JIS X 0208 subset (1.49 MB)**. The smaller subsets look tempting but fail
real names: 5.1 percent of the candidates in SKK-JISYO.ML need a level-2 kanji, and a KOReader
book title or a name typed on another device can carry any of them. The full file saves nothing
worth the extra 0.58 MB: what it adds beyond JIS X 0208 is mostly vertical forms and JIS X 0212
extras. Against the JIS X 0208 subset, every candidate in SKK-JISYO.L, ML and jinmei draws except
9 of 240,298 in L (symbols outside the kept rows). Keep the subset command in a small
`tools/subset-dotgothic.sh` and commit its output, as `po/*.mo` is committed, so building needs no
Python.

RAM: the font lives in the binary's read-only data and is paged in from flash as glyphs are
looked up. The glyph cache stays at 1,024 slots (`CACHE_SLOTS`, flushed at 768); a Japanese screen
uses 100 to 250 distinct glyphs per size, so a flush comes a little sooner but costs only a
re-render.

### Rendering: one fix worth making

DotGothic16 is a pixel font on a 16-pixel em (1000 units, about 62.5 units per design pixel), and
each "pixel" is drawn as a dot with a gap of about 11 units to its neighbour. The fork renders it
as an outline (its `K_UNIT` is 0), so at the UI's 34 and 44 px each design pixel is 2.1 or 2.75
device pixels: uneven dots, grey anti-aliased edges and faint grey seams, beside Jersey 10 drawn
razor sharp. Rendering it the way Grimoire draws its other pixel faces fixes that: rasterize at
exactly 16 px with coverage thresholded (each dot is 83 percent covered, so it comes out solid and
the gaps vanish), then blow up to whole k x k blocks, k = round(px / 16), so 44 px draws at 48 and
34 px at 32. About 30 lines in `font.rs` (the existing `K_UNIT` path assumes an integer grid in
font units; this face needs a pixel-height variant). Bold: do not overprint kanji (a one-pixel
overprint fills in dense characters such as 議 at a 16-pixel em); kana could take it, kanji stay
regular.

The y_all_screens shots (`build/shots/l10n/ja-gothic/`, `ja-classic/`) are where to look.

### Other things the review should fix

- `docs/FORMAT.md` needs two updates: the `language` row (852) lists `en` or `es`, so add `ja`.
  What older versions do with it is already covered: 1.2.0 and 2.0.0-beta.1 read an unknown code
  as `auto` and write it back, and 1.0.0-beta drops the key. The annotate paragraph (line 607)
  still says Japanese titles lose their kana and kanji; with the fallback they stay. In older
  versions such a notebook shows its name without what their fonts cannot draw (Untitled when
  nothing is left) and keeps the bytes in `notebook.ini`; that should be checked against the
  1.0.0-beta reader before it is written down. No `.qpg` change, no new tool id.
- Line breaking has no kinsoku: a line can start with 。、」 or end with 「. About 15 lines in
  `wrap_greedy`; polish, not a blocker.
- `make check` with `fit[ja]` lines: not run for this study. Applying the fork's tree in this
  worktree to run it was refused, and the lane may only work in its own worktree. It is the first
  thing the PR's gate shows.

### The flick pad (`e623163`): good bones, five bugs

The design is right: the accents key becomes あ in Japanese only, a 3 x 4 pad types on the lift
because the swipe chooses the kana, the key shows the kana the swipe would pick while the finger
moves, ゛ and 小 rewrite the last character (`change_last`), and the abc key goes back to the
letters. Because every text entry uses the one `Keyboard`, all ten entry points get it at once.
The bugs, all small:

1. **The abc key is labelled áé on the kana page.** `key_label` shows "abc" for `KB_KEY_PAGE`
   only on page 1; on page 4 it shows áé, though a tap goes to the letters. One line.
2. **The や key's swipes are not the phone's.** Directions are the vowels (tap a, left i, up u,
   right e, down o), so iOS and Gboard put ゆ up and よ down, with 「 and 」 left and right. The
   fork's table has ゆ left and よ up, so a phone user's up-swipe for ゆ types よ. One line of the
   table (わ, with を left and ん up, is right).
3. **No handakuten:** ぱ ぴ ぷ ぺ ぽ cannot be typed, nor ゔ. Phones use one modifier key that
   cycles: か to が and back, は to ば to ぱ, つ to っ to づ, や to ゃ. Make ゛ that key (labelled
   ゛゜ or 小゛゜) and the 小 key frees up for something else. About 40 lines with tests.
4. **No katakana.** A notebook called ノート or メモ cannot be named. Cheapest: a カナ key that
   flips the pad (each kana + 0x60), or turns the last run of hiragana into katakana. About 20
   lines.
5. **No 、。？！** on the pad. Phones put them on the key right of わ; ー can keep its tap and take
   、 。 ？ ！ on its swipes. About 10 lines.

Minor: the `TRANSLATORS:` comment sits on literal strings (あ, ゛, 小) that never go through
`tr()`, so xgettext never sees it; drop it. The slop of `ui_px(40)` on a 150 px key is sensible.

### How to shape the contributor's PR

- **Cut it from current main**, not from the fork's `main`. The fork's branch carries its PR #1
  merge, which brings `tools/kindle-dev`, `tools/kindle-dev-ssh`, the Makefile `dev`/`dev-ssh`
  targets and README support-status edits. Those belong to open PR #9 ("Create easy development
  tools over SSH & MTP") and must not ride along. Only docs changed on main since the fork's base
  (`c842240`), so the Japanese commits rebase cleanly.
- **Four commits**, each passing on its own: (1) the translation (`ja.po`, `ja.mo`, glossary,
  TRANSLATING.md, KOReader strings); (2) the font (the subset file, OFL text, the subset script,
  the fallback and pixel rendering, FORMAT.md and DESIGN.md); (3) the flick pad with the five fixes
  above and their tests; (4) CHANGELOG and BACKLOG.
- **Gate:** `make check`, with no `fit[ja]` line in either style. The PR description lists the
  screens a native reader should check in the shots: the top bar (New, Folder, Share must stay
  narrow enough for the scenario taps, as TRANSLATING.md says), Settings, Pen feel, the Index
  sheets.
- **Native read:** someone who reads Japanese goes through `po/ja-glossary.md` first (it lists the
  choices: ノート vs ノートブック, フォルダー, the paper names), then the tightest screens in the
  shots. Until then `Last-Translator` stays `draft` and the CHANGELOG says "draft translation".
- **The C tree:** the contributor mirrored the keyboard and font changes into `src/`. `make check`
  does not build the C tree (`c-check` is separate), so mirroring is welcome but should not hold
  the PR.

## 2. Input method options

### Where text is typed today

Every text entry is the one on-screen `Keyboard` (`keyboard.rs`, UTF-8, `KB_TEXT_MAX` 96 bytes, so
95 bytes of text: 31 kana or kanji), opened from ten places:

| Where | Title | Stored in | Limit |
| --- | --- | --- | --- |
| Library: new notebook, rename | name | `notebook.ini` `name=` | `STORE_NAME_LEN` 96: 31 kana/kanji |
| Library: new folder, rename folder, move to a new folder | Folder | `folder=` (a slash path) | the whole path under 96 bytes, so two levels of about 15 kanji |
| Pages view: page name | Page name | nav, `NAV_NAME_LEN` 96 | 31 |
| Pages view: Go to page | number pad | | |
| Editor: Graph page name, link card page name, Name the new page | Page name | nav | 31 |
| Find sheet (and Link to...'s Search) | Page number or name | not stored | 31 |

31 characters is plenty for Japanese names (a Japanese name carries two to three times what the
same count of Latin letters does), so no limit needs to change for names. Folder paths are the
tightest; leave them unless testers hit it.

**Bluetooth keyboards do not type at all today.** `hidkeys.rs` decodes presses into key codes
(`InType::Key`), and `app.rs` sends every one to `shortcuts_lookup`; nothing routes a key to an
open keyboard. So a Bluetooth keyboard can turn pages but cannot name a notebook, in any language.
That is the first piece of stage 2, and it helps everyone.

**Find matching** (`i18n_name_match` over `i18n_fold_next`) folds capitals and accents. It does
not fold katakana to hiragana or fullwidth letters to ASCII, so typing のーと will not find ノート.

### Romaji to kana

A table of about 230 entries (a, ka, kya, shi/si, tsu/tu, fu/hu, ja/zya/jya, small kana with x/l,
`nn` and `n'` for ん, a doubled consonant for っ, `-` for ー) and a longest-prefix converter with a
pending buffer of at most 3 ASCII letters. The standard tables are facts, not anyone's code; write
our own. About 250 lines of data, 120 of code, 80 of tests. Pending letters show underlined in the
field and count against the same 95 bytes.

It serves two inputs: Bluetooth keyboards (the main use), and, optionally, a "romaji" mode of the
on-screen letters page in Japanese for people who never learned flick.

### Bluetooth keyboards typing

- A keymap from evdev codes to characters for the US layout with shift (letters and digits are the
  same on a JIS keyboard; its symbol keys differ and can follow later), Backspace, Enter as Done,
  Escape as Cancel. A new `hidtext.rs`, about 120 lines.
- Routing in `app.rs`: when a keyboard modal is open, keys go to it before `shortcuts_lookup`
  (the editor already knows which of its five keyboards is open, line 1616), about 40 lines, plus
  a `keyboard_type(kb, cp)` / `keyboard_key(kb, code)` entry beside `keyboard_pointer`, about 40.
- Japanese keys: a JIS keyboard's 変換 (KEY_HENKAN 92), 無変換 (94), かな (93), 半角/全角 (85),
  and an Apple JIS keyboard's かな/英数 (LANG1/LANG2, KEY_HANGEUL 122 / KEY_HANJA 123) switch
  romaji on and off and, in stage 3, convert. On a US keyboard, a shortcut-free toggle
  (Ctrl+Space) does it.
- Nothing logs which keys were typed: the header of `hidkeys.rs` already promises "no key
  logging", and a name being typed is exactly that.
- Scenarios drive it headless with `key` events as the existing shortcut scenarios do. About 100
  lines.

### Flick layout

Stage 1 takes it (the contributor's pad with the five fixes).

### Kana to kanji: engines and data

| Option | Data on the Kindle | RAM | Quality for names | Speed on the MT8113 | Licence vs GPL-3.0-or-later |
| --- | --- | --- | --- | --- | --- |
| **SKK dictionaries, our own lookup** | S 53 KB, M 138 KB, ML 904 KB, M+ML+jinmei **1.30 MB** (0.64 MB zipped), L 3.92 MB, L+jinmei 4.19 MB (1.85 MB zipped), packed as below | the mapped pages a lookup touches (about 14 per lookup) | word by word, the user picks segments; hand-ordered candidates; no loanwords (the katakana chip covers those) | a lookup is a binary search over 4 to 12 K block heads: well under 0.1 ms; segmenting a full 31-character reading under 1 ms | **GPL-2.0-or-later: compatible** |
| Mozc (`dictionary_oss` + connection costs) | text sources 60 MB + a 36 MB cost matrix; compiled about 20 MB (Debian `mozc-server` installs 21 MB) | tens of MB | sentence conversion, the best open quality | fine once built | BSD-3, dictionary BSD-3 with the ICOT term: compatible |
| Anthy (anthy-unicode) | Debian `anthy-common` installs 25.5 MB | MBs | sentence conversion, dated | fine | engine LGPL-2.1+; dictionary (alt-cannadic) listed by Debian as GPL-2: if that is GPL-2-only it cannot be combined with GPL-3.0-or-later. Check upstream before any use |
| libkkc | Debian `libkkc-data` installs 30.6 MB (n-gram model) | tens of MB | sentence conversion | slower (trigram lattice) | GPL-3.0-or-later: compatible |
| Akaza (Rust, bigram + Viterbi) | model built from Wikipedia and Aozora; tens of MB; expects SKK-JISYO.L installed separately | tens of MB | good sentence conversion | fine | code MIT; model derived from CC BY-SA 4.0 text (one-way compatible with GPLv3) |

Porting Mozc, Anthy or libkkc means C++ or GLib builds against static musl armv7 and 20 to 30 MB of
data, for sentence conversion that names and Find do not need. They become worth it only if typed
text on pages arrives (section 3).

**The SKK choice, measured.** The dictionaries come from skk-dev/dict (EUC-JP, GPL-2.0-or-later
headers). Readings and candidates:

| Dictionary | Readings | Candidates | UTF-8 text | Packed | Zipped |
| --- | ---: | ---: | ---: | ---: | ---: |
| S | 3,379 | 6,763 | 73,810 | 53,440 | 30,722 |
| M | 8,346 | 12,826 | 194,881 | 137,597 | 72,118 |
| ML | 48,750 | 67,342 | 1,311,205 | 903,982 | 445,124 |
| M + ML + jinmei (names) | 64,598 | 104,555 | | **1,304,212** | **642,713** |
| L | 175,791 | 240,307 | 6,156,797 | 3,918,383 | 1,717,598 |
| L + jinmei | 185,121 | 266,336 | | 4,185,526 | 1,850,248 |

"Packed" is a prototype of the file Grimoire would read: readings sorted by bytes, front-coded in
blocks of 16 with a block index, each reading's candidates as one UTF-8 run, SKK's annotations
and Lisp forms dropped. Spot check, 29 words a notebook or page might be named (会議, 日記, 数学,
宿題, 料理, 講義, 授業, 計画, 予定, 漫画, 田中, 佐藤, 翻訳, 英語, 日本語, 仕事, 家族, 旅行, 研究,
論文, 試験, 復習, 楽器, and loanwords such as レシピ, ギター, プロジェクト): M finds 24, ML 23, L
24; every miss is a loanword, which the katakana chip types. ML is not a superset of M (M has 田中
and ML does not), hence M + ML + jinmei. L orders rarer words first more often (さとう gives 砂糖
before 佐藤, ゆうき 有機 before 勇気); learning fixes that for each user, but M + ML read better out
of the box.

**Recommendation: M + ML + jinmei, 1.30 MB**, as its own file in the app folder (read once into
memory, or mapped), so the binary does not grow and the file can be swapped for L + jinmei
(4.19 MB) if testers miss words. A small build tool turns the EUC-JP sources into the packed file;
commit the output with the sources' GPL notice, as `po/*.mo` is committed.

**Amazon's own Japanese keyboard: checked, and no.** What is documented (community reverse
engineering on MobileRead, firmware 5.14 on a Paperwhite 5 and Oasis): the Kindle keyboard is a
separate program, an X11 client, opened with `lipc-set-prop -s com.lab126.keyboard open
<appId>:<mode>:<flag>`; X applications that run under the framework's X server (evilvte,
Gargoyle) receive its keys, and Amazon's own apps get committed text over LIPC through properties
the app implements. Grimoire is not an X client: it draws to the framebuffer through FBInk, holds
the pen and touch devices with `EVIOCGRAB`, disables pillow and stops awesome while it runs
(KINDLE.md, "The Device Passcode"). Using the Amazon keyboard would need all of: linking Xlib into
the static musl binary and owning a focused X window; giving up the touch grab while it is up;
running awesome again so it can composite the keyboard; reconciling X's e-ink updates with
FBInk's; and implementing an undocumented LIPC IME protocol to get kanji rather than key codes,
which no third-party app is known to have done. That is more work than stages 2 and 3 together
and could break on any firmware update. Not verified on a device (this lane does not touch the
Kindle). If the owner wants certainty, `lipc-probe -v com.lab126.keyboard` on a Kindle lists
the keyboard's properties without changing anything.

### A candidate bar on e-ink

- **Place:** a strip about 110 px tall between the text field and the keys, inside the keyboard's
  own area, so it docks with the keyboard (`keyboard_set_dock`) wherever it sits. The pad's keys
  shrink from 150 to about 130 px to make room.
- **Pre-edit:** the unconverted reading shows in the field with a solid 3 px underline (dotted
  lines ghost on e-ink).
- **On demand, not live:** while there is pre-edit, the space bar reads 変換 (Convert) and fills
  the bar; nothing redraws the bar on every kana, which keeps e-ink flashing down. Done or 確定
  commits the reading as typed.
- **Chips:** up to about 8 per row at 1860 px (2 to 4 kanji each at 44 px), the first one
  inverted (black fill, the cleanest thing a fast update draws); a tap commits that chip; ‹ ›
  page through more. The last two chips are always the reading in hiragana and in katakana, which
  needs no dictionary (stage 2 can ship the bar with only those).
- **Segments:** if the whole reading has no entry, the longest prefix that has one is the first
  segment, shown boxed; ‹ › on the field shrink or grow it (SKK and Mozc both work this way);
  committing it leaves the rest as pre-edit for the next round.
- **Okurigana:** SKK's okuri-ari entries (かk + く = 書く) let verbs convert: the romaji layer
  knows the consonant; on the flick pad, the kana's row gives it.
- **Learning:** a chip picked moves to the front for its reading, in a small `user-dict.txt` in
  the data root (capped, say 2,000 lines, oldest dropped). A new file older versions never read,
  so no format risk; it goes into FORMAT.md's file list.
- **Refresh:** each bar change is one fast update of the bar's rectangle, as Find already flushes
  its rows (DESIGN.md, "Find typing").
- **Find:** the sheet matches committed text plus the pre-edit kana, with the stage-2 folding, so
  a page named in kana is found as it is typed.

## 3. If typed text on pages comes later

Names and Find need word conversion; text on a page would need much more:

- A **format addition**: a text object in the page file. No tool id of 10 or more, no header
  bump; it must live where the released readers drop it harmlessly (FORMAT.md's rules for what
  1.0.0-beta zeroes and drops), and every older version's view of such a page documented. That
  is its own lane.
- **Editing:** a caret, selection, multi-line layout with kinsoku, and a text buffer far beyond 96
  bytes (`KB_TEXT_MAX` is a name's).
- **Sentence conversion:** word-by-word picking gets tedious for sentences. Then L + jinmei plus a
  unigram or bigram cost model (Akaza-style Viterbi over SKK candidates, tens of MB) or Mozc's data
  becomes worth its size.
- **Export:** PDFs and PNGs would have to draw the text with the subset font, and undo stays one
  gesture per step (typing a word, one step at its commit).

None of that is needed for stages 1 to 3.

## 4. Staged plan

**Stage 1: the translation and the font (beta.2).** Contributor PR shaped as above, plus the
maintainer's fixes: the font subset (1.49 MB instead of 2.07), pixel rendering for DotGothic16
(about 30 lines), the five flick-pad fixes (about 75 lines with tests), FORMAT.md and DESIGN.md
lines, a native read of the glossary and the shots. Cost: binary +1.56 MB (font + ja.mo), zip
+0.61 MB. One lane, plus waiting for a native reader.

**Stage 2: kana input (beta.3).**
- Bluetooth keyboards type into the keyboard, every language: about 300 lines with scenarios.
- Romaji to kana, for Bluetooth keyboards and an optional on-screen romaji mode: about 450 lines
  with the table and tests.
- Katakana and hiragana chips (the bar's first form, no dictionary): about 150 lines.
- Find folds katakana to hiragana and fullwidth to ASCII (`i18n_fold_next`, and its C mirror):
  about 50 lines.
- Total about 700 to 900 lines, about +10 KB of binary. One lane.

**Stage 3: kanji conversion (beta.3 or later).**
- Build tool, packed dictionary format and lookup: about 300 lines.
- Segmentation, okuri-ari, conversion state in the keyboard: about 200 lines.
- The candidate bar (layout, draw, hits, paging, dirty rectangles): about 250 lines.
- Learning file: about 100 lines, plus its FORMAT.md entry.
- Tests and scenarios: about 250 lines.
- Data: M + ML + jinmei, 1.30 MB on the Kindle, 0.64 MB in the zip; L + jinmei (4.19 MB / 1.85 MB)
  if testers ask.
- Total about 1,100 to 1,300 lines. One lane, Rust only (the C tree need not mirror it; that is
  the owner's call).

Not planned: Amazon's keyboard; Mozc, Anthy or libkkc until typed text on pages is on the table.

## Sources

- Kindle Scribe hardware (MT8113, about 1 GHz, 1 GB RAM): [GoodEReader](https://goodereader.com/blog/?p=413807),
  [Currys listing](https://www.currys.co.uk/products/amazon-kindle-scribe-2024-10.2-ereader-premium-pen-16-gb-tungsten-grey-10273352.html),
  [TechInsights teardown (2022 model)](https://electronics360.globalspec.com/article/20961/techinsights-teardown-amazon-kindle-scribe).
  Amazon publishes neither; the hwtcon e-ink driver Grimoire talks to is MediaTek's.
- SKK dictionaries: [skk-dev/dict](https://github.com/skk-dev/dict) (files measured directly; GPL-2.0-or-later headers),
  [FreshPorts skk-jisyo](https://freshports.org/japanese/skk-jisyo).
- Mozc data and licence: [dictionary_oss README](https://raw.githubusercontent.com/google/mozc/master/src/data/dictionary_oss/README.txt),
  [Debian review](https://dfsg-new-queue.debian.org/reviews/mozc), [mozc-server package](https://packages.debian.org/sid/amd64/mozc-server),
  file sizes from the GitHub API on google/mozc.
- Anthy: [anthy-common](https://packages.debian.org/sid/anthy-common),
  [Debian copyright](https://metadata.ftp-master.debian.org/changelogs/main/a/anthy/unstable_copyright).
- libkkc: [libkkc-data](https://packages.debian.org/sid/libkkc-data),
  [Debian copyright](https://metadata.ftp-master.debian.org/changelogs/main/libk/libkkc-data/unstable_copyright).
- Akaza: [akaza-im/akaza](https://github.com/akaza-im/akaza).
- Amazon keyboard over LIPC: [MobileRead, Gargoyle keyboard on 5.14.2](https://www.mobileread.com/forums/showthread.php?p=4614982),
  [MobileRead wiki, Running X apps on Kindle Touch](https://wiki.mobileread.com/wiki/Running_X_apps_on_Kindle_Touch),
  [sixfoisneuf, Kindle internals](https://www.sixfoisneuf.fr/posts/kindle-hacking-deeper-dive-internals/).
- Flick layouts: [KOReader ja_keyboard](https://koreader.rocks/doc/modules/ui.data.keyboardlayouts.ja_keyboard.html)
  (AGPL-3.0: read for behaviour, do not copy code), the iOS and Gboard 12-key layout.
- DotGothic16: [fontworks-fonts/DotGothic16](https://github.com/fontworks-fonts/DotGothic16) (file measured directly).
