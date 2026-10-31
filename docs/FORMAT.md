# Grimoire file format

What Grimoire keeps on disk, byte by byte, and the rules every version follows so that notebooks
move freely between versions. The page format's short summary lives in `src/store.h`; this file is
the full reference for anyone reading or writing Grimoire files (the host exporter, a KOReader
plugin, or a project of your own).

The rule behind all of it: **carry what you do not understand.** Going back to an older Grimoire
is a one-file copy, so a notebook written by a newer version will be opened by an older one sooner
or later. 1.0.0-beta and 1.0.0-rc.1 (whose page code is identical) are in people's hands for good
and can never be fixed, so the rules below keep them in mind forever. From 1.0.0-beta.2 on, a
version keeps whatever it does not understand and writes it back unchanged, and it opens a page it
cannot fully read as read-only instead of setting it aside.

This file names these carry-forward rules after **1.0.0-beta.2**, the version they were written
for, and the last column of Moving between versions lists them under that name. 1.0.0-beta.2 was
never released on its own: the rules first shipped inside 1.1.0-beta.1, together with the 1.1
features. So every released version with these rules also knows the data 1.1 added: it draws
image papers, for example, and never calls them "Newer paper".

## Files

```
root/settings.ini                       the device's settings (below)
root/folders.ini                        "# Grimoire folders", then one folder name per line
root/sleep.pgm                          the pinned sleep screen (P5 PGM, full page)
root/notebooks/<id>/notebook.ini        one notebook (below)
root/notebooks/<id>/p<page id>.qpg      one page (below)
root/notebooks/<id>/thumb.pgm           page 1, about a third of the page size (P5 PGM)
root/notebooks/<id>/erased.txt          pages that may still hold erased ink (since 1.1; Compaction)
root/notebooks/<id>/p<id>.qpg.damaged-<time>   a page that could not be read, kept
root/notebooks/<id>/bg/<pic>.png        a picture the notebook's image papers use (1.1; Image papers)
root/notebooks/<id>/bg/index.ini        which content each picture holds (Image papers)
root/notebooks/<id>/bg/pages.ini        the image paper of each page that has one (Image papers)
root/notebooks/<id>/source.ini          the book a notebook annotates, page by page (1.1; Annotated pages)
root/notebooks/<id>/nav.ini             its named sections (1.2.0; Named sections)
root/notebooks/<id>/links.ini           links drawn on its pages (2.0; Links)
root/shortcuts.ini                      keyboard and controller mappings the owner edits (2.0; settings.ini)
root/controller.ini                     controller buttons learned in Settings (2.0; settings.ini)
root/gestures.ini                       touch, pen-button bindings and recorded finger shapes, over the built-ins (2.0; settings.ini)
root/live-keys.ini                      the keys a Live view browser sends, the owner edits (2.0; settings.ini)
root/touch-trace                        an empty file the owner makes: asks for touch-trace.txt (2.0)
root/touch-trace.txt                    the touch trace (docs/KINDLE.md), never exported (2.0)
root/bluetooth/                         the Bluetooth helper, unpacked by the owner, and its own data (2.0)
root/.update-failed                     an update KPM could not install: told at the next start, then removed (2.0)
root/templates/                         pictures the owner copies in to use as paper (Templates)
root/photos/                            photos the owner copies in, each to be a page's paper (2.0; Photos)
root/import/<Name>/                     page pictures to become a notebook at the next start (Imports)
root/import/.annotate/                  a page KOReader sends to annotate, and its request (Annotated pages)
root/trash/                             deleted notebooks and pages, moved here, never unlinked
root/PDFs/                              the PDF mirror (an export, not data)
```

`root` is `/mnt/us/grimoire` on the Kindle. A notebook id is the folder's name: letters, digits,
`-` and `_` only. Page ids are positive numbers; the order of the pages is the `pages` key of
`notebook.ini`, not the numbers. Every file is written atomically (`<file>.tmp`, fsync, rename).

## Pages: `p<id>.qpg`

Little-endian throughout. Floats are IEEE 754 single precision.

### The first 16 bytes (every version, forever)

| offset | size | field | meaning |
|---|---|---|---|
| 0 | 4 | magic | `QPG1` |
| 4 | 2 | u16 version | 1 or 2 today (see Version policy) |
| 6 | 2 | u16 paper | the paper, if it is one 1.0.0-beta knows (0 to 16) |
| 8 | 4 | u32 w | page width in pixels (1 to 20000) |
| 12 | 4 | u32 h | page height in pixels (1 to 20000) |

Any future version keeps these 16 bytes as they are, so even a reader that knows nothing else can
tell a page from a newer version apart from damage, and knows its size.

### Version 1 and 2

| version 1 offset | version 2 offset | size | field |
|---|---|---|---|
| | 16 | 4 | u32 paper: the full 32-bit paper value |
| 16 | 20 | 4 | u32 nstrokes |
| 20 | 24 | ... | nstrokes stroke records |
| ... | ... | ... | chunks, to the end of the file |

In version 2 the u16 at offset 6 says 3 (Sheet music) for a custom music layout, as every
version has written it, and 4 (Blank) for any other kind. Readers ignore it in version 2.

### Stroke record (64 bytes, then the points)

| offset | size | field | meaning |
|---|---|---|---|
| 0 | 1 | u8 tool | the tool id (see Registry) |
| 1 | 3 | u8 ext[3] | colour, layer, flags (see Registry); 0 0 0 for an ordinary stroke |
| 4 | 4 | u32 seed | drives the brush's jitter and grain; erase strokes use bits 24 to 31 |
| 8 | 52 | f32 params[13] | the tool's settings when the stroke was drawn, in ToolParams order |
| 60 | 4 | u32 npts | the number of points |
| 64 | 18 x npts | points | |

A point is 18 bytes:

| offset | size | field | meaning |
|---|---|---|---|
| 0 | 4 | f32 x | page pixels, already smoothed |
| 4 | 4 | f32 y | page pixels |
| 8 | 2 | u16 pressure | pressure 0..1, times 65535, rounded |
| 10 | 2 | i16 tilt_x | degrees from upright toward +x, times 100, rounded (-9000 to 9000) |
| 12 | 2 | i16 tilt_y | degrees from upright toward +y, times 100, rounded |
| 14 | 4 | u32 t_ms | milliseconds (only differences matter) |

The 13 parameters, in order, with the range every reader clamps them to:

| # | name | range | | # | name | range |
|---|---|---|---|---|---|---|
| 0 | size_min_mm | 0.10 to 12 | | 7 | tilt_min_deg | 0 to 85 |
| 1 | size_max_mm | 0.10 to 12 | | 8 | tilt_max_deg | 5 to 90 |
| 2 | opacity_min | 0 to 1 | | 9 | tilt_size | 0 to 4 |
| 3 | opacity_max | 0 to 1 | | 10 | tilt_opacity | 0 to 1 |
| 4 | pressure_floor | 0 to 0.5 | | 11 | grain | 0 to 1 |
| 5 | pressure_ceiling | 0.05 to 1 | | 12 | smoothing | 0 to 1 |
| 6 | pressure_gamma | 0.25 to 4 | | | | |

Readers also read a float that is not finite as 0 and clamp x and y to -10000..30000. Rendering
replays each stroke from its first point, so how a page looks depends on these bytes alone:
`tests/test_golden.c` holds renders recorded from 1.0.0-beta that every later version must
reproduce.

### Page size

Ruled, dot and graph paper and their margins are drawn in millimetres at the page's dpi (300 on
every Scribe) from the page's top left, and stroke widths are in millimetres: a bigger page has
more lines, not wider ones (music papers spread their staves over the page's height between
margins of a fixed size). Scaling a page's points therefore moves its ink off the paper it was
written on. The Scribe 3 and the Scribe Colorsoft are 1986x2648, the 2022 and 2024 Scribes
1860x2480 (6.8% apart): ink scaled from one to the other lands about two medium rules off near
the bottom of the page.

- **1.1 on:** a notebook keeps the page size it was made with (`page_w`, `page_h` in
  `notebook.ini`), wherever it is opened. The editor fits the page to the screen (its width to the
  screen's width), as the sleep screen does; the Pages view, the thumbnails and every export are
  made at the page's own size. New pages get the notebook's size. A page file of another size
  than its notebook's (one a version before 1.1 scaled to another screen, below) has its points
  scaled back to the notebook's size when it is opened, which puts the ink written on the
  notebook's own screen back on its lines, and is written at that size after an edit. Exports
  scale it back the same way and leave its file as it is.
- **Before 1.1** (1.0.0-beta, rc.1): every page opens at the screen's size, its points
  scaled when the stored size differs, and a page edited there is written at the screen's size.
  Back in 1.1 such a page goes back to its notebook's size: the ink it had moves back onto its
  lines, and ink the older version added on the other Scribe moves by the ratio of the sizes.

### Erase strokes

From 1.1 on, a stroke with tool 3 is an erase stroke: white ink, or the precise eraser. It is
drawn in its place in the stroke order like any other stroke, but instead of putting ink down it
lightens each pixel it covers toward the paper under that pixel, never past it. So it takes away
the ink drawn before it, keeps the paper's own lines (rules, staves, a grid, dots), and leaves ink
drawn after it alone.

- **Texture.** Seed bits 24 to 31 name the drawing tool whose look the stroke has, plus 1:
  1 pencil, 2 mechanical pencil, 3 ballpoint, 5 highlighter, 6 fountain pen, 7 fineliner,
  8 marker, 9 charcoal, 10 calligraphy pen. 0 is the precise eraser's round hard tip. Any other
  value (4, which would be the eraser itself, or a tool a later version adds) draws the round tip.
  White ink keeps everything of its tool but the colour (width, nib, grain, pressure and tilt), so
  white pencil leaves the paper's tooth.
- **Seed.** The brush hashes seed bits 0 to 23 only (per-dab jitter, charcoal's specks).
- **Parameters.** The texture tool's settings as the stroke was drawn: for white ink that pen's,
  for the round tip the eraser's. How far past its points the stroke can draw (which sets the area
  a version redraws around it) is that tool's reach with those settings.
- **Blend.** A dab lightens each pixel it covers by its share, the way a dab of ink darkens one:
  its coverage spread over the n dabs that overlap on the stroke's spine (share = 1 - (1 -
  coverage)^(1/n), with coverage capped at 0.999), so that the stroke as a whole moves the pixel
  toward the paper by its coverage. The pixel becomes value + (paper - value) x share, rounded,
  only if that is lighter, and never lighter than the paper. Where the dab's own coverage is 0.999
  or more, the pixel becomes exactly the paper. Coverage, dab spacing and n are the texture tool's,
  as for its ink.
