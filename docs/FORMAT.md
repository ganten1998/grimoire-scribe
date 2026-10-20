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

Readers also read a float that is not finite as 0 and clamp x and y to -10000..30000. A page
stored at another size than the screen's is scaled when it is opened (the points are multiplied),
and written at the new size after an edit. Rendering replays each stroke from its first point, so
how a page looks depends on these bytes alone: `tests/test_golden.c` holds renders recorded from
1.0.0-beta that every later version must reproduce.

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

No released version has written a tool 3 stroke (the eraser removes whole strokes), so tool 3 is
free to mean an erase stroke: white ink, or a precise eraser, that restores the paper.

**Seed.** For tools 0 to 9 except 3, the seed feeds the brush (per-dab jitter, charcoal dust). For
an erase stroke (tool 3), bits 24 to 31 hold its texture: the drawing tool whose texture it uses,
plus 1 (0 = a round tip), and a version that draws erase strokes hashes only the low 24 bits.
1.0.0-beta keeps the seed byte for byte and never reads it for tool 3.

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
| `0x5.......` | reserved for image papers (1.1) |
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

A paper a reader does not know (a value in an unassigned or reserved namespace, a music layout
with a part it lacks, a fixed kind added later) is drawn as blank paper and called "Newer paper"
from 1.0.0-beta.2 on, and the page keeps the value and writes it back (version 2).

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
- A page with no new data is written byte for byte as 1.0.0-beta writes it.

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
| `music_layout` | the last custom music layout built: a paper value (decimal) |
| `theme` | `gothic` or `classic` |
| `settings_version` | 2: which default-changing upgrades have run |
| `pen.valid`, `pen.swap`, `pen.ax`, `pen.bx`, `pen.ay`, `pen.by` | pen calibration (0/1, 0/1, then floats) |
| `touch.valid`, `touch.swap`, `touch.ax`, `touch.bx`, `touch.ay`, `touch.by` | touch calibration |
| `<tool>.<param>` | a tool's setting: `<tool>` is `pencil`, `mech`, `ballpoint`, `eraser`, `highlighter`, `fountain`, `fineliner`, `marker`, `charcoal` or `calligraphy`; `<param>` is one of the 13 parameter names above |

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

1.0.0-beta drops all of this on its next settings save (and writes its clamped values).

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

From 1.0.0-beta.2 on, a line whose key it does not know is written back after the keys it knows,
whole lines up to 2 KB in all (a line that does not fit is left out and logged), and a `paper`
it does not know is kept: new pages get it. A new notebook always starts on a paper the version
offers. 1.0.0-beta drops unknown lines on its next save and reads an unknown `paper` as Medium
rule.

## Where new data goes

- Data about one page: the stroke's ext bytes, an erase stroke's seed bits, the page's paper
  value, or a chunk.
- Data about a whole notebook: files of its own in the notebook's folder, which older versions
  never touch.
- `notebook.ini` and `settings.ini`: only what 1.0.0-beta may safely forget, since it drops
  unknown keys on its next save.

## Moving between versions

What 1.0.0-beta and 1.0.0-rc.1 (identical here) and 1.0.0-beta.2 do with each kind of data a
newer version may write. `tests/test_format.c` checks the page rows against a frozen copy of
1.0.0-beta's page reader and writer (`tests/beta_format.h`), and what 1.0.0-beta.2 does in every
row.

| new data | 1.0.0-beta, 1.0.0-rc.1 | 1.0.0-beta.2 |
|---|---|---|
| ext bytes (colour, layer, flags) | opens the page and ignores them; a re-save writes 0 0 0 | keeps them and writes them back |
| an erase stroke (tool 3) | opens the page and draws nothing for it, so erased ink shows again; keeps the stroke on a re-save; its eraser can delete it | the same, with its ext bytes kept |
| an erase stroke's texture (seed bits 24 to 31) | kept | kept |
| a safe-to-copy chunk | ignored; dropped on a re-save | written back as it was |
| a critical chunk | ignored; dropped on a re-save | the page opens read-only |
| bytes after the strokes that are not chunks | ignored; dropped on a re-save | the page opens read-only |
| a paper it does not know | blank paper; a re-save turns it into version 1 Blank | blank paper, "Newer paper"; kept (version 2) |
| page version 3 to 255 | set aside as `.damaged-<time>`, a blank page in its place | read-only, shown blank; copies set aside earlier come back if the page is still in the notebook |
| a tool id of 10 or more | set aside, as above | read-only; the strokes it knows are shown |
| a stroke parameter past its range | drawn at the range's end; a re-save writes it narrowed | the same (see Frozen invariants) |
| an unknown `notebook.ini` key | dropped on the next save | kept |
| an unknown `notebook.ini` paper | read as Medium rule | kept; new pages get it |
| an unknown `settings.ini` key | dropped on the next save | kept |
| a `settings.ini` value it cannot use | clamped, and saved clamped | used as the nearest value, written back as it was |
| a newer `settings_version` | saved as 2 | kept |
| a page with none of the above | written byte for byte the same | written byte for byte the same |
