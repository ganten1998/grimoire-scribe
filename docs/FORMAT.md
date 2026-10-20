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

## Files

```
root/settings.ini                       the device's settings (below)
root/folders.ini                        "# Grimoire folders", then one folder name per line
root/sleep.pgm                          the pinned sleep screen (P5 PGM, full page)
root/notebooks/<id>/notebook.ini        one notebook (below)
root/notebooks/<id>/p<page id>.qpg      one page (below)
root/notebooks/<id>/thumb.pgm           page 1, about a third of the page size (P5 PGM)
root/notebooks/<id>/p<id>.qpg.damaged-<time>   a page that could not be read, kept
root/notebooks/<id>/bg/<pic>.png        a picture the notebook's image papers use (1.1; Image papers)
root/notebooks/<id>/bg/index.ini        which content each picture holds (Image papers)
root/notebooks/<id>/bg/pages.ini        the image paper of each page that has one (Image papers)
root/templates/                         pictures the owner copies in to use as paper (Templates)
root/import/<Name>/                     page pictures to become a notebook at the next start (Imports)
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
- **Before 1.1** (1.0.0-beta, rc.1, beta.2): every page opens at the screen's size, its points
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
| 3 | eraser: an erase stroke (draws nothing up to 1.0.0-beta.2) | | 8 | charcoal |
| 4 | highlighter | | 9 | calligraphy pen |

No version before 1.1 writes a tool 3 stroke (their eraser removes whole strokes), so giving tool 3
its meaning in 1.1 changed no existing page: an erase stroke, white ink or a precise eraser that
restores the paper (see Erase strokes).

**Seed.** For tools 0 to 9 except 3, the seed feeds the brush (per-dab jitter, charcoal dust). For
an erase stroke (tool 3), bits 24 to 31 hold its texture: the drawing tool whose texture it uses,
plus 1 (0 = a round tip), and a version that draws erase strokes hashes only the low 24 bits
(see Erase strokes). 1.0.0-beta keeps the seed byte for byte and never reads it for tool 3.

**ext[3]** (the three spare bytes of the stroke record). No version up to 1.0.0-beta.2 interprets
them; 1.0.0-beta.2 reads, keeps and writes them back.

| byte | meaning | 0 means |
|---|---|---|
| ext[0] | colour: an index into one append-only palette (never reordered) | the tool's own gray |
| ext[1] | layer | the base layer |
| ext[2] | flags; every bit's meaning is reserved | nothing |

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

- **0, a template.** A page added after a page on it (the editor's next-page button at the end of
  the notebook, the page left when every page is deleted) gets it too.
- **1, a document page** (an imported page picture). The picture belongs to its own page: a page
  added after it gets the notebook's own paper instead (Blank if that is a document page's too).

### The pictures: `bg/<pic>.png`

The picture of id `<id>` is `notebooks/<notebook id>/bg/<pic>.png`, `<pic>` the id as 6 lowercase
hex digits. A page names only pictures of its own notebook. A picture is written once and never
changed, and its id never names another picture, so a page looks the same in every later version:
using a template on a page copies it into the notebook, and changing or deleting the template's
file later changes no page. A missing or unreadable picture draws white paper (and the page keeps
its paper value).

A picture is a PNG of the notebook's page size (`page_w` x `page_h`), 8-bit gray, or RGB when its
source has colour. It is made from a source picture (PNG, JPEG or PNM; at most 8192 pixels a side
and 40 megapixels) like this: transparency over white; 16-bit channels cut to 8; then fitted inside
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
- for a template, the key of the source file's bytes and the page size the picture is made for, so
  the id is known before the picture is made (that runs in the background): using the same file
  again gives the same paper value, and makes its picture again if its file went missing.

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
paper.

Raw notebook zips carry the notebook's `bg/` (`raw/bg/...`).

### Templates: `root/templates/`

The pictures the owner copies in to use as paper (over USB, while Grimoire is closed): files whose
names end in `.png`, `.jpg`, `.jpeg` or `.pgm` (any case) and do not start with a dot. Grimoire
makes the folder at startup, with a `README.txt`, if it is missing, and keeps previews of the files
in `templates/.cache/` (P5 PGM, a sixth of the page each, named by a hash of the file's name, size
and time; an empty `.bad` file of that name marks a file that is no picture it can use).

### Imports: `root/import/<Name>/`

At startup, each folder `import/<Name>/` that holds page pictures (named as templates are) becomes
a notebook named `<Name>`, at the top level: one page per picture, in the natural order of the file
names (runs of digits compare by their value, so `page-2` comes before `page-10`; case does not
count), each page a document page; the notebook's own paper is Blank. It is made in the background:

- The notebook is built in `notebooks/.import-<notebook id>/` (a dot-folder: the Library and every
  older version skip it) and renamed into place once it is whole. Then the folder moves to
  `import/done/<Name>-<YYYYMMDD-HHMMSS>`.
- A folder being imported holds `.grimoire-import`: the id of the notebook it becomes, or
  `done <notebook id>` when the notebook was made but the folder could not move (it is not imported
  again).
- `import/.lock` is held (flock) while an import runs, one at a time, also by the imports a session
  leaves to finish after Grimoire closes. A start that finds it held leaves the folders for the
  next start.
- A failed import (a picture that does not read, a full disk) or one cut off leaves no notebook and
  keeps its folder as it was; the next start removes what a cut-off one left in `notebooks/` and
  tries again. A start takes at most 8 folders; a folder of more than 2000 pictures is not
  imported, nor one without pictures.

Grimoire makes `import/` at startup, with a `README.txt`, if it is missing.

## `settings.ini`

`key=value` lines; `#` starts a comment line. Readers strip one carriage return at the end of a
line. Values are clamped to their ranges when read.