- **Paper.** The page's paper under the pixel, as the version draws it (a paper it does not know is
  blank: white). Drawn without its paper (the share studio's ink layer, `?layer=ink`), a page's
  erase strokes lighten toward white.

What an erase stroke takes away stays in the file, in the strokes before it. So a writer may leave
out of a page the strokes that do not change how it looks: ink that later erase strokes restore
entirely, and erase strokes that change no pixel, both checked on the page drawn with its paper
and on its ink layer (`page_droppable` in `src/page.h`). A stroke erased only in part stays whole,
with the erase strokes over it.

The editor never keeps an erase stroke that changed no pixel of the page as it was drawn (over
blank paper, or over ink taken away already): such a stroke is not stored and is no undo step.

### Compaction

Until it is left out, erased ink is in the page file: a zip of the notebook, a copy made with
`grimoire-pull` and any older version still find it. From 1.1 on Grimoire takes out the ink erased
entirely (every pixel of it, on the page and on its ink layer) when a notebook closes in the
editor, in a background process (on a heavily erased page this can take a minute on the Kindle).
A stroke erased only in part stays whole in the file, with the erase strokes over it: the usual
case when the precise eraser goes through part of a word written in one stroke, or when its soft
edge only lightens the rim of a stroke. The erased part of such a stroke can still be found in a
zip, a `grimoire-pull` copy or an older version.

- `erased.txt` in the notebook's folder lists the pages to look at, one page id per line in
  decimal (lines that start with `#` are comments, and a line that is not a page id is skipped).
  The editor adds a page when it writes the page with an erase stroke it did not have.
- For each page listed, the page file is read, the strokes `page_droppable` marks are left out
  (checked at the density Grimoire draws at, 300 dpi on every Scribe), and the page is written
  again: to `p<id>.qpg.compact.tmp`, renamed over `p<id>.qpg`, and only if the page file still
  holds, byte for byte, what was read (checked just before the rename; an older version or a copy
  from a computer may have written it since). The other strokes and the paper are written as any
  save of the page writes them, and the page looks exactly as it did.
- A page holding data this version reads past is left as it is: one that opens read-only (What a
  reader does with a page, below), or one with ext bytes on any stroke (other than a tip variant it
  knows, Tip variants below), an erase stroke whose texture byte is neither 0 nor a drawing tool
  + 1, a chunk of any kind after the strokes, or a paper it does not know. The version that wrote
  that data may draw the page otherwise (a colour, a layer, a texture that covers less than the
  round tip), so how 1.1 draws the page cannot tell what may be left out of it. A version that
  adds such data can count on 1.1 never rewriting a page that carries it.
- A page that is done goes off the list: compacted, with nothing to leave out, missing (there is
  no ink in it), or no longer in the notebook. A page compaction cannot judge stays listed, for a
  later pass or for the version that can: one that does not read, one of another size than its
  notebook's (compacted once the editor writes it at the notebook's size), and one holding data
  1.1 reads past. So does a page it could not write. The list goes once it is empty. A list that
  is there but does not read (a read error) is never written over: the editor tries again at its
  next save, and compaction leaves it as it is.
- Opening the notebook stops its compaction first, whether it waits or runs, and so does Grimoire
  leaving (a notebook it closes as it leaves is not compacted then): a page is never left half
  written (each is its old file or its new one), and the list keeps the pages that are not done.
  They are compacted the next time the notebook closes, or when Grimoire next starts.
- Compaction never changes how a page looks, so the notebook's PDF mirror (`PDFs/`) is the same
  whether it is written before or after; the two never run at the same time, and a reader of a
  page file sees it before compaction or after, never between.

1.0.0-beta and 1.0.0-rc.1 never read or write `erased.txt`: it stays as it is (and goes along
with the folder to `trash/`). A listed page one of them saves is compacted from what it wrote.
Pages deleted to `trash/` are not compacted.

### Chunks

After the last stroke record, a page may carry chunks, one after another, to the end of the file:

| size | field |
|---|---|
| 4 | tag: four ASCII letters or digits, the first a letter |
| 4 | u32 length of the data that follows |
| length | data |

The case of the tag's first letter says what a reader that does not know the chunk must do:

- **Uppercase: critical.** The page needs this chunk to be understood. A reader that does not know
  it opens the page read-only and never writes it.
- **Lowercase: safe to copy.** A reader that does not know it keeps the chunk and writes it back
  exactly as it was, after the strokes and in the same order, whatever else it changed on the page.
  So a safe chunk must stay true when an older version adds, removes or erases strokes, changes
  the paper, or scales the page to another size (its points are scaled and the new size written,
  while the chunk stays as it was): it must never refer to strokes by their index, hold page
  coordinates or depend on the page size. Data that would go stale belongs in a critical chunk.

Bytes after the strokes that are not chunks (a header cut short, a tag that is not letters and
digits, a length that runs past the end of the file) make the page read-only, never damaged.

### Version policy

- Version 1: the paper is one of the 17 fixed kinds 1.0.0-beta knows (0 to 16).
- Version 2: any other paper (17 and up: a custom music layout, or a kind a later version
  defines). Version 2 is written **if and only if** the paper is 17 or more. The threshold is 17
  for good, even after more fixed kinds are added.
- **Never write a version above 2, and never add a tool id, while 1.0.0-beta is in use.**
  1.0.0-beta and rc.1 reject such a page: they rename it to `p<id>.qpg.damaged-<time>` and show a
  blank page in its place, and they do that just by showing it (the Pages view's previews and the
  page-1 thumbnail open pages too). New data goes into the ext bytes, the erase-stroke seed bits,
  paper values and chunks instead: 1.0.0-beta opens all of those.
- Versions 3 to 255 are kept for a future format; a later version takes the next number. From
  1.0.0-beta.2 on, a reader opens such a page read-only, shown blank (it reads only the first 16
  bytes), and never writes it.
- Version 0, or 256 and above, is damage.

### Registry

**Tool ids** (the u8 tool). Never renumbered; no new id while 1.0.0-beta is in use.

| id | tool | | id | tool |
|---|---|---|---|---|
| 0 | pencil | | 5 | fountain pen |
| 1 | mechanical pencil | | 6 | fineliner |
| 2 | ballpoint | | 7 | marker |
| 3 | eraser: an erase stroke (draws nothing in 1.0.0-beta and rc.1) | | 8 | charcoal |
| 4 | highlighter | | 9 | calligraphy pen |

No version before 1.1 writes a tool 3 stroke (their eraser removes whole strokes), so giving tool 3
its meaning in 1.1 changed no existing page: an erase stroke, white ink or a precise eraser that
restores the paper (see Erase strokes).

**Seed.** For tools 0 to 9 except 3, the seed feeds the brush (per-dab jitter, charcoal dust). For
an erase stroke (tool 3), bits 24 to 31 hold its texture: the drawing tool whose texture it uses,
plus 1 (0 = a round tip), and a version that draws erase strokes hashes only the low 24 bits
(see Erase strokes). 1.0.0-beta keeps the seed byte for byte and never reads it for tool 3.

**ext[3]** (the three spare bytes of the stroke record). 1.0.0-beta writes 0 0 0 on a re-save, and
from 1.0.0-beta.2 on a version reads, keeps and writes them back. Since 2.0.0-beta.5 a version
reads ext[2]'s low 4 bits, the tip variant; nothing else in them is interpreted yet.

| byte | meaning | 0 means |
|---|---|---|
| ext[0] | colour: an index into one append-only palette (never reordered) | the tool's own gray |
| ext[1] | layer | the base layer |
| ext[2] | bits 0 to 3: the tip variant (below); bits 4 to 7: flags, every bit's meaning reserved | the tool's first tip; nothing |

**Tip variants.** A tool's tip, the shape of its dabs, changes only by a new variant, which strokes
drawn with it record. A page already written keeps its look in every later version, and tool ids
stay 0 to 9 (above). Variants are numbered per tool (the tool a stroke draws with: its own, or an
erase stroke's texture), append-only; a variant's look never changes. A version draws a variant it
does not know with the tip it gives that tool's new strokes.

| tool | variant | tip | |
|---|---|---|---|
| any | 0 | the tool's first tip, as every version drew it before variants | |
| highlighter (4) | 1 | a fixed vertical broad edge | 1.2.0 to 2.0.0-beta.4 drew every highlighter stroke with it, old ones too; recorded on strokes since 2.0.0-beta.5, which draws variant 0 round again |

**Paper values** (u32). The top four bits name the namespace.

| value | namespace |
|---|---|
| 0 to 16 | fixed kinds (below), append-only: a later kind takes the next number |
| `0x4.......` | a custom music layout (below) |
| `0x5.......` | an image paper: a picture of the page's notebook (1.1; Image papers) |
| `0x6.......` | reserved for a ruled-paper builder |
| anything else | unassigned |

| kind | paper | | kind | paper |
|---|---|---|---|---|
| 0 | Narrow rule (5.5 mm, margin) | | 9 | Guitar tab |
| 1 | Medium rule (7.1 mm, margin) | | 10 | Dot grid (5 mm) |
| 2 | Wide rule (9.0 mm, margin) | | 11 | Graph paper (5 mm) |
| 3 | Sheet music | | 12 | Cornell notes |
| 4 | Blank | | 13 | 4-string bass tab |
| 5 | Narrow, no margin | | 14 | 5-string bass tab |
| 6 | Medium, no margin | | 15 | 6-string bass tab |
| 7 | Wide, no margin | | 16 | 7-string bass tab |
| 8 | Grand staff | | | |

A custom music layout is `0x40000000 | systems << 24 | part[i] << 4*i`: 1 to 12 systems in bits
24 to 27 and up to six parts, top to bottom, in the low nibbles, ending at the first 0. Parts
(append-only): 1 treble staff, 2 bass staff, 3 alto staff, 4 tenor staff, 5 drum staff, 6 plain
staff, 7 to 10 tab with 4 to 7 lines, 11 lyric line.

An image paper is `0x50000000 | role << 24 | id` (Image papers, below): bits 24 and 25 the role
(0 a template, 1 a document page; 2 and 3 not assigned), bits 26 and 27 kept for colour (0), bits 0
to 23 the id of one of the notebook's pictures.

A paper a reader does not know (a value in an unassigned or reserved namespace, a music layout
with a part it lacks, an image paper with role 2 or 3 or a colour bit set, a fixed kind added
later) is drawn as blank paper and called "Newer paper" from 1.0.0-beta.2 on, and the page keeps
the value and writes it back (version 2).

**Chunk tags.** None are defined yet.

| tag | kind | meaning | since |
|---|---|---|---|
| | | | |

### Frozen invariants

- The first 16 bytes of a page file.
- ToolParams is exactly 13 floats in the order above (`tests/test_golden.c` checks its size).
- The parameter ranges above, in both directions, while 1.0.0-beta or 1.0.0-beta.2 is in use.
  Both clamp a stroke's parameters into these ranges when they read a page and write the clamped
  values when they save it. A narrower range would change how existing pages look, and a wider one
  would not survive: these versions draw a value past a range at the range's end and narrow it for
  good on their next save of the page. A stroke that needs a value past a range keeps its
  parameters in range and carries the rest elsewhere: in its ext bytes (a flag assigned in the
  Registry first) or in a critical chunk (a safe chunk cannot name strokes).
- Tool ids, fixed paper kinds and music parts are never renumbered or reordered.
- How an erase stroke draws (Erase strokes: the texture byte, the hash bits, the blend), from 1.1
  on: a page with erase strokes must look the same in every later version, as any page does.
- A page with no new data is written byte for byte as 1.0.0-beta writes it.
- The limits of `settings.ini` and `notebook.ini` given in their own sections below.

## What a reader does with a page (1.0.0-beta.2 on)

| the page has | opens as |
|---|---|
| a bad magic, fewer than 16 bytes, version 0 or 256+, an impossible size, strokes cut short | damage: renamed `p<id>.qpg.damaged-<time>`, a blank page in its place |
| version 3 to 255 | read-only, shown blank |
| a stroke with a tool id of 10 or more | read-only; those strokes are left out, the others shown |
| a critical chunk it does not know | read-only, everything it can draw shown |
| bytes after the strokes that are not chunks | read-only, everything it can draw shown |
| ext bytes, seeds, safe chunks, a paper it does not know | an ordinary page; all of it is written back |

A read-only page is never written, whatever happens on it: writing, erasing (the eraser, the pen's
eraser end, the side button), clearing it and changing its paper are refused with the message
"This page was made by a newer Grimoire. It is read-only here." (shown also when such a page is
opened), and saving leaves its file as it is. Deleting it still works (it goes to `trash/`).

When a notebook is opened in the editor (not when the Library lists it), every
`p<id>.qpg.damaged-<time>` whose `p<id>.qpg` is missing and that opens now, normally or
read-only, is renamed back to `p<id>.qpg` (the newest such copy if there are several). If
`p<id>.qpg` exists again (an older version wrote a fresh page there after it was edited), both
are kept. Only a page still in the notebook's `pages` list comes back: a copy of a page no longer
in it (deleted since, or missing from a page list rebuilt after damage) stays aside. A new page
never takes the number of a page file that is already there. Every recovery is logged in
`grimoire.log`.

## Image papers (1.1 on)

An image paper is a picture as a page's paper: a template the owner copied in, or a page of a
document they imported as pictures. Its paper value (Registry) names the picture by an id of 24
bits, and a role:

- **0, a template.** A page added after a page on it (the editor's next-page button, or a next-page
  key or controller button, at the end of the notebook, the page left when every page is deleted)
  gets it too.
- **1, a document page** (an imported page picture). The picture belongs to its own page: a page
  added after it gets the notebook's own paper instead (Blank if that is a document page's too).

