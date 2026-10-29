# beta5-links: links made by the beta.5 inked flow

Phase 2 migrates the owner's beta.5 links (an inked lasso whose stroke box is the link's box) to links.ini v2 areas
(PHASE2-BRIEF.md 1.3). Phase 2 also removes the inked flow, so this notebook was made once, before the removal, by
the phase 1 build, whose links.ini bytes equal the C beta.5's (rust/PORTING.md rule 1).

## How it was made

- Runner: `grimoire-headless` built at 72fafda (`claude/rust-index-p1`) with `make headless` in a separate worktree
  of that commit.
- Generator: `make.qs` beside this file (not a scenario; it is not under tests/scenarios), run the way
  `make scenarios` runs a file: a fresh `GRIMOIRE_ROOT`, its `# settings:` lines appended to settings.ini (plus an
  `http_port`), its `# copy:` line applied (tests/fixtures/index-keys), then
  `GRIMOIRE_LANG=en GRIMOIRE_ROOT=<root> GRIMOIRE_SCRIPT=make.qs grimoire-headless`, from the worktree's root. All
  its checks passed.
- Copied: `<root>/notebooks/20261028-210350-6efe/` with its `links.ini`, `nav.ini`, `notebook.ini` and page files.
  Not copied: `thumb.pgm` (the library's thumbnail, made again) and `erased.txt` (the compaction list: with it, the
  first start would compact page 1 and take link C's erased ring out of the page file).

The seeds in links.ini are random per run; making the fixture again gives other seeds, the same shape.

## What it holds

One notebook of 4 pages, page 3 named `Árbol` (nav.ini). links.ini orders a page's links by seed.

Page 1 (four links, three with their ink on the page):

| Link | Ring (`pen line`) | Goes to | Its ink |
|---|---|---|---|
| A | 300 1000 900 1200 | page 3 | on the page |
| B | 300 1400 900 1500 | page 2 | on the page |
| C | 1000 1000 1500 1200 | the Index | its stroke is on the page, covered whole by a precise erase stroke (eraser_mode=1) |
| D | 1000 1400 1500 1500 | page 4 | none: the ring was undone, the link is dormant |

Page 4 (two links): a small ring (600 1000 900 1100) to page 3, then a big ring around it (300 800 1500 1500) to
page 2. A tap inside the small ring follows the big one in beta.5 (the generator checks it: page 2); smallest wins
follows the small one (page 3).