| key | value |
|---|---|
| `current_tool` | the tool the pen draws with: a tool id, 0 to 9 |
| `draw_tool` | the drawing tool the toolbar's tool button shows: 0 to 9 |
| `panel_x`, `panel_y` | the floating panel's top-left, screen pixels (-1 = the default place) |
| `panel_collapsed` | 0 or 1 |
| `side_button` | holding the pen's button: 0 nothing, 1 eraser, 2 highlighter. Numeric and append-only: a new behaviour takes the next number (1.0.0-beta clamps the value to 0..2) |
| `mirror_pdf` | 0 or 1: write `PDFs/<folder>/<name>.pdf` when a notebook closes |
| `follow_rotation` | 0 or 1 |
| `invert` | 0 or 1: white on black |
| `pinch_zoom` | 0 or 1 |
| `sleep_mode` | the sleep screen: 0 the page I'm on, 1 a pinned page, 2 the Kindle's own |
| `sleep_minutes` | sleep after this long without input, 0 to 240 (0 = never) |
| `http_port` | the share server's port, 1024 to 65535 (the launcher reads it too, the last such line) |
| `passcode_told` | 0 or 1: the notice that the Kindle's passcode is paused while Grimoire is open was shown (since 1.0.0-beta.2; 1.0.0-beta drops it, and the notice shows once more) |
| `music_layout` | the last custom music layout built: a paper value (decimal) |
| `theme` | `gothic` or `classic` |
| `settings_version` | 2: which default-changing upgrades have run |
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
  Both catch a `panel` line gone stale, whichever way: 1.0.0-beta.2 carries the line through a
  calibration of its own, on a Scribe of either size.
- A `panel` value that does not read as `WxH` (100 to 6000 each) is a line this version does not
  understand: kept while the calibration's panel is not recorded. At startup a valid calibration
  gets this panel's (it counts as not recorded until then, as above), and the next save writes
  that in place of the line.

From 1.0.0-beta.2 on, `settings.ini` carries what a version does not understand:

- A line whose key it does not know (a new key, an unknown dotted key such as a tool or a tool
  setting it lacks, a calibration field it lacks) is written back as it was read, after the keys
  it knows (up to 64 KB of such lines).
- A known key whose value it cannot use as written (`side_button=3`, a theme it lacks, a setting
  beyond its range, a music layout with a part it lacks) is used as the nearest value it has, and
  written back as it was until a save finds the setting at another value. That save writes the
  new value, and from then on the setting is saved as set, also when it is set back. `http_port`
  is the exception: it is saved as used, because the launcher opens the firewall for the port it
  finds in the file.