### The pictures: `bg/<pic>.png`

The picture of id `<id>` is `notebooks/<notebook id>/bg/<pic>.png`, `<pic>` the id as 6 lowercase
hex digits. A page names only pictures of its own notebook. A picture is written once and never
changed, and its id never names another picture, so a page looks the same in every later version:
using a template on a page copies it into the notebook, and changing or deleting the template's
file later changes no page. A missing or unreadable picture draws white paper (and the page keeps
its paper value), and so does a file there that is no PNG, is bigger than 32 MB, or takes more than
96 MB to decode: Grimoire decodes a notebook's pictures as it draws them, and decodes nothing else
there.

A picture is a PNG of the notebook's page size (`page_w` x `page_h`), 8-bit gray, or RGB when its
source has colour; or, for an annotated page (below), the PNG KOReader's plugin wrote, kept as it is
when it is of the notebook's page size and one Grimoire draws as it is (any PNG within the limits
above: gray, palette or colour, 1 to 16 bits a channel, with or without transparency, which is drawn
over white). It is made from a source picture (PNG, JPEG or PNM; at most 8192 pixels a side
and 40 megapixels; a file of at most 64 MB, a template's at most 16 MB; and one that decodes in at
most 256 MB, the file's own bytes included, by what its header says decoding takes, so a 16-bit
colour PNG of more than about 20 megapixels or a progressive JPEG of more than about 35 is refused,
as is one whose data inflates past twice what its header says) like this: transparency over white;
16-bit channels cut to 8; then fitted inside
the page with its aspect kept (the fitted width or height rounded to whole pixels) and centred on
white, each page pixel the average of the part of the picture it covers, weighted by area, with
each axis's weights in 12-bit fixed point (they are rounded from their running total, so each adds
up to 4096) and the average rounded to the nearest value. A source of the page's own size is kept
as it is. A gray screen shows a picture's luma,
Y = (2126 R + 7152 G + 722 B + 5000) / 10000 rounded down (Rec. 709). A picture of another size
than the page it is drawn on (a notebook a version before 1.1 moved to a Scribe of the other size)
is fitted to it the same way, pixel by pixel from page coordinates.

### Ids: `bg/index.ini`

A writer may give a new picture any id that none of the notebook's pictures has (an id is taken
when `index.ini` names it, or when a file `<pic>.png` exists that `index.ini` does not name, whatever
it holds). Grimoire takes the low 24 bits of a 64-bit content key, stepping to the next id while
that one is taken by another key (0 is never given):

- for a page picture of an import, the key of the picture as stored (a hash of its width, height,
  channels and pixels): the same page picture twice in one import is one file;
- for a template, a key of the source file's bytes (a 64-bit hash of them, with their length) and
  the page size the picture is made for, so the id is known before the picture is made (that runs
  in the background): using the same file again gives the same paper value, and makes its picture
  again if its file went missing. The hash is the one the template's preview keeps (Templates).
- for an annotated page, the same key of the PNG KOReader sent (its bytes and the notebook's page
  size): the same page sent twice is one picture.
- for a photo (2.0), a key of the same inputs as a template's but of another kind (its own seed):
  a photo is turned upright before it is fitted, so the same file used as a template and as a
  photo makes two pictures.

`index.ini` is `# ...`, then one `<pic>=<key, 16 hex digits>` line per picture. The keys are
Grimoire's own; other writers can leave their pictures out of it.

### `bg/pages.ini`

`# ...`, then a `p<page id>=<paper value>` line for every page of the notebook whose paper is an
image paper. Every version from 1.1 on keeps it up to date: a save that gives a page an image paper
lists it, one that gives it another paper takes it off the list, and deleting a page takes it off.

It undoes what 1.0.0-beta does to such a page: 1.0.0-beta draws an image paper as blank paper and
writes a page it saves as version 1 Blank (also a page it only re-saves, as page 1 after a page
delete). A page that reads as Blank but that `pages.ini` lists with an image paper gets that paper
back when it is opened or exported: a page someone set to Blank in a version that keeps the list
is off it. One set to Blank from the picker of a version before 1.1 cannot be told apart, and gets
its picture back too. A page an older version gave another paper (from its own picker) keeps that
paper. When the notebook is opened in the editor, every page the list names that is still one of
its pages and reads as version 1 Blank gets its paper back and is written again with it, whether it
is visited or not (a page is read whole only when its first 8 bytes say version 1 Blank, so a big
notebook of imported pages opens without reading each page); an export changes no file. Once the
notebook closes, its thumbnail and PDF, which the older version drew on blank paper, are drawn
again. A page the list names that reads read-only (a newer version's) is left as it is. A
`pages.ini` that is there but does not read is never written over (its entries would be lost): it
is left as it is until it reads again.

Raw notebook zips carry the notebook's `bg/` (`raw/bg/...`).

### Templates: `root/templates/`

The pictures the owner copies in to use as paper (over USB, while Grimoire is closed): files whose
names end in `.png`, `.jpg`, `.jpeg` or `.pgm` (any case) and do not start with a dot. Grimoire
makes the folder at startup, with a `README.txt`, if it is missing, and keeps previews of the files
in `templates/.cache/`, each named by a hash of the file's name, size and time: `<hash>.pgm`, the
preview (P5 PGM, a sixth of the page); `<hash>.key`, a line `<hash of the file's bytes, 16 hex
digits> <width> <height>`, from which the template's id comes (Ids), so the file need not be read
again to use it; and an empty `<hash>.bad` for a file that is no picture it can use: its bytes are
no picture Grimoire takes (as above), or it is bigger than 16 MB. A file that does not read when
the previews are made (no memory for it, a read error) gets no `.bad`: a later scan tries it again.
A file whose path is 600 bytes or longer is left out.

New notebook and the editor's paper sheet offer the templates as tiles (their previews), behind
the last tile, More papers... (1.1). A page given one gets its image paper (role 0, a template) at
once and the picture is made in the background; until it is there the page draws white paper. A
notebook made on one in New notebook has that image paper as its own `paper` (`notebook.ini`), and
its first page has it too. The background job that makes a template's picture for a notebook also
draws the notebook's `thumb.pgm` again when its first page is on that picture (the editor may
have drawn it on white paper): to `thumb.pgm.card.tmp`, renamed over `thumb.pgm` only if that
still holds, byte for byte, what it held before the job read the first page (checked just before
the rename). The editor, which may have the notebook open, writes the card itself when it leaves
the first page or closes the notebook, saving the page first: if it did meanwhile, the job draws
the card again from the page as it is then, and after three tries leaves the editor's.

### Photos: `root/photos/` (2.0.0-beta.5 on)

Photos the owner copies in (over USB, while Grimoire is closed), named as templates are. Grimoire
makes the folder at startup, with a `README.txt`, if it is missing, and keeps previews in
`photos/.cache/` as for templates. The editor's paper sheet has a Photos... tile after More
papers...: it offers the photos as tiles, and a page given one gets an image paper on it at once,
a document page (role 1): the page's own picture, so a page added after it gets the notebook's
`paper`, not the photo. Its picture is made in the background as a template's is, but first turned
upright by the JPEG's EXIF orientation (tag 0x0112 of its APP1 Exif block, read within the file's
first 64 KB; none, or a PNG, is taken as upright), and so is its preview. Ids: as above. Older
versions draw such a page as any picture page (1.1 on), or blank (1.0.0-beta, which `bg/pages.ini`
puts right again).

### Imports: `root/import/<Name>/`

At startup, each folder `import/<Name>/` that holds page pictures (named as templates are) becomes
a notebook named `<Name>`, at the top level: one page per picture, in the natural order of the file
names (runs of digits compare by their value, so `page-2` comes before `page-10`; case does not
count), each page a document page; the notebook's own paper is Blank. It is made in the background:

- The notebook is built in `notebooks/.import-<notebook id>/` (a dot-folder: the Library and every
  older version skip it) and renamed into place once it is whole. Then the folder moves to
  `import/done/<Name>-<YYYYMMDD-HHMMSS>`.
- Every page's header is checked (it is a picture Grimoire takes, as above) before any picture is
  made, so a folder with a page that is not fails at once.