- A `settings_version` above its own is written back as it is.
- The 0/1 keys (`panel_collapsed`, `mirror_pdf`, `follow_rotation`, `invert`, `pinch_zoom`,
  `passcode_told`, and the calibration's `valid` and `swap`) read any other value as on when it
  starts with a nonzero number and as off otherwise, and write it back as it was under the rule
  above.

1.0.0-beta drops all of this on its next settings save (and writes its clamped values).

Frozen while 1.0.0-beta is in use:

- The 0/1 keys stay 0 or 1: 1.0.0-beta saves any other value as 1 (or as 0, if it does not
  start with a nonzero number). A new state needs a new key.
- Tool settings are written with 4 decimals and calibration values with 6, by every version: a
  finer value comes back rounded on the next save.

## `notebook.ini`

| key | value |
|---|---|
| `name` | the notebook's name, one line |
| `folder` | its folder, one line ("" = top level; one level of folders) |
| `paper` | the paper new pages get: a paper value (decimal) |
| `created`, `modified` | unix seconds |
| `last_page` | the page index opened next time |
| `next_page_id` | the id the next new page gets |
| `pages` | the page ids in order, comma-separated |
| `page_w`, `page_h` | the size of the notebook's pages in pixels, 100 to 6000 each: the screen it was made on (since 1.1; see Page size) |

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
offers. 1.0.0-beta drops unknown lines on its next save and reads an unknown `paper` as Medium
rule.

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
  never touch (as `bg/`, Image papers).
- `notebook.ini` and `settings.ini`: only what 1.0.0-beta may safely forget, since it drops
  unknown keys on its next save. (The page size and the calibration's panel are such keys: what a
  version does without them is in their own sections.)

## Moving between versions

What 1.0.0-beta and 1.0.0-rc.1 (identical here) and 1.0.0-beta.2 do with each kind of data a
newer version may write. `tests/test_format.c` checks the page rows against a frozen copy of
1.0.0-beta's page reader and writer (`tests/beta_format.h`), and what 1.0.0-beta.2 does in every
row.

| new data | 1.0.0-beta, 1.0.0-rc.1 | 1.0.0-beta.2 |
|---|---|---|
| ext bytes (colour, layer, flags) | opens the page and ignores them; a re-save writes 0 0 0 | keeps them and writes them back |
| an erase stroke (tool 3) | opens the page and draws nothing for it, so erased ink shows again (all but the ink the writer left out) and white ink does not show; keeps the stroke on a re-save; its eraser can delete it | the same, with its ext bytes kept |
| an erase stroke's texture (seed bits 24 to 31) | kept, so white ink comes back unchanged in 1.1 after a re-save here | kept |
| a safe-to-copy chunk | ignored; dropped on a re-save | written back as it was |
| a critical chunk | ignored; dropped on a re-save | the page opens read-only |
| bytes after the strokes that are not chunks | ignored; dropped on a re-save | the page opens read-only |
| a paper it does not know | blank paper; a re-save turns it into version 1 Blank | blank paper, "Newer paper"; kept (version 2) |
| an image paper (1.1) | blank paper, ink kept; a re-save turns it into version 1 Blank, and 1.1 brings it back from `bg/pages.ini` (Image papers) | blank paper, "Newer paper"; kept (version 2). A page it adds after a page on one gets the same paper value, a document page's too, so in 1.1 that new page shows the page's picture |
| a notebook's `bg/` folder | never read or written; its raw zips leave it out | the same |
| an image paper as a notebook's `paper` | read as Medium rule: its new pages get Medium | kept; new pages get it (drawn blank there) |
| `templates/`, `import/` in the data root | never read | the same |
| page version 3 to 255 | set aside as `.damaged-<time>`, a blank page in its place | read-only, shown blank; copies set aside earlier come back if the page is still in the notebook |
| a tool id of 10 or more | set aside, as above | read-only; the strokes it knows are shown |
| a stroke parameter past its range | drawn at the range's end; a re-save writes it narrowed | the same (see Frozen invariants) |
| an unknown `notebook.ini` key | dropped on the next save | kept |
| an unknown `notebook.ini` paper | read as Medium rule | kept; new pages get it |
| an unknown `settings.ini` key | dropped on the next save | kept |
| a `settings.ini` value it cannot use | clamped, and saved clamped | used as the nearest value, written back as it was |
| a 0/1 `settings.ini` key holding another value | saved as 1 (or 0) | used as on or off, written back as it was |
| a name or folder longer than 95 bytes | cut at byte 95, inside a character if one falls there | cut before the character that crosses byte 95 |
| more than 2000 pages | the ids past 2000 dropped from `pages` on the next save | the same |
| a newer `settings_version` | saved as 2 | kept |
| `page_w`, `page_h` (a notebook's page size) | dropped on the next save (1.1 then takes the size most of its page files have); every page opens at the screen's size, its points scaled, and one it edits is written at that size | kept; the pages open, and edited ones are written, as with 1.0.0-beta |
| `pen.panel`, `touch.panel` | dropped on the next save; the calibration is used as it is, on any panel | kept, also through a calibration of its own (1.1 catches that by its scale, on a Scribe of either size) |
| a page with none of the above | written byte for byte the same | written byte for byte the same |