- A folder being imported holds `.grimoire-import`: the id of the notebook it becomes, or
  `done <notebook id>` when the notebook was made but the folder could not move (it is not imported
  again). A start that finds a mark with an id keeps that id: a notebook already made under it (an
  import cut off between its rename into place and the folder's move) is finished, its folder
  moved, and never made twice. An import that fails removes the mark.
- `notebook.ini` is the last file written into the dot-folder, after every other one was made
  durable (each file is fsynced), and the rename into place is made durable before the folder
  moves (the folders are fsynced). A dot-folder that holds `notebook.ini` is whole: the next start
  renames it into place. Any other is what an import that was cut off left, and is removed.
- `import/.lock` is held (flock) while an import runs, one at a time, also by the imports a session
  leaves to finish after Grimoire closes. A start that finds it held leaves the folders for the
  next start.
- A failed import (a picture that does not read, a full disk) or one cut off leaves no notebook and
  keeps its folder as it was. `import/.attempts` says how each folder still in `import/` fared:
  `# ...`, then a line `<hash of the folder's name> <hash of its pictures' names, sizes and times>
  <tries>` (16 hex digits, 16 hex digits, a number) for each folder an import tried, its tries
  counted as each starts (99 once a page was no picture Grimoire takes). A folder refused so, or
  tried at 3 starts (a full disk, an import cut off each time), waits until its pictures change: a
  file added, removed, renamed or replaced, or the folder renamed. A start drops the lines of
  folders no longer there. A start takes at most 8 folders; a folder of more than 2000 pictures is
  not imported, nor one without pictures, nor one whose path is 600 bytes or longer.

Grimoire makes `import/` at startup, with a `README.txt`, if it is missing.

### Annotated pages: `root/import/.annotate/` and `source.ini` (1.1 on)

KOReader's Grimoire plugin (`kindle/koreader/grimoire.koplugin`), in a book, has Tools > Grimoire >
Annotate this page in Grimoire (and a gesture that does the same). It draws the page the reader is
on into a picture of the Scribe's page, upright: for a paged document (PDF, DjVu, comics, pictures)
the whole page fitted and centred on white, at the reader's contrast, as the document has it (not
reflowed, its scan not cleaned up, not inverted for night mode); for a reflowable one (EPUB, FB2,
MOBI, TXT) the text as the screen shows it, crengine's own status bar at the top left white
(KOReader's footer is never in it), its pictures not inverted for night mode, and a screen turned
sideways fitted into the upright page. A
KOReader without the calls for that writes a screenshot of the reader instead (the screen's size and
shape, menus closed). It writes, into `import/.annotate/` (a dot-folder: the folder import skips it),
emptied first, the picture as a PNG and a request, each into a temporary file renamed into place, and
hands over to Grimoire as Open Grimoire does, with `--annotate` after `--hosted koreader`: the
launcher sets `GRIMOIRE_ANNOTATE=1` for that start only. A book whose file lies in the PDF mirror
(`root/PDFs/` or a folder in it, by any path that leads there) is refused before anything is
written: a notebook's own PDF would become a notebook of a notebook.

`request.ini`, one `key=value` a line (`#` lines are comments, a carriage return at a line's end is
dropped, keys it does not know are left alone):

| key | value |
|---|---|
| `version` | 1 (a request without the line counts as 1; any other version is refused) |
| `book` | the book's key: KOReader's partial MD5 of its file (as its book settings keep it), as `source` above |
| `title` | the book's title (its metadata's, else its file's name without the extension): a new notebook's name. Never logged |
| `page`, `pages` | the page the reader is on (from 1) and the book's page count, `page` at most `pages` |
| `kind` | `paged` or `reflowable` (lowercase letters) |
| `picture` | the picture's file name in `import/.annotate/`: letters, digits, `-`, `_` and `.`, ending in `.png`, not starting with a dot |
| `time` | when it was written, unix seconds (without it, the file's own time counts) |

At startup Grimoire looks for `import/.annotate/request.ini`, and empties the folder (and removes
it) whatever it does with what it finds:

- A start not asked to open it (no `GRIMOIRE_ANNOTATE=1`, or not hosted by KOReader: Grimoire
  started from the Library, or by Open Grimoire, after a hand-over that never reached it), or a
  request whose time is more than 5 minutes away (before, or after: the clock was set back), is
  deleted unopened, with one log line.
- The picture must read whole: a file of its own (a link is not followed), a PNG with all its
  chunks and nothing after its end, its critical ones (IHDR, PLTE, IDAT, IEND) matching their CRCs,
  and a picture Grimoire takes (Image papers), of at most 16 MB.
  Otherwise nothing is made, and a toast says so.
- The book's notebook is the newest one whose `notebook.ini` has its key as `source`, else the
  newest one with no `source` line whose `source.ini` names the book (one whose `source` line an
  older version dropped). Its compaction, queued or running, stops first, as when a notebook opens.
  With none, a notebook is made for the book: named after the title (on one line, control
  characters as spaces, one space between two words, broken UTF-8 and the characters Grimoire's
  fonts cannot draw left out (Chinese, Japanese and Korean: they would show as boxes), cut to 95
  bytes between two characters; Untitled, in the language in use, when nothing is left; with
  " 2", " 3"... added when a notebook has that name already, as two notebooks of one name would
  share a PDF), at the top level, at the device's page size, its one page the annotated one. It
  is built in `notebooks/.annotate-<id>/` (a dot-folder the Library and every older version skip)
  and renamed into place once whole, so a failure leaves no notebook; a start finds what one cut
  off left there, and removes it. Its own `paper` is Blank, as an imported notebook's: what a page
  added to it gets, and what a page whose file is missing or does not read shows (never another
  book page's picture). Its card in the Library is its thumbnail, which the editor makes as it
  leaves the page.
- The book page's page is the one `source.ini` maps it to, when that page is still in the
  notebook: it opens as it is (a page sent again is never a second page). Otherwise a new page is
  added on its picture, a document page (role 1), just before the first page of the notebook that
  holds a later book page (so pages written after an annotated page, notes on it, stay with it),
  else at the end. The page is listed last: its id is taken first (`notebook.ini` written with
  `next_page_id` past it, the page not listed, `modified` as it was), then its page file, its line
  in `bg/pages.ini` and its line in `source.ini` are written, and only then does `notebook.ini`
  list it. A start cut off on the way leaves files that name an id no version gives again
  (1.0.0-beta takes `next_page_id` as it is), never a page; the same book page sent again is a
  new page with the next id. A new first page takes the place of the old one on the Library's
  card: the thumbnail goes, and the editor makes the new page's as it leaves the page. Anything
  that cannot be written takes back what was added (all but the id, which stays given up): the
  notebook stays as it was, and a toast says so. A notebook of 2000 pages takes no more.
- The editor opens on that page (its `last_page` notes it, unchanged `modified` when the page was
  there already). A page added, or a notebook made, at that start is a change like any other: the
  notebook's PDF in `PDFs/` (the mirror, when it is on) is made again as it closes.

The picture becomes the page's picture as it is (moved into `bg/<pic>.png`) when it is of the
notebook's page size and one Grimoire draws as it is (a PNG of at most 32 MB, 6000 px a side, that
decodes within 96 MB); the page shows it at once. Any other (a notebook made on a Scribe of the other
size, a screenshot taken sideways) waits as `bg/<pic>.source.png` and is made into `bg/<pic>.png` in
the background, as a template's picture is (fitted, Image papers), after which the source is
removed, made or not; the page draws white until then, and is drawn again once it is made. When it
is the notebook's first page, the job also draws the card again, as a template's does (Image
papers: the editor made the card white if the page was left before the picture was made). A page
whose picture went missing (its fitting never ran) gets it again from a request for that page that
sends the same picture.

`source.ini` in the notebook's folder, written atomically each time a page is added:

```
# Grimoire: the book this notebook annotates, from KOReader (docs/FORMAT.md)
book=0f3c6e2a9b1d4c7e8f5a2b3c4d5e6f70
kind=paged
page.12.96=1
page.40.96=5
```

`book` and `kind` as in the request (from the first one), then a `page.<book page>.<page count>=<page
id>` line for each annotated page, in book order (by `(page - 1) / count`, then the page number, then
the count). The count is part of the book page: a reflowable book's page numbers change with its
font, margins and screen, so page 12 of 300 and page 12 of 340 are two pages, each with the picture
of what was on the screen then. The same page number with the same count is the same page, whatever
the layout: a layout that changed but kept the count opens the page made at the one before, with
its picture. A paged document's count never changes. A line whose page is no longer in the notebook
(deleted) is left out of the next write, and that book page is a new page if it is sent again. Lines
this version does not know are written back as they were: whole lines of up to 255 bytes, 1 KB in
all.

1.0.0-beta and 1.0.0-rc.1 never read or write `source.ini` or `import/.annotate/`: they stay as
they are (and `source.ini` goes with its notebook to `trash/`). The plugin hands over with
`--annotate` only to a launcher whose `# Capabilities:` line says `annotate`, so an older Grimoire
never gets a request. Raw notebook zips carry `source.ini` as `raw/source.ini` (1.1.0-beta.2 on), so
a notebook put back from one keeps its book pages; they leave `bg/<pic>.source.png` out.

### Whole books: `root/import/.book/` (2.0.0-beta.5 on)

In a paged document (PDF, DjVu, comics, XPS), KOReader's Grimoire plugin has Tools > Grimoire > Open
this book in Grimoire (and a gesture), and the file browser's Open with... lists Grimoire for those
files (never as a default). The plugin reads the book notebook's `source.ini` (above) for the book
pages it holds already, draws every other page as Annotate draws its page (whole, fitted and centred
on white, at the reader's contrast and saturation, not reflowed, cleaned up or inverted; from the
file browser, at the ones the reader would open the book with: the book's own, else the defaults set
for books, else KOReader's), each into
`import/.book/page-<n>.png` (a dot-folder: the folder import skips it), emptied first, each written
into a temporary file and renamed into place, under a progress message a tap stops. Then it writes
the request and hands over with `--book` after `--hosted koreader` (`GRIMOIRE_BOOK=1` for that start
only), to a launcher whose `# Capabilities:` line says `book`. A stopped or failed run leaves the
folder empty and hands nothing over. A book of more than 2000 pages, or one in the PDF mirror, is
refused before anything is drawn.

`request.ini` as Annotated pages' (the same rules for a line, `book`, `title`, `kind` and `time`),
with:

| key | value |
|---|---|
| `version` | 1 |
| `pages` | the book's page count, at most 2000 |
| `page` | the page to open, from 1, at most `pages`: the one the reader is on, or the one KOReader left the book at |
| `picture.<n>` | `page-<n>.png`, that name and no other, once for each book page `n` sent (1 to `pages`) |

At startup (after `import/.annotate/`) Grimoire takes `import/.book/request.ini` as it takes an
annotated page's request: not asked to open it, or more than 5 minutes away, it is deleted
unopened; the folder is emptied and removed once the request is done with, whatever came of it. The
book's notebook is found as an annotated page's is, its compaction stopped, or one is made for the
book as above (from the request's title), when at least one page came. Then every page sent that
`source.ini` does not hold yet is added on its picture, as an annotated page's is: in book order
among the pages there (just before the first page that holds a later book page, so notes stay after
their page), a picture of the page size moved into place as it is, any other fitted in the
background (one at a time while the background queue is idle). A page `source.ini` holds already is
left as it is, and its picture is deleted.

Pen and touch wait while the pages are added, a few at a time between screen updates (a toast says
how many of how many), so a book of any size keeps the launcher's heartbeat going. The ids of every
page it may add are taken first (`notebook.ini`'s `next_page_id` past them, nothing listed), then
each page's picture and file are written, and at the end `bg/index.ini`, `bg/pages.ini`,
`source.ini` and `notebook.ini` are each written once, the last listing the new pages (a new
notebook: built in `notebooks/.annotate-<id>/` and renamed into place). A picture that does not
read, a file that cannot be written, or more pages than a notebook holds, takes back every page file
and picture it wrote (the ids stay given up): the notebook stays as it was, and a toast says why.
The editor then opens on the page to open (else the first page added).

1.0.0-beta to 2.0.0-beta.4 never read `import/.book/`; the plugin never sends to them.

## Named sections: `nav.ini` (1.2.0 on)

An optional UTF-8 file inside the notebook directory. `name.<page id>=<title>` names a section
start; ids refer to `notebook.ini`'s page ids, not their current positions. The Index lists them
in notebook order. A name is a trimmed single line of at most 95 bytes, cut between UTF-8
characters; controls become spaces. An empty name removes it, and the last line for an id wins.
Names whose page was deleted are omitted on the next successful atomic save. A failed save
does not prune the in-memory names.

Unknown non-comment lines are carried in order, up to 255 bytes each and 16 KiB together, so
future navigation records survive. A file larger than 256 KiB, containing NUL, exceeding those
limits or the 2000-name limit, or unreadable/non-regular, disables name editing and is never
overwritten. Symlinks are also refused. Earlier Grimoire versions ignore this sidecar; page
ink, tool ids, page versions and stroke bytes are unchanged. Raw notebook ZIPs include it as
`raw/nav.ini`. From 2.0.0-beta.5 the notebook's PDF has the names as its outline (the table of
contents a PDF reader lists), in notebook order, each opening its page.

## Links: `links.ini` (2.0.0-beta.5 on)

An optional UTF-8 file inside the notebook directory: `# ...`, then one
`link.<page id>=<seed>,<target>` line per link drawn on a page. The seed is 8 hex digits (a reader
takes 1 to 8, either case); the target is `index` (the Index) or `p<page id>` (a page of the
notebook, the id from 1, no leading zeros). Current builds also write `n:<notebook ID>:p<page ID>`
for a page in another notebook. The ID is the notebook directory's stable ID, never a display name,
folder or arbitrary filesystem path; IDs allow only ASCII letters, digits, `_` and `-`, at most
23 bytes. Page IDs are nonzero decimal u32 values without leading zeros. Targets have a 64-byte
buffer including the terminator. Expanded targets are recognized only for this notebook-link
syntax; other long future target lines remain byte-exact unknown data. Older readers carry these
link and area lines but cannot follow the new destinations. New links cannot target `index`, but
existing `index` links remain valid. The last line for a page id and seed wins.

A link made by 2.0.0-beta.5 is the ink drawn around what links: the page's ink strokes (not erase
strokes) whose seed is the link's; from 2.0.0-beta.6 a new link is drawn with a lasso that leaves no
ink and has an area (below), and its seed is one no ink stroke on its page has. Its box is the box of their points, made at least 7 mm a side around its centre. A link
with an area (below) uses its area instead, clamped to the page and made at least 7 mm a side the
same way; an area that a smaller page leaves empty falls back to the ink. Where boxes overlap, the
link with the smaller box wins, and of two the same size the one with the lower seed (from
2.0.0-beta.6; 2.0.0-beta.5 follows the link whose last stroke comes later in the page). The strokes are ordinary
ink, so a version that does not read links shows them as they were drawn, and erasing, undoing or
compacting them away leaves the link with no box: it does nothing, and is kept (a redo brings its
box back) until its page is deleted. Grimoire gives every new ink stroke a seed that no ink stroke on
its page and no link of the page has, so a stroke never joins a link's box by chance. A link of a
page that is no longer in the notebook, and a link to such a page, is left out of the next save of
the file; a target this version does not know (any other run of 1 to 15 bytes from `!` to `~`) is
kept and does nothing.

From 2.0.0-beta.6 a link may also have an area, an `area.<page id>=<seed>,<x>,<y>,<w>,<h>` line
written right after its `link.` line: an axis-aligned rectangle in page pixels at the notebook's page
size, the same coordinates as the page's strokes. The page id and seed are as in a `link.` line; x,
y, w and h are 1 to 4 decimal digits each with nothing else, 1 <= w, 1 <= h and x + w, y + h <= 6000.
The last area line for a page id and seed wins, wherever it is in the file. An area line that does
not parse is carried as an unknown line (its link has no area); one whose page id and seed have no
link is left out of the next save. More than 4000 area lines make the file not read, as more than
4000 links do. A file whose links have no area is byte for byte what 2.0.0-beta.5 writes; beta.5
carries the area lines it does not read as unknown lines, within their limits below.

From 2.0.0-beta.6 a link is (page, seed, target, area): the area is what a finger's tap follows, and it is
never drawn in the page, its thumbnail, its PDF page image, the share page or the sleep picture (the
Links bar's Areas shows it as a dashed box). A link made by 2.0.0-beta.5 is migrated: as its page is
first read in a session (or, for a page not opened, on the editor's idle ticks, one page at a time), each
of its links with no area gets the box of the points of its ink strokes, without the 7 mm growth (the
minimum rounded down, the maximum up, at least 1 px a side, clamped to the page), and links.ini is
written after a pause. The ink stays on the page as ordinary ink: migration never removes or changes a
stroke, and deleting a link never deletes ink either. A link with no such stroke (its ink undone or
erased in beta.5) stays as it is; a read-only page's links are not migrated (their ink boxes keep
working). Copy page gives the copy every link of its page, with the same seeds, targets and areas.

From 2.0.0-rc.7 a snapped link area also stores an optional
`outline.<page id>=<seed>,<kind>,<x>:<y>,...` line. Kind is `c` (closed polygon) or `o`
(open path); coordinates are integer page pixels, 0 to 6000, within the link's rectangular area.
Closed paths have 3 to 64 points and open paths 2 to 64. The renderer closes a polygon itself.
Dense curves are simplified before saving. Outlines attach after all area lines are read,
regardless of file order; the last valid matching outline wins. Up to 4000 outline lines are
accepted. Unknown or malformed bounded lines are preserved; an oversized unknown line protects
the file from writes, as below. Outline records without a link are dropped on a successful save.
An outline outside its area's bounds is preserved as opaque metadata and the rectangle keeps
working. Copy, undo/redo, area redraw and clear retain or replace the full geometry together.
Outlines remain overlays; page ink, page previews and exported page images do not contain them.
PDF link annotations retain rectangular bounds.

Earlier 2.0 builds use the rectangular fallback when they can carry these new records within
their unknown-line limits. An outline line longer than 255 bytes, or unknown metadata over
16 KiB, makes links read-only in those builds; they preserve the file instead of overwriting it.
Reopen in rc.7 or newer to edit it. Notebook ink remains readable independently.

What a 2.0.0-beta.5 build does with this file: it reads every `link.` line, and a link with an area but
no ink does nothing there (one made from 2.0.0-beta.6 on), while a migrated link keeps working there
through its ink. It carries the `area.` lines as unknown lines up to 16 KiB together (about 430 links);
past that, or past 256 KiB, the file does not read there, so its links are read-only there and the file is
never overwritten. A beta.5 build that deletes a link leaves its area line, which this version drops as
an orphan at its next save; one that changes a link's target keeps it, and it still matches. links.ini
has no version line: a later kind of area comes as a key of its own, which this version carries.

Unknown lines are carried in order, up to 255 bytes each and 16 KiB together, as `nav.ini`'s are,
and a file that does not read (the same rules) disables adding and changing links and is never
overwritten; the section names go on working. The file is read and written up to 512 KiB (from
2.0.0-beta.6; 256 KiB before). There are at most 64 links on a page (a file may hold
more for a page: they are kept) and 4000 in a notebook (a file with more does not read).

Links are kept apart from `nav.ini` for the versions before: 1.2.0 to 2.0.0-beta.4 refuse a
`nav.ini` with more than 16 KiB of lines they do not know, which then shows them an empty Index,
and a notebook can have more links than that holds. They never read `links.ini`. Raw notebook ZIPs
include it as `raw/links.ini`, and the notebook's PDF has each link to a page as a link on its page
(a link to the Index has nowhere to go there).

## `settings.ini`

`key=value` lines; `#` starts a comment line. Readers strip one carriage return at the end of a
line. Values are clamped to their ranges when read.

| key | value |
|---|---|
| `current_tool` | the tool the pen draws with: a tool id, 0 to 9 |
| `draw_tool` | the drawing tool the toolbar's tool button shows: 0 to 9 |
| `panel_x`, `panel_y` | the floating panel's top-left, screen pixels (-1 = the default place) |
| `panel_collapsed` | 0 or 1 |
| `side_button` | holding the pen's button: 0 nothing, 1 eraser, 2 highlighter (ink, even with `white`), 3 white ink with the pen's drawing tool (with the eraser picked, the pen a tap on the eraser goes back to: `before_eraser`; since 1.1). Numeric and append-only: a new behaviour takes the next number (1.0.0-beta clamps the value to 0..2: it reads 3 as 2, the highlighter, and saves 2) |
| `eraser_mode` | the eraser (its button, the pen's eraser end, `side_button` 1): 0 takes whole strokes away (never an erase stroke: that would bring back the ink it covers), 1 is the precise eraser, which draws erase strokes with the round tip (Erase strokes) and the eraser's settings, its widths divided by the zoom (at least 0.10 mm) so that it keeps its size on the screen. Numeric and append-only (since 1.1; 1.0.0-beta drops it and its eraser takes whole strokes away) |
| `white` | 0 or 1: the drawing tools draw white ink, erase strokes with each tool's own settings and texture (since 1.1; 1.0.0-beta drops it) |
| `panel_tools` | the pens pinned to the editor's panel under its tool button, in pin order: their names as `<tool>.<param>` has them (below), comma-separated, e.g. `highlighter,fineliner`; empty for none, spaces around a name allowed. A reader keeps the drawing tools it has, each once, in the order given, at most 4: a name it does not have (a later version's pen), `eraser` (the eraser has its own button) and a repeat are left out, and the line is written back as it was until the pins change (below). The pen in use (`current_tool`) always has a button: one that is neither `draw_tool` nor pinned (its pin dropped) becomes `draw_tool` as a notebook opens. Since 1.1; 1.0.0-beta drops it on its next save (it has no pins) |
| `before_eraser` | the pen in use when the eraser was last picked on the panel: a tool id, 0 to 9 but the eraser's 3, or -1 for none (the default; any other value is read as -1 and written back as it was, below). A tap on the eraser in use goes back to it while it is on the panel (`draw_tool`, or pinned), else to `draw_tool`; while the eraser is in use the pen's button draws white (`side_button` 3) with that pen too. The pen's eraser end and its button never set it. Since 1.1; 1.0.0-beta drops it on its next save (its eraser goes back to `draw_tool`) |
| `mirror_pdf` | 0 or 1: write `PDFs/<folder>/<name>.pdf` when a notebook closes |
| `follow_rotation` | 0 or 1 |
| `invert` | 0 or 1: white on black |
| `pinch_zoom` | 0 or 1 |
| `sleep_mode` | the sleep screen: 0 the page I'm on, 1 a pinned page, 2 the Kindle's own, 3 KOReader wallpaper (since 1.1.0-beta.2); native PIN overrides the choice |
| `sleep_minutes` | sleep after this long without input, 0 to 240 (0 = never) |
| `http_port` | the share server's port, 1024 to 65535 (the launcher reads it too, the last such line) |
| `passcode_told` | 0 or 1: legacy notice flag from releases that paused the Kindle passcode; no longer used by native PIN support (since 1.1.0-beta.1; 1.0.0-beta drops it, and the notice shows once more) |
| `music_layout` | the last custom music layout built: a paper value (decimal) |
| `theme` | `gothic` or `classic` |
| `language` | the language of the screens (since 1.1): `auto` (the default, also when the key is missing: the Kindle's own language if Grimoire speaks it, else English), `en` or `es`. A code this version does not speak (a later one's) acts as `auto` and is written back as it was (below). 1.0.0-beta drops it on its next save, so a language picked in Settings goes back to Automatic |
| `status_on` | 0 or 1: the small clock and battery in a corner of the notebook page (`status_corner`; a tap there turns it on and off, and it shows on the page only, never on the Library or a sheet; docs/KINDLE.md, The clock and battery corner), on by default, small and top left. Since 2.0.0-beta.5; older versions keep the line as one they do not know |
| `status_size` | its size, 0 small, 1 medium, 2 large (the Display sheet's row), 1 (medium) by default since 2.0.0-rc.12 (0 before), and clamped to 0..2. Since 2.0.0-beta.5, as `status_on` |
| `gestures` | 0 or 1: touch patterns and one-finger gestures, on by default; off, the touch recognizer does not dispatch these actions. Pinch and pan remain available when pinch zoom is enabled. Since 2.0.0-beta.5; older versions keep the line as one they do not know |
| `landscape_fit` | 0 or 1: sideways, the editor shows the whole page (the page is as wide as the screen is tall, so it rests at the height of the screen), on by default; off, the page's width fills the screen as it does upright. Since 2.0.0-beta.5, as `gestures` |
| `hints` | 0 to 2147483647: a bitmask of the one-time tips already shown (bit 0 the first-open tip, 1 the first-link tip (from 2.0.0-beta.6), 2 the link areas tip, 3 and 4 a counter 0 to 3 of the Links bar's header hints, 6 the pan hint, 7 the finger-shape reject tip, 8 the sideways hint, which takes a notebook's first open from the first-open tip when it comes up sideways, so the tip comes at the next open; the other bits are for later versions), 0 by default. Since 2.0.0-beta.5, as `gestures` |
| `link_areas` | 0 or 1: dotted boxes over the page show where a finger tap follows a link (the Links bar's Areas button), off by default. Since 2.0.0-beta.5, as `gestures` |
| `status_corner` | the corner of the notebook page the clock and battery sits in (and where its tap zone is): 0 top left, the default, 1 top right, 2 bottom right, 3 bottom left (clockwise from the top left); clamped to 0..3. The Display sheet's row and the `status-corner` shortcut step it. Written after `link_areas`. Since 2.0.0-beta.5, as `gestures` |
| `rotation_lock` | 0 follows the Kindle's rotation; 1..4 locks to the current orientation (stored as 1 plus the rotation value). The `rotation-lock` action changes it; sensor turns are ignored while locked. Written after `status_corner`. Since phase 3; earlier versions carry the unknown key. |
| `gesture_new_page` | 0 or 1: two fingers left (the next page) on the last page add a page, as the next-page key does, on by default; off, the gesture only says it is the last page (Display sheet, under Sideways; the `gesture-new-page` action). Written after `rotation_lock`. Since 2.0.0-beta.5, as `gestures` |
| `whatsnew_seen` | the version whose What's new notes were last shown or due (`2.0.0-rc.11`): the first start after an update opens them once over the Library, when the file was saved by a Grimoire before (it has `settings_version`) and this names another version. A file without `settings_version` (a fresh start, or one written by hand) records the running version as seen. Written after `shape_min_mm`. Since 2.0.0-rc.11, as `gestures` |
| `whatsnew_hide` | 0 or 1: What's new's Don't show this again, off by default; on, an update opens no notes by themselves (the bat in the Library's top bar still opens them). Written after `whatsnew_seen`. Since 2.0.0-rc.11, as `gestures` |
| `shape_corner` | 0 to 100: Smart shapes' Corner roundness, the corners of a snapped rectangle, square, quad or triangle rounded by this percentage of the largest radius the shape holds (one radius for every corner; at 100 the arcs at the ends of its tightest edge meet in its middle, so a square is a circle), 0 (Sharp) by default, clamped to 0..100. The arcs are part of the stroke's points (the stroke is ink); link areas keep sharp corners. Written after `whatsnew_hide`. Since 2.0.0-rc.12, as `gestures` |
| `status_mode` | Retired status-strip preference. New saves write 1 (corner). Legacy 0 becomes the small enabled top-left corner, 2 becomes an off corner. There is no reserved strip or mode selector. `status-mode` is a compatibility alias for the corner toggle. |
| `shortcuts_enabled` | 0 or 1: keyboard and controller shortcuts, off by default (`shortcuts.ini`, below). Since 2.0.0-beta.1; 1.1.0-beta.1 and 1.1.0-beta.2 keep the line and have no shortcuts, 1.0.0-beta drops it on its next save |
| `controller` | 0 or 1: start the Bluetooth helper with Grimoire (Settings > Controller > Bluetooth controller; docs/SHORTCUTS.md), off by default; it only starts while shortcuts are on too. Since 2.0.0-beta.5; versions before keep the line (from 1.0.0-beta.2 on) or drop it (1.0.0-beta), and start nothing |
| `updates` | `stable` (releases only) or `preview` (betas too): which of Grimoire's feeds Settings > Updates checks (docs/KPM.md, Updating). Absent: by the running version (a beta or RC: preview). Since 2.0.0-beta.5; versions before keep or drop it as above |
| `settings_version` | 3: which default-changing upgrades have run (2: the pencil and mechanical pencil defaults; 3: the ballpoint's smoothing, since 2.0.0-beta.4). Each moves a value still at its old default, in files from before its version |
| `pen.valid`, `pen.swap`, `pen.ax`, `pen.bx`, `pen.ay`, `pen.by` | pen calibration (0/1, 0/1, then floats) |
| `pen.panel` | `WxH`: the panel the pen calibration was made on, upright (since 1.1; below) |
| `touch.valid`, `touch.swap`, `touch.ax`, `touch.bx`, `touch.ay`, `touch.by` | touch calibration |
| `touch.panel` | `WxH`: the panel the touch calibration was made on (since 1.1) |
| `<tool>.<param>` | a tool's setting: `<tool>` is `pencil`, `mech`, `ballpoint`, `eraser`, `highlighter`, `fountain`, `fineliner`, `marker`, `charcoal` or `calligraphy`; `<param>` is one of the 13 parameter names above |

A calibration maps the digitizer's axes (u and v, each 0 to 1, swapped when `swap` is 1) to the
pixels of the panel it was made on: x = ax * u + bx, y = ay * v + by. From 1.1 on, `panel`
records that panel, upright, and at startup Grimoire scales a calibration made on a panel of
another size (a grimoire folder copied from one Scribe to another) to its own: ax and bx by the
ratio of the widths, ay and by by the ratio of the heights. It logs that, and the next settings
save writes the new values with this panel's `panel`.

- A calibration with no `panel` (made before 1.1, or saved by 1.0.0-beta, which drops the key)
  counts as made on an 1860x2480 Scribe (the 2022 and 2024 Scribes, where nearly every
  calibration before 1.1 was made).
- Its own scale has the last word. A calibration only corrects the default mapping, which spans
  the panel exactly: it spans a panel when its |ax| and |ay| are within 2.5% of that panel's
  width and height (the two Scribe sizes are 6.8% apart). It was made on the first of these it
  spans: the recorded (or assumed) panel, this one, then 1860x2480 and 1986x2648; spanning none,
  the recorded one. So one recorded as another panel's that spans this one was made here and is
  kept as it is (a calibration made on a 1986x2648 Scribe before 1.1), and one recorded as this
  panel's that spans a Scribe's of the other size instead was made there and is scaled from it.
  Both catch a `panel` line gone stale, whichever way, such as one a version kept without knowing
  the key through a calibration of its own, on a Scribe of either size.
- A `panel` value that does not read as `WxH` (100 to 6000 each) is a line this version does not
  understand: kept while the calibration's panel is not recorded. At startup a valid calibration
  gets this panel's (it counts as not recorded until then, as above), and the next save writes
  that in place of the line.

From 1.0.0-beta.2 on, `settings.ini` carries what a version does not understand:

- A line whose key it does not know (a new key, an unknown dotted key such as a tool or a tool
  setting it lacks, a calibration field it lacks) is written back as it was read, after the keys
  it knows (up to 64 KB of such lines).
- A known key whose value it cannot use as written (a `side_button` number added later, a theme it
  lacks, a setting beyond its range, a music layout with a part it lacks, a language it does not
  speak) is used as the nearest value it has (for a language, `auto`), and written back as it was
  until a save finds the setting at another value. That save writes the new value, and from then
  on the setting is saved as set, also when it is set back. `http_port` is the exception: it is
  saved as used, because the launcher opens the firewall for the port it finds in the file.
- A `settings_version` above its own is written back as it is.
- The 0/1 keys (`panel_collapsed`, `mirror_pdf`, `follow_rotation`, `invert`, `pinch_zoom`,
  `passcode_told`, `white`, `shortcuts_enabled`, `status_on`, `gestures`, `landscape_fit`, `link_areas`, and the
  calibration's `valid` and `swap`) read any
  other value as on when it starts with a nonzero number and as off otherwise, and write it back
  as it was under the rule above.

1.0.0-beta drops all of this on its next settings save (and writes its clamped values).

`shortcuts.ini`, beside `settings.ini`, holds the keyboard and controller mappings, and the owner
edits it (its lines are in docs/SHORTCUTS.md). Grimoire writes starter mappings to it only when
shortcuts are turned on and the file does not exist, and never changes it after that; while the
file is missing the starter mappings apply, and an empty file means no mappings. 1.1.0-beta.2 and
earlier ignore it; 1.2.0 and the 2.0 betas read it. A line naming an action a version does not know
(2.0.0-beta.5 added fifteen) is refused and counted there, and the other lines still load. From
2.0.0-beta.5 on, a file that binds none of the D-pad's four directions (`pad.Up`, `Down`, `Left`,
`Right`: no earlier version read a D-pad, so none wrote them) gets the starter file's D-pad lines in
memory; the file is not changed.

`controller.ini` (2.0.0-beta.5 on), beside it, holds the controller buttons learned in Settings >
Controller > Buttons: `# ...` lines, then `pad.<button>=<action>` lines in `shortcuts.ini`'s grammar,
a button by name (`South`) or number (`316`). Grimoire writes it whole and atomically each time a
button is learned, and deletes it on Reset buttons. While it is there, the controller's buttons come
from it alone (`shortcuts.ini`'s `pad.` lines are set aside); a `key.` line in it is refused, as a
line `shortcuts.ini` cannot read is, and so is an action this version does not know. A learn writes
the file again from the lines that were read, so such a line goes then. Versions before
2.0.0-beta.5 never read it.

`live-keys.ini` (2.0.0-beta.6 on), beside them, holds the keys a browser showing Live view sends
to Grimoire: `key.` lines in `shortcuts.ini`'s grammar (a `pad.` line is set aside). Grimoire writes
it once, from the built-in keyboard mappings, the first time Live view starts and the file does not
exist, and never changes it after that; an empty file means no keys. It is read when Live view
starts, so an edit counts from the next start. It is independent of `shortcuts.ini`: editing one
never changes the other, and Live view keys work whether or not keyboard shortcuts are on. Versions
before 2.0.0-beta.6 never read it.

`gestures.ini`, beside the shortcut files, overlays touch and pen-button bindings and recorded
finger shapes on the built-ins. With no file, two fingers down open the Graph, up opens the Links bar, left
turns to the next page, and right to the previous page. The two-finger tap and one-finger directions
have no built-in action. Three-finger patterns are available only when the panel reports at least
three touch slots; the owner's Kindle reports two. The pen button's built-ins are
`pen.Button.hold=clear-page`; triple-click is unassigned. RC1 files with the duplicate triple/hold Clear pair load as hold only. Custom assignments remain available.

The reader accepts the `shortcuts.ini` line rules: `#` and `;` comments, trimmed `key=value`, later
duplicate keys win, case-insensitive keys, a 64 KiB file limit. The canonical writer starts with
these three comment lines, byte for byte:

```ini
# Grimoire: gestures (Settings > Gestures). Built in unless a line says otherwise:
# touch.2.down=index-map, touch.2.up=links, touch.2.left=next-page, touch.2.right=previous-page, pen.Button.hold=clear-page
# shape.ACTION=1,N,S1,S2[,S3]: a finger shape you drew; format 1, N samples; each sample is 32 points in drawing order, x then y as two hex digits, the larger side spanning 00-ff.
```

The live keys are:

- `touch.1.<up|down|left|right>` for a one-finger swipe; `touch.1.tap` is refused.
- `touch.<2|3>.<up|down|left|right|tap>` for two- and (on capable panels) three-finger patterns.
- `pen.<Button|Button2>.<tap|double|triple|hold|drag>` for pen-button gestures. `Button2` is live
  only when the pen reports BTN_STYLUS2.
- `shape.<action>=1,N,S1,S2[,S3]` for a recorded shape. `N` is 2 or 3 samples. Each sample
  stores 32 normalized points in drawing order as x then y, two hexadecimal digits per coordinate
  (128 hexadecimal digits per sample). The encoder uses lowercase hex; the decoder accepts either
  case. A value has exactly 261 bytes for two samples or 390 for three, without whitespace or an
  ending newline; each sample's larger side spans 00..ff. Degenerate samples (both extents under
  32 units) are refused. The line's action must allow the SHAPE source.

Touch and shape bindings accept registered actions that allow the matching source, or `nothing` for
a cleared built-in touch slot. `hold-eraser` is refused for touch and shapes because it needs a
release. Each action has at most one touch binding and one shape binding; shape lines are written in
action-id order. Touch lines are written in one-finger up/down/left/right order, followed by two-
finger up/down/left/right/tap, then three-finger in the same order.

Pen slots accept any registered action with the PEN source, or `nothing`. Every action allows
PEN, including `shortcuts` and `bluetooth-controller`. The momentary `hold-eraser` value is
legal only in `hold` and `drag` slots; it starts erasing at button press and ends at release,
without waiting for hold recognition. `pan` and `zoom` are registered PEN-only actions (internal
IDs 82/83), legal only in a pen `drag` slot. A bound drag owns the contact and leaves no ink;
an unbound or `nothing` drag keeps Settings' side-button ink choice. Built-in triple and hold
may both own `clear-page`; this deliberate duplicate is legal. Unsupported `pen.Button2.*`
lines and all `pen.Eraser.*` lines are carried verbatim, never shown or dispatched. The eraser
end continues to erase by contact, independently of gestures.

A click is a press of at most 450 ms with no nib contact. When double or triple is bound,
a single click waits 350 ms after release; otherwise it fires immediately. Double and triple
require each next press within 350 ms of the previous release, and resolve 350 ms after the
last release. A hovering hold fires once at 1200 ms. Nib contact while held begins a drag;
lift or release ends it. Leaving proximity cancels pending clicks and ends holds/drags.

After the comment header, the writer emits changed touch slots (`=nothing` for a cleared built-in),
then changed live pen slots (Button before Button2, tap/double/triple/hold/drag), shape lines,
and carried lines in their read order. Line endings are
LF. Unknown or newer-format lines are carried byte-for-byte, up to 1 KiB per line and 16 KiB total;
the file itself is bounded to 64 KiB. A `shape.<action>` value with a nonzero unsupported format
version is carried verbatim and shown as "finger shape (newer version)"; recording a replacement
warns before discarding it. Malformed or unsupported touch/shape lines are refused and
counted (`gestures: N line(s) of gestures.ini not used`). When the overlay has no changes, saving
removes `gestures.ini`.

Settings > Gestures reloads the map. A failed read leaves built-ins visible but marks the map
unwritable. Learning or clearing edits a copy and writes it atomically with `write_file_atomic`;
the in-memory map changes only after a successful write. Reset is the recovery path: it removes the
file even after an unreadable load, then restores the built-ins. Gestures do not require Enable
shortcuts. Saving, clearing, loading and resetting recompute pen multi-click waits from the live
map. Hardware capability survives reset. `.qpg` pages are unchanged by bindings or finger-shape trails.

The recognizer also writes a coordinate-free measurement after each grabbed two-finger session:

```text
touch: measure d0 %d along %d across %d spread %d%% fingers %d %d cos %d ms %d first-lift %d pen %d
```

`d0`, `along`, `across`, and the two finger travels are integer tenths of a millimetre; `spread` is
a signed percent, `cos` is the smaller direction cosine from 0 to 100, and the times are
milliseconds. `pen` is time since pen lift, or `-1` while the pen is near. The line contains no
coordinates and is silent while input is not grabbed. It follows the existing `touch: session` line.

A single finger on the notebook's ink that the recognizer took for the pen's writing hand (it sent the
notebook nothing) is named at its lift, after its `touch: fingertips` line:

```text
touch: a finger on the page was the pen's hand: the pen touched
touch: a finger on the page was the pen's hand: the pen was near
touch: a finger on the page was the pen's hand: the pen had left %u ms before
```

The editor's own lines follow link making and the contacts it ignores (at most 40 a minute, then
`editor: 40 contact and link lines this minute: the rest of it is not logged`):

```text
link: %s -> %s, no contact owned
link: %s -> %s, the pen|finger owns %s
editor: a pen|finger down ignored: the pen|finger owns %s
editor: a pen|finger down ignored: a pinch is under way
editor: a pen|finger down while the pen|finger owned %s: its lift was lost
editor: a lasso was left with no pen drawing it: back to the strip
```

When work held the app so long that the input queue (4096 events) filled, the next event it takes adds
`input: the event queue was full: %u events dropped, %u of them lifts` (a lift is a pen's up or leave, a
finger's up or a pinch's end: a gesture whose lift was dropped ends at the next down of its input, as
`its lift was lost` says).

The link-making states are `off`, `armed` (the strip is up), `lasso`, `pick` (Link to..., or what it
opened) and `naming` (New page...'s keyboard); the owners are those of the `editor.owner` probe
(docs/SIM.md): `draw`, `erase`, `panel`, `modal`, `display`, `pages`, `tools`, `build`, `templates`,
`link`, `bar`, `outside`, `find`, `lasso`, `card`, `linkto`, `strip`, `map` and `pen-drag`.

`bluetooth/` (2.0.0-beta.5 on) is not Grimoire's format: it is where the owner unpacks the
Bluetooth helper (kindle-hid-passthrough), whose files, paired devices and their keys live there.
Grimoire only reads whether its program is there, and on each start sets three lines of its
`config.ini`: its two `[paths]` into this folder, and `[media_remote] enabled = false`
(kindle/app/bluetooth.sh). Raw notebook ZIPs never include it.

Frozen while 1.0.0-beta is in use:

- The 0/1 keys stay 0 or 1: 1.0.0-beta saves any other value as 1 (or as 0, if it does not
  start with a nonzero number). A new state needs a new key.
- Tool settings are written with 4 decimals and calibration values with 6, by every version: a
  finer value comes back rounded on the next save.

## `notebook.ini`

| key | value |
|---|---|
| `name` | the notebook's name, one line. A missing or empty one is read as "Untitled" (from 1.1 in the language of the screens: "Sin título" in Spanish), which the next save writes, as new notebooks get a name in that language ("Cuaderno 20 oct") |
| `folder` | its folder, one line ("" = top level; one level of folders) |
| `paper` | the paper new pages get: a paper value (decimal); from 1.1 also an image paper of the notebook's (a template's, for a notebook made on one in New notebook) |
| `created`, `modified` | unix seconds |
| `last_page` | the page index opened next time |
| `next_page_id` | the id the next new page gets |
| `pages` | the page ids in order, comma-separated |
| `page_w`, `page_h` | the size of the notebook's pages in pixels, 100 to 6000 each: the screen it was made on (since 1.1; see Page size) |
| `source` | the book the notebook annotates: KOReader's partial MD5 of the book's file, 1 to 63 letters, digits, `-` or `_` (since 1.1; Annotated pages). A value of another form is a line this version does not know (kept, as below). 1.0.0-beta drops the line on its next save; `source.ini` names the book too, and 1.1 writes the line back when it next adds or opens a page for that book |

A `notebook.ini` without a page size (written before 1.1, or saved by 1.0.0-beta, which drops the
keys), or with a value outside 100 to 6000, takes the size most of its page files have, by their
first 16 bytes (any version 1 to 255; a page without them does not count), a tie going to the size
of the first such page in `pages` order; with no such page, the device's own. The next save writes
it. A notebook moved between Scribes by a version before 1.1 has its pages edited there at the
other Scribe's size (and 1.0.0-beta re-saves page 1 after a page delete), so it takes the size of
most of its pages: the pages of the other size are scaled to it.

From 1.0.0-beta.2 on, a line whose key it does not know is written back after the keys it knows,
whole lines up to 2 KB in all (a line that does not fit is left out and logged), and a `paper`
it does not know is kept: new pages get it. A new notebook always starts on a paper the version
offers: from 1.1 that may be a template. Such a notebook is made on the paper picked before it (or
Medium rule), and once it exists the template's picture is given its id in the notebook's `bg/`
(made there in the background), and the notebook's `paper` and its first page go onto that image
paper (the page listed in `bg/pages.ini`). If that cannot be done the notebook stays on the paper
it was made on. 1.0.0-beta drops unknown lines on its next save and reads an unknown `paper` (an
image paper too) as Medium rule.

Frozen while 1.0.0-beta is in use:

- A notebook's `name` and `folder`, and each line of `folders.ini`, hold at most 95 bytes of
  UTF-8. 1.0.0-beta cuts a longer one at byte 95, inside a character if one falls there, and
  writes it back cut; 1.0.0-beta.2 cuts it before the character that would cross byte 95, so
  what it writes stays valid UTF-8 (a name becomes the mirror PDF's file name).
- A notebook has at most 2000 pages: both versions keep only the first 2000 ids of `pages` and
  drop the rest from the list on their next save (those page files stay, in no page list).

## Where new data goes

- Data about one page: the stroke's ext bytes, an erase stroke's seed bits, the page's paper
  value, or a chunk.
- Data about a whole notebook: files of its own in the notebook's folder, which older versions
  never touch (as `bg/`, Image papers, `erased.txt`, the pages left to compact, and `source.ini`,
  an annotated notebook's book pages).
- `notebook.ini` and `settings.ini`: only what 1.0.0-beta may safely forget, since it drops
  unknown keys on its next save. (The page size, the calibration's panel, `eraser_mode`,
  `white`, `panel_tools`, `before_eraser` and `shortcuts_enabled` are such keys: what a version
  does without them is in their own sections.)

## Moving between versions

What 1.0.0-beta and 1.0.0-rc.1 (identical here) do with each kind of data a newer version may
write, and what the carry-forward rules (1.0.0-beta.2, at the top of this file) do with it. Data
marked 1.1 shipped in 1.1.0-beta.1 together with those rules, so for it the last column says what
1.1 does. Data marked 2.0 is newer than 1.1: there the last column says what 1.1.0-beta.1 and
1.1.0-beta.2 do with it. `tests/test_format.c` checks the page rows against a frozen copy of
1.0.0-beta's page reader and writer (`tests/beta_format.h`), and the carry-forward rules.

| new data | 1.0.0-beta, 1.0.0-rc.1 | 1.0.0-beta.2 |
|---|---|---|
| ext bytes (colour, layer, flags) | opens the page and ignores them; a re-save writes 0 0 0 | keeps them and writes them back |
| a highlighter stroke's vertical tip (ext[2] = 1, 2.0) | draws it round; a re-save writes 0, so it is round from then on, also in 2.0 | keeps the byte; 1.1 draws it round, 1.2.0 to 2.0.0-beta.4 draw every highlighter stroke vertical |
| an erase stroke (tool 3, 1.1) | opens the page and draws nothing for it, so erased ink shows again (all but the ink the writer left out) and white ink does not show; keeps the stroke on a re-save; its eraser can delete it | drawn (Erase strokes) |
| an erase stroke's texture (seed bits 24 to 31, 1.1) | kept, so white ink comes back unchanged in 1.1 after a re-save here | drawn |
| a safe-to-copy chunk | ignored; dropped on a re-save | written back as it was |
| a critical chunk | ignored; dropped on a re-save | the page opens read-only |
| bytes after the strokes that are not chunks | ignored; dropped on a re-save | the page opens read-only |
| a paper it does not know | blank paper; a re-save turns it into version 1 Blank | blank paper, "Newer paper"; kept (version 2) |
| an image paper (1.1) | blank paper, ink kept; a re-save turns it into version 1 Blank, and 1.1 brings it back from `bg/pages.ini` (Image papers), redrawing the thumbnail and PDF it made | its picture drawn (Image papers) |
| a notebook's `bg/` folder (1.1) | never read or written; its raw zips leave it out | read and kept up to date; raw zips carry it |
| an image paper as a notebook's `paper` (1.1: a notebook made on a template) | read as Medium rule, and saved so on its next save of `notebook.ini`: its new pages get Medium. Its pages keep their own papers (1.1 brings back from `bg/pages.ini` those it re-saved), so back in 1.1 only a page added after an imported page, or one whose file is missing, gets Medium | kept; new pages get it |
| `templates/`, `import/` in the data root (and `import/.attempts`, 1.1) | never read | read |
| `photos/` in the data root (2.0) | never read | never read; a page on a photo is drawn as any picture page |
| `links.ini` (links drawn on pages, 2.0) | never read or written: the ink around a link shows as ink, and a tap on it does nothing; a page it deletes leaves its links for 2.0 to drop | the same; 1.2.0 to 2.0.0-beta.4 never read it either (Links) |
| `area.` lines in `links.ini` (a link's area, 2.0.0-beta.6) | never read or written, as the file | the same; 2.0.0-beta.5 carries them as unknown lines within its limits, and a link with an area but no ink does nothing there (Links) |
| page version 3 to 255 | set aside as `.damaged-<time>`, a blank page in its place | read-only, shown blank; copies set aside earlier come back if the page is still in the notebook |
| a tool id of 10 or more | set aside, as above | read-only; the strokes it knows are shown |
| a stroke parameter past its range | drawn at the range's end; a re-save writes it narrowed | the same (see Frozen invariants) |
| an unknown `notebook.ini` key | dropped on the next save | kept |
| an unknown `notebook.ini` paper | read as Medium rule | kept; new pages get it |
| an unknown `settings.ini` key | dropped on the next save | kept |
| a `settings.ini` value it cannot use | clamped, and saved clamped | used as the nearest value, written back as it was |
| a 0/1 `settings.ini` key holding another value | saved as 1 (or 0) | used as on or off, written back as it was |
| `language=` (1.1) | dropped on the next save: 1.1 follows the Kindle's language again | used |
| a name or folder longer than 95 bytes | cut at byte 95, inside a character if one falls there | cut before the character that crosses byte 95 |
| more than 2000 pages | the ids past 2000 dropped from `pages` on the next save | the same |
| a newer `settings_version` | saved as 2 | kept |
| `page_w`, `page_h` (a notebook's page size, 1.1) | dropped on the next save (1.1 then takes the size most of its page files have); every page opens at the screen's size, its points scaled, and one it edits is written at that size | used (Page size) |
| `pen.panel`, `touch.panel` (1.1) | dropped on the next save; the calibration is used as it is, on any panel | used |
| `eraser_mode`, `white` (1.1) | dropped on the next save; its eraser takes whole strokes away and its pens draw ink | used |
| `panel_tools` (pens pinned to the panel, 1.1) | dropped on the next save (its panel has no pins): back in 1.1 the pens are pinned again by hand. A pinned pen in use stays the pen in use there, on no button of its panel; back in 1.1 it goes to the tool button (`draw_tool`) | used |
| `before_eraser` (the pen the eraser goes back to, 1.1) | dropped on the next save: its eraser, tapped again, goes back to `draw_tool`, as 1.1's does without the key | used |
| `side_button=3` (white ink on the pen's button, 1.1) | read as 2, the highlighter, and saved as 2 | used |
| `erased.txt` (pages left to compact, 1.1) | never read or written: it stays as it is | read and written (Compaction) |
| `source=` in `notebook.ini` (an annotated notebook's book, 1.1) | dropped on the next save; 1.1 finds the notebook through `source.ini` and writes the line back | used |
| `source.ini` (its book pages, 1.1) | never read or written: it stays as it is. A page it deletes leaves a line there that 1.1 drops, and that book page becomes a new page when it is sent again | read and written (Annotated pages) |
| `import/.annotate/` (a request from KOReader, 1.1) | never read; KOReader's plugin never sends one to a launcher without the `annotate` capability | read |
| `bg/<pic>.source.png` (a page picture waiting to be fitted, 1.1) | never read or written; raw zips leave it out | fitted in the background; raw zips leave it out |
| `shortcuts_enabled` (2.0) | dropped on the next save | kept; there are no shortcuts there |
| `shortcuts.ini` (2.0) | never read | never read |
| `controller` (2.0) | dropped on the next save | kept; there is no Bluetooth helper there |
| `controller.ini`, `bluetooth/` (2.0) | never read | never read; a helper someone else starts still makes its controllers into input devices, which 1.2.0 and 2.0.0-beta.1 to beta.4 read through `shortcuts.ini` |
| `gestures.ini` (2.0) | never read | never read: 1.2.0 to 2.0.0-beta.4 have no touch gestures (two fingers only pinch) |
| the `settings.ini` keys `gestures`, `landscape_fit`, `hints` and `link_areas` (2.0) | unknown lines: dropped on the next save | unknown lines: kept, so turning back to a build before them and forward again keeps what was set |
| a page with none of the above | written byte for byte the same | written byte for byte the same |

## Highlighter rendering in 1.2.0

1.2.0 gave the highlighter a fixed vertical broad edge for every stroke, old ones too: the stroke
record named no tip. Since 2.0.0-beta.5 each stroke records the tip it is drawn with (Tip
variants, above; [issue #5](https://github.com/ganten1998/grimoire-scribe/issues/5)). Strokes
from before it name none and draw with the round tip they were drawn with through 1.1, including
the ones drawn vertical in 1.2.0 to 2.0.0-beta.4, which cannot be told apart from them.

### Smart-shape settings

`shapes=1` enables smart shapes by default; `0` disables them. Values 2 and 3 are carried as future states
and used as enabled here. `shape_hold_ms=550` is the normal dwell time, clamped to 300..1500 milliseconds.
The tool picker cycles 400, 550 and 900 milliseconds. These keys are appended to the older settings.

### Nested Library folders

A folder name may contain slash-separated components, up to95UTF-8bytes in total. Components
are nonempty and cannot be `.` or `..`; a new folder is entered as one component relative to the
folder currently open. Ancestors of a notebook or an explicit empty folder are inferred when
listing. These are logical names: notebook directories and `.qpg` files do not move on a folder
rename or move. Older versions display a nested logical name as a flat folder.

### Index map cache: graph.ini

`notebooks/<id>/graph.ini` is disposable layout state. It carries the sorted graph generation
hash, All/Around mode, whether unlinked pages are shown, and integer world positions keyed by
page ID. Reserved pin entries and unknown bounded lines are retained. A file that cannot be read
safely is ignored and never overwritten that session. Raw notebook ZIPs exclude this cache.
Deleting it loses no ink, names, links or notebook content.

`graph.ini.lock` is an empty advisory-lock sidecar for map-cache transactions. It has no notebook
content, is excluded from raw exports with the map cache, and may be removed while Grimoire is
closed. It keeps an atomic replacement of graph.ini from invalidating the writer's lock.

Smart-shape settings: `shapes` (on by default), `shape_hold_ms` (550; menu choices 400/550/900),
`shape_quads` (0 uneven allowed, 1 rectangles/default, 2 squares only), `shape_round` (0 as drawn,
1 prefer circles/default, 2 circles only), `shape_corner` (0 to 100, Corner roundness; 0 sharp),
`shape_min_mm` (Smallest shape: the smallest closed shape a held stroke snaps to, screen mm across, 3 to 40, 10 by default; since 2.0.0-rc.12 the recognizer's own floor, which rc.9 to rc.11 could only raise above a fixed 10 mm, and their 0 for Any reads as 10) and `shape_links` (on by default). Prefer
circles uses minor/major axis ratio 0.72; As drawn keeps the recognizer's 0.88 threshold. An
ellipse's fit errors are held to its minor semi-axis but never less than half its major one, so a
long ellipse passes as surely as a round one. Uneven allowed keeps a quad whose corners are not all
within 15 degrees of square with its corners as found, opposite edges within 10 degrees of parallel
made parallel (both pairs: a parallelogram, one pair: a trapezium), and a parallel pair within
5 degrees of the page's axes put on them; a quad that is not convex stays as found. Link areas
retain rectangular bounds plus the optional bounded outline in links.ini; snapping never changes
QPG data.


## Disposable page previews: `.page-previews/`

Each notebook may contain small `p<page ID>.pgm` previews, at most 300 by 400 pixels, and matching
`.stamp` files. Stamps include the source page file's identity, size and precise modification time,
plus a template-generation marker. A changed page or template invalidates the preview. The UI reads
only bounded cached images; a background worker renders missing visible batches, capped at 64 pages.
Worker writes use process-specific temporary names, verify the source did not change during rendering,
and publish image then stamp. Preview errors never replace or modify page files. The entire cache
can be removed without losing notebook data; page IDs keep previews correct when pages are reordered.
