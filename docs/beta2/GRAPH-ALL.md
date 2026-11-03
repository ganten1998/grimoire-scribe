# The Graph of all notebooks (2.0.0-beta.2 design)

**Owner ruling (2026-11-02, the owner's words):** "I see no reason why the graph view shouldn't open with
a default of all nodes in existence from all notebooks. The view can have a "Notebook" but the default has
an all option which works like obsidian or emacs."

So the Graph opens on **All notebooks**: Obsidian's global graph, org-roam-ui. Every node in the library
is drawn, links within and across notebooks are its edges, and a **Notebook** chip narrows the view to
one notebook (removed by the owner's ruling of 2026-11-03: section 11). One notebook's view is exactly today's Graph, lane C's anchors included. Rust tree only
(rust/src); the C tree gets nothing (PORTING.md, as for backlinks). No page-format change. Two new
caches in the data root, both disposable.

## 1. What All contains

**Nodes.** In every notebook under `root/notebooks/` (folders included, annotated-book notebooks and
read-only pages included, the trash left out), a page is a node when it is **linked**: it has a link
out (to a page, to its notebook's Index, to itself, or to a page of another notebook that is still
there) or a link in (from any page of any notebook). This is the node rule the Graph has had since RC5
("Show only linked pages in the graph"), and the panel already says it in its own words:
`Nodes: N · Orphans: M · Links: L`. "All nodes in existence" therefore means every node of every
notebook. Orphans (pages with no link) are counted across the library and never drawn, as today.

*Decided by the owner (2026-11-03, the owner's words):* "the view for unlinked pages is beyond
unnecessary, only linked pages should be shown, and from all notebooks". A build that drew orphans (light
grey dots on a block per notebook, an Orphans toggle; commit 9a1bfef) was taken out again; its fix to the
library index (a notebook read from its files keeps its page names) stays. graph-all.ini lines that build
wrote (`orphans=`, a notebook's block at page id 0, `pos.<id>.0`) are dropped when read and never written
back. The panel still counts orphans (`Nodes: N · Orphans: M · Links: L`), as it did before.

**Edges.** One per pair of linked pages, as `graph_build` makes them: both directions, multiplicity,
self links counted but no edge, a page's links to one page of another notebook counted once. Targets:
`p<id>` resolves in the source page's notebook, `n:<notebook>:p<id>` in that notebook, `index` stays a
badge on the page (never an edge: every page linking its notebook's Index would make hubs). A link to a
page or notebook that is gone makes no node and no edge (`nav_for_graph`'s rule, now at library scale).

**Unlinked pages** at that scale: the satellites and the tray the task mentions have been dead code
since RC5 (`graph_build`'s `linked_only` is what the view uses; BACKLOG lists the removal). All builds no
satellites and no tray and has no Unlinked toggle.

**No per-notebook hub node.** A hub (the notebook as a node its pages hang from) was considered and
rejected:

- Obsidian's global graph draws no folder nodes (folders only colour groups, if you set them up), and
  org-roam-ui draws no file or directory nodes. A hub is not how either "works".
- A hub adds one fake edge per page: a notebook of 100 pages becomes a 100-spoke star, the hairball
  graph.rs already refuses for the Index badge, and on grayscale e-ink a solid black blob.
- It would make every page linked, which undoes the node rule above.

Grouping comes from layout and captions instead (section 6): each notebook gets an **island**, a
packed group of the components that mostly belong to it, with the notebook's name over it.

## 2. The Notebook control

*Removed 2026-11-03 (section 11): what follows is the design as first built.*

- **Where:** the header band between the Index tabs and the plot, where the notebook's name is drawn
  today (y 195 to 260). The name becomes a chip: a framed button reading **All notebooks** or the
  notebook's name, with a small down arrow. The band grows to 80 px (`ui_px`) for a finger-sized
  target; the plot starts 20 px lower.
- **Tap:** a small menu under the chip, three rows of 100 px: **All notebooks**; the editor's notebook
  by name (listed first, as asked); **Other notebook...**, which opens the existing folder browser
  (`NotebookPicker`) in a notebook-only mode: a tap on a notebook picks it, with no page step.
- **One notebook's view** is today's Graph built for that notebook: its `graph.ini`, its layout, its
  View: All / Around, lane C's anchors. For the editor's own notebook it is built from the live
  `nb` and `nav` as now. For another notebook it is built from its files (`store_load_info`,
  `nav_load`; three reads, on the tap); its dots open that notebook (section 5) and Rename is disabled.
- **What opens:** All, every time the Graph opens. A picked notebook lasts while the Graph is open
  (through the List and Pages tabs and back) and is forgotten when it closes.
  *Recommended over remembering it*: the owner asked for All as the default; a remembered narrow pick
  would make the Graph open as one notebook some days and as the library on others, with nothing on
  the way in to say which. One extra tap to narrow is cheap; a surprise is not. (If the owner wants it
  remembered, it is one `scope=<id>` line in `graph-all.ini`.)
- **View: All / Around** stays as it is, remembered per scope: `graph.ini`'s mode line for a
  notebook, `graph-all.ini`'s for All. "View: All" (the whole map) and "All notebooks" (the whole
  library) both mean "everything", so the words do not fight.

## 3. Node identity and a stable layout

The layout hashes `GraphNode.pid` for its jitter and breaks every tie by it; across notebooks page ids
collide (every notebook starts at 1). The change keeps the per-notebook path bit for bit:

- `GraphNode` gains `key: u32` (what the layout hashes, sorts and breaks ties by) and `book: u16` (an
  index into the model's book table, 0 in a one-notebook model). Every layout use of `.pid` (pid_hash
  callers, `pid_order`, `graph_gen`, `pack_warm`'s tie angle) reads `.key`. A one-notebook model sets
  `key = pid`, so its layouts, `graph.ini` and gens are unchanged (a test compares positions before and
  after on the corpus).
- **All's key:** `key = mix(fnv1a32(notebook id), pid)` (the murmur3 finalizer graph.rs has), never 0.
  With 8000 nodes the chance that a library has any two equal 32-bit keys is about 0.7%, so collisions
  are resolved, not ignored: nodes are visited in (notebook id, page id) order and a later node whose
  key is taken is mixed again with the next salt until it is free. Deterministic for a given library.
  A key only steers jitter and tie order; kept positions are stored by (notebook id, page id), so a key
  that changes because a colliding page appeared moves nothing that has a position.
- **Lookup table:** the u16 slot table stays (node index + 1 per slot), but hashes (book, page id)
  and compares both, and its size becomes the next power of two at least twice the node count (4096
  for a notebook as now; at most 16384 slots, 32 KiB, for All). `GRAPH_SLOTS` becomes the per-notebook
  size; `graph_node_of` takes the table's length from the model.
- **Caps.** All's model holds linked pages only (no Hidden entries: node i is not page i), so the
  pages of the library do not count, only its linked pages:
  `GRAPH_ALL_NODES_MAX = 8000` linked pages and `GRAPH_ALL_EDGES_MAX = 40000` links (backlinks'
  `BL_LINKS_MAX`). Past either, All shows a toast, "Too many linked pages for All notebooks: showing
  this notebook.", and opens the editor's notebook. STORE_MAX_PAGES (2000 per notebook) is unchanged.
  Memory at the caps: nodes about 0.6 MB, edges and neighbour lists about 1 MB, the layout's working
  arrays about 0.5 MB.
- **Stability across notebooks** (the kept layout is `graph-all.ini`, keyed by notebook id + page id):
  - a page moved within its notebook, a notebook renamed or moved to another folder: nothing moves
    (ids, not places or names);
  - a notebook deleted: its nodes vanish, every other keeps its position (no edge gets longer, so
    `graph_relax_stressed` moves nothing);
  - a few new linked pages: `graph_place_new` beside their neighbours, as today;
  - **a whole new component** (a new notebook's linked pages, or pages copied in with their links): a
    new step, `place_new_components`, lays each component with no placed member out on its own and
    puts it beside its notebook's island (or outside everything), so no kept node moves. The 10% rule
    (`GRAPH_RELAYOUT_PCT`) then counts only fresh nodes in components that already have placed
    members, so adding a notebook never relayouts the library.

## 4. Data loading on the Kindle

Reading every notebook's `notebook.ini`, `nav.ini` and `links.ini` at Graph open is ruled out: on the
host it is cheap (table below), but `/mnt/us` is FUSE on the Kindle and 500 notebooks are 1500 file
opens. Note that today's Graph already walks every `notebook.ini` on every open, synchronously
(`store_page_totals` in `mapview_open`, for the "across N notebooks" line); All replaces that walk with
the index below.

**The library index, `root/graph-index.ini`** (new module `libindex.rs`, the backlinks pattern; a
cache: deleting it loses nothing, never exported, no other version reads it):

```
# Grimoire: every notebook's pages, names and links, for the Graph (a cache: deleting it loses nothing)
nb=20261010-143205-a3f1,9f3c22d05b1e4471,1b2c...,77aa...
name=Lecture notes
folder=School/2026
pages=1,2,5,7,8
pname=5,Thermodynamics
link=5,p7
link=7,n:20261012-090000-b2c4:p3
link=8,index
```

- `nb=<id>,<stamp of notebook.ini>,<stamp of nav.ini>,<stamp of links.ini>` (backlinks' `file_stamp`
  hash; 0 for a file that is not there), then that notebook's name, folder, page order, page names and
  every link (targets as in `links.ini`, links from pages not in `pages` left out). A notebook whose
  `notebook.ini` does not read is `nb=<id>,0,0,0` alone, read again at every check.
- **Limits:** file read up to 4 MiB; at most 40000 links in all (a notebook that would pass it is kept
  as `nb=<id>,0,0,0`, "left out", and the panel says "N notebooks left out"). A file that does not read
  is ignored and built again.
- **Sizes** (text, by the formats above): 50 x 100 pages with 1000 names and 5500 links about 140 KB;
  500 notebooks of 1 page about 80 KB; 100 x 200 pages with 4000 names and 22000 links about 550 KB.
  In memory, compact (links as page id, book index, page id: 12 bytes), under 1 MB at the caps.
- **Kept current two ways**, exactly as backlinks.ini is:
  1. *On save, at once:* where this version writes the files, the notebook's entry is updated in memory
     with the new file's stamp: `nav_save_links` (next to `backlinks_note_saved`), `nav_save` (names),
     `store_save_info` (pages, name, folder; this covers lane A's Move to... and Copy to notebook...),
     and create, trash and restore of a notebook.
  2. *Lazily, on idle ticks:* `libindex_step()` reads the file on the first idle tick after start, then
     checks up to 8 notebooks' three stamps a tick and reads at most **one** notebook's changed files a
     tick (at most 256 KiB + 512 KiB + `notebook.ini`, pumped), so the main loop goes round every tick
     and the watchdog (15 s) never comes near. It runs where `backlinks_step` runs (editor idle) and
     also while the Graph is open and untouched, so a first-ever All fills in while you look at it.
  The editor's own notebook is never read from the index: at build time its entry is replaced by the
  live `nb` and `nav`, unsaved links included.
- **Graph open in All** (main thread): the index is in memory (loaded on the first idle tick; if the
  Graph is opened before that tick, it is loaded now, pumped: one file). Build the model from memory
  (`graph_build_all`, O(n + e)), apply `graph-all.ini` (kept in memory after its first read), then
  Ready / Incremental / Full as today. While the index is still being built (no file yet), the header
  says "Reading notebooks: 12 of 50" and new nodes join as their notebooks are read (placed by
  `graph_place_new` / `place_new_components`; the map's tick watches the index generation as
  `mapview_anchors_refresh` watches backlinks').
- **Big layouts** run in a bgjob child as now: a new `BgKind::GraphLayoutAll = 6`, path
  `root/graph-all.ini`. The parent flushes `graph-index.ini` first; the child reads it, builds the same
  model, runs `graph_layout_warm`, and writes `graph-all.ini` under its lock (`graph-all.ini.lock`).
  The parent adopts the positions as kept positions: if its model changed meanwhile, the difference is
  placed incrementally, never thrown away.

**`root/graph-all.ini`** (All's layout; a cache):

```
# Grimoire: the Graph of all notebooks (a cache: deleting it loses nothing)
gen=5f3a9c21
mode=all
pos.20261010-143205-a3f1.5=-310,442
```

Notebook ids have no `.`, so `pos.<notebook id>.<page id>` parses. Sorted by (id, page id); unknown
lines carried as `graph.ini` carries them; read up to 1 MiB (8000 positions are about 300 KB). The gen
is `graph_gen` over the keys.

**Budgets and measurements.** Designed for 100 notebooks, 20000 pages, 8000 linked pages, 40000 links.
Measured on the host (Apple M5 Pro, release build, warm file cache) with a scratch crate linking the
`grimoire` library at 26ef6c9 (synthetic models squeezed into one notebook, since one model is capped
at 2000 pages today):

| case | linked pages / edges / components | build | seed | full layout | place 1% new |
|---|---|---|---|---|---|
| 20 x 15, dense | 291 / 328 / 10 | 0.1 ms | 0.1 ms | 8 to 10 ms | 0.2 ms |
| 20 x 50, typical | 768 / 590 / 180 | 0.1 ms | 0.1 ms | 3 ms | 0.2 ms |
| 20 x 100, typical | 1504 / 1174 / 330 | 0.1 ms | 0.3 ms | 7 ms | 0.5 ms |
| 20 x 100, every page linked | 1991 / 2531 / 8 | 0.1 ms | 6 ms | 236 ms | 7 ms |

| reading every notebook's three files | pages / names / links | time |
|---|---|---|
| 50 x 100 | 5000 / 1000 / 5455 | 2.4 ms |
| 500 x 1 | 500 / 500 / 53 | 19.7 ms |
| 100 x 200 | 20000 / 4000 / 21807 | 6.0 ms |

Kindle estimates: graph.rs's comments take the Kindle as the host x 20; for a release build against a
release build x 20 to x 30 is safer (the comments' own host figures are about 7 times mine, which
suggests they were measured on a debug build). FUSE file costs are not measured here; the
implementer logs them on the device.

- 50 x 100 (about 3800 linked pages): with the index in memory, open in under 1 s (build and kept
  positions are milliseconds; drawing dominates, see section 6). A first full layout goes to the child:
  about 1 to 2 s for a typical library, 6 to 7 s for one where every page is linked, the seed shown at
  once.
- 500 x 1: the layout is trivial (about 100 nodes); the first-ever index is 500 notebooks at one a
  tick, roughly 10 to 25 s of idle ticks, once. After that a full check is about 63 ticks of stats.
- At the caps (8000 nodes) a full layout is about 1.1 s on the host (n log n from the 2000-node case),
  so about 30 s in the child on the Kindle. It runs only after a large change, because new notebooks
  are placed as blocks (section 3).

## 5. Taps

- **A node of the editor's notebook:** as today (select, card, Open, Around it, Rename).
- **A node of another notebook:** the card reads its page name (or "Page N") and a second line,
  "Lecture notes, page 12. Links to 3, linked from 2."; Open, Around it, and Rename disabled. Open
  returns `MapResult::Target("n:<id>:p<pid>")`, so `follow_target` and `open_external_page` open that
  notebook at that page and push a `NotebookBack`: Back returns to the page (zoom and offset) the Graph
  was opened from, as lane C's anchors do. Back does not reopen the Graph (anchors do not either; one
  gesture opens it again). A pen's double-tap rule (`PEN_REOPEN_MS`) is unchanged.
- **Broken links at All scale:** they draw nothing (no node, no edge). A node that has one gets the
  broken mark (a small dashed square at its lower right, lane C's dashed language), its card says
  "1 broken link", and the panel counts them library-wide ("Broken: N"). Repair stays where lane C put
  it: pick that notebook with the chip and tap its dashed anchor (one tap to the link's card). A
  broken link on a page with no other link is visible only there and in that notebook's List.

## 6. Rendering on grayscale e-ink

- **Islands.** Each component is assigned to its *home notebook* (the notebook most of its pages are
  in; ties to the lower notebook id). A full layout packs each notebook's components with the existing
  `pack` into one island disc, then packs the islands (the largest in the middle) with the same
  function. `pack_warm` keeps islands where they were on later layouts. No fake forces, no hub: links
  alone shape each island, and cross-notebook components sit in one of their notebooks' islands.
- **Captions.** Over each island, its notebook's name in the label face, drawn when the island is wider
  on screen than its caption and the caption overlaps no dot or label (the label code's own test).
  Node labels stay page names; in Around, a member from another notebook than the centre's is
  labelled "Thermodynamics (Lecture notes)", cut to the label width.
- **Dots.** Same patterns (degree levels), rings (open page, selected) and Index lozenge for every
  notebook, as Obsidian draws every note alike by default. The open page's ring is drawn at least 8 px
  across so "you are here" survives a whole-library fit. Dots are never drawn under a 3 px radius, and
  their knockouts scale with them, so a 3000-node fit reads as a shape and not as white speckle.
- **Selection** additionally draws every node of the selected node's notebook with a thin outer ring:
  which pages are its notebook, shown only when asked, without grey fills that fight the degree
  patterns.
- **Cost.** Label placement today tests each candidate against every node (candidates x nodes:
  2000 named x 8000 nodes is 16 million tests a redraw). In All it tests against a coarse screen grid
  (`grid_pairs` exists) and places at most 80 labels at fit, by the existing priority. `page_number`'s
  linear search is replaced by the number kept on the node. The panel's counts come from the index.
- **Around in All:** `graph_around` on All's model: the rings of the centre page cross notebooks
  (2 links deep, as now). No other change.

## 7. Lane C's anchors and the open BACKLOG item

- **In All**, other notebooks' pages are real nodes, so anchors are not built (`anchors_build` runs only
  in one-notebook mode) and the plot uses the full width.
- **In one notebook's view** anchors stay exactly as lane C built them, beside the plot.
- **"A page linked only from other notebooks has no dot in its own Graph"**: in All it has a dot (an
  incoming link from any notebook makes it linked). One notebook's view keeps lane C's rule, so its
  `graph.ini` gen never depends on other notebooks being saved. The BACKLOG item stays open for the
  one-notebook view, amended to say so.

## 8. Staged plan for one implementer

Each step builds and passes `cargo test` alone; each test is checked to fail without its change.

1. **Keys and books in graph.rs** (about 80 lines). `GraphNode.key`, `book`; layout reads `key`;
   slot table sized from the model. Tests: one-notebook layouts, gens and `graph.ini` bytes identical
   before and after over the corpus; lookup at 4096 and 16384 slots.
2. **libindex.rs and graph-index.ini** (about 450 lines with tests). Parse and write, refusal of bad
   files, `libindex_step`, the `note_*` hooks, live overlay of the editor's notebook. Tests: build from
   nothing, one notebook changed (one read), a notebook deleted and restored, the 40000-link cap and a
   left-out notebook, a corrupt file rebuilt, stamps after a save need no re-read, step work bounded
   (one notebook read a tick).
3. **graph_build_all and graph-all.ini** (about 300 lines). Keys with the collision rule (a test hook
   forces two equal keys), cross-notebook edges, Index badges, broken targets ignored, caps refused,
   positions by (notebook id, page id). Tests: identity (same page id in two notebooks = two nodes),
   determinism, stability (pages moved, notebook renamed, notebook deleted: no kept node moves).
4. **Islands and new components** (about 150 lines). Home notebook, two-level pack,
   `place_new_components`, the 10% rule over attached nodes only. Tests: islands never overlap; adding
   a notebook with 50 linked pages moves no kept node; a full layout is deterministic.
5. **The child** (about 80 lines). `BgKind::GraphLayoutAll`; adoption by kept positions. Tests:
   completion belongs to All, not to a notebook with the same id string; a model changed during the
   job is placed incrementally.
6. **mapview** (about 400 lines). Scope state (All by default), chip and menu, the picker's
   notebook-only mode, node page and notebook helpers, the card, Target taps, captions, minimum dot
   size, label grid and cap, panel counts from the index (drop `store_page_totals` from the open),
   "Reading notebooks", the toast at the caps. Strings through `tr()` with TRANSLATORS comments and
   Spanish (below); `y_all_screens` captures for en and es.
7. **Scenarios** (`tests/map_fixture.py` grows a `--library BOOKSxPAGES` mode): open the Graph = All
   (`editor.map.scope is all`, nodes include another notebook's); pick This notebook (scope `one`,
   anchors there); Other notebook... picks a third; tap a node of another notebook (the editor is in
   that notebook at that page); Back (first notebook, same page and zoom); Around across notebooks; a
   50 x 100 library opens with `main.longest_unpumped_ms is ok` and logs its open time; a first-ever
   index fills in ("Reading notebooks").
8. **Docs.** FORMAT.md (the two caches, in Files and as sections, and "Moving between versions": never
   read by other versions), the CHANGELOG beta.2 line, BACKLOG (the item in section 7 amended; the
   deferred list below), PORTING.md (Rust only).

**New strings** (Spanish proposals, to check against the widths): "All notebooks" / "Todos los
cuadernos"; "Other notebook..." / "Otro cuaderno..."; "Reading notebooks: %d of %d" / "Leyendo
cuadernos: %d de %d"; "%s, page %d. Links to %d, linked from %d." / "%s, página %d. Enlaza a %d,
enlazada desde %d."; "Too many linked pages for All notebooks: showing this notebook." /
"Demasiadas páginas enlazadas para Todos los cuadernos: se muestra este cuaderno."; "%d broken
link(s)" with plural forms; "%d notebooks left out" / "%d cuadernos fuera".

**Deferred** (proposed by this design, 2026-11-02; the owner decides):

- ~~Orphans in All (section 1).~~ Decided by the owner (2026-11-03): never drawn.
- Remembering the Notebook pick across opens (section 2): not built, by the recommendation.
- Renaming another notebook's page from the Graph (writing its `nav.ini`): Rename is disabled there.
- Back reopening the Graph after following a node: built for the Graph opened from the Library (section 10);
  the editor's own Graph still does not, nor do anchors.
- Filters beyond Notebook (a folder, a search, "find a page in the graph", already in BACKLOG).
- Folding `backlinks.ini` into `graph-index.ini` (one scan instead of two): after beta.2, once both
  have shipped; lane C's cache is left as it is.
- Building the first-ever index in a bgjob child instead of idle ticks, if the device shows the idle
  fill too slow for big libraries.
- Removing the dead satellite and tray code (already in BACKLOG).

## 9. As built (graph lane, 2026-11-02)

Built as staged above, on claude/b2-graph. Where the build departs from the design, and why:

- **Memory of the index.** A link is kept as `GraphLink { src, to: GraphTo }`, the other notebook's id held
  as its 24-byte id rather than a book index (about 32 bytes a link, about 1.3 MB at 40000 links instead of
  under 1 MB). Book indexes shift as notebooks come and go; the id keeps every entry valid on its own.
- **The child reads the index from memory.** `BgKind::GraphLayoutAll` builds its model from the index as the
  parent had it when it forked (the parent still flushes `graph-index.ini` first, and a child that never had
  it loaded reads the file). Same model, one file read fewer.
- **Stretched edges.** After placing new pages, one notebook's Graph runs `graph_relax_stressed` whenever the
  gen differs. In All a new notebook changes the gen too, and a library's layout can hold edges longer than
  the stress rule already, so that pass could ask for a relayout of the whole library when a notebook was
  only added. All runs it only when the kept part changed: `graph_gen_of(placed)` (the gen over the nodes
  that kept their places and the edges between them) differs from the file's. New pages and notebooks alone
  never stretch a kept edge, so they move no kept dot, as section 3 asks.
- **Where a new component goes.** "Beside its notebook's island" is the nearest clear spot over 16
  directions around the island (the first turned by the component's hub key), each walked outward until clear
  of every placed component.
- **Links to notebooks not read yet.** While the index's first check is still going, a link to a notebook it
  has not read is left out (as one to a notebook it cannot read is), not counted broken; once a check has gone
  through every notebook, a link to one that is gone is broken.
- **"Reading notebooks".** With no file to start from, the index lists the notebooks as it loads (one
  directory read), so the chip can say "Reading notebooks: 0 of 50" from the first tick. The Graph builds
  again from the index at most every 2 s while it fills, so the screen is not refreshed for every notebook
  read.
- **One cache a tick.** On idle ticks the library index's step runs once backlinks.ini's check is through
  (BACKLOG: to watch on a big library on the device).
- **Panel counts.** The library's pages come from the index in one notebook's view too (its library totals
  are filled in when the index's first check ends; scenarios that check them wait for it). "%d notebooks left
  out" has plural forms ("%d notebook left out"). The card's Spanish follows the existing card's wording:
  "%s, página %d. Enlaces a %d, desde %d."
- **The List tab** always lists the editor's notebook's links, whatever the Graph shows (its rows are followed
  from the editor's notebook).
- **One notebook's view of another notebook** keeps that notebook's anchors; a tap on a broken anchor there
  does nothing (its card belongs to that notebook's page; repair it from that notebook).
- **The open log.** "graph: All notebooks open, N notebooks, N nodes, N edges: built N ms, drawn N ms" and
  "libindex: N of M notebooks read again, N ms" give the device's numbers (section 4 left FUSE costs to the
  implementer).

**Measured** (Apple M5 Pro, release build, `cargo test --release --test test_graph_all_bench -- --nocapture`;
synthetic libraries from `tests/graph_corpus/library.rs`, about 1.1 links a page, one in twenty to another
notebook; "reopen" is the model built and graph-all.ini's positions applied, what an open costs before it
draws):

| case | linked pages / edges / components | build | seed | full layout | warm layout | place 1% new | reopen | graph-index.ini |
|---|---|---|---|---|---|---|---|---|
| 20 x 15, dense | 282 / 351 / 12 | 0.1 ms | 0.1 ms | 9 ms | 2.6 ms | 0.1 ms | 0.1 ms | 8 KB |
| 20 x 50 | 898 / 1010 / 36 | 0.1 ms | 0.2 ms | 83 ms | 13 ms | 0.1 ms | 0.2 ms | 20 KB |
| 20 x 100 | 1766 / 2066 / 43 | 0.2 ms | 7.7 ms | 204 ms | 77 ms | 0.2 ms | 0.3 ms | 39 KB |
| 50 x 100 | 4435 / 5189 / 134 | 0.5 ms | 24 ms | 597 ms | 229 ms | 18 ms | 1.0 ms | 99 KB |
| 20 x 100, every page linked | 1988 / 4638 / 2 | 0.3 ms | 7.6 ms | 260 ms | 95 ms | 0.2 ms | 0.5 ms | 74 KB |
| 500 x 1 | 500 / 286 / 214 | 0.1 ms | 0.1 ms | 1.3 ms | 2.0 ms | 0.1 ms | 0.1 ms | 75 KB |
| 100 x 200, sparse (0.22 a page) | 7025 / 4200 / 2826 | 0.6 ms | 1.4 ms | 13 ms | 242 ms | 9.3 ms | 1.4 ms | 181 KB |
| 100 x 200, 1.1 a page | past the caps (more than 8000 linked pages): opens on this notebook | | | | | | | |

graph-index.ini read back (parsed) in 0.5 ms or less in every case. At x20 to x30 for the Kindle, the
50 x 100 library's full layout (about 12 to 18 s) runs in the child with the seed shown; an open with kept
positions stays in milliseconds before the draw.

## 10. The Graph as a place of its own, and leaving notebooks out (graph lane, 2026-11-03)

**The owner's words (2026-11-03):** "For the new graph view -- that's fine, but the view for unlinked pages is
beyond unnecessary, only linked pages should be shown, and from all notebooks -- meaning we can simply add an
option in the options for managing folders and notebooks to omit a notebook or all contents of a folder from
the graph view entirely. Next is a further change - the graph view is now its own entity and way of managing
pages (interlinked ones) It needs to be accessible from the main menu through the swipe down with the pages
view taken out if summoned from the main menu (won't have pages to view)."

**Linked pages only.** The orphan build (section 1) is gone; see section 1's decision.

**Leave out of the Graph.** A notebook's menu (the card's ... button: Open, Rename, Move to folder, Save PDF,
then this row, Delete) and a folder's (Rename folder, Move folder, this row, Remove folder) have **Leave out
of the Graph**, or **Show in the Graph** when it is left out. A notebook keeps it in its `notebook.ini`
(`graph=0`, written without touching `modified`, so the Library's order stays); a folder in
`root/graph-omit.ini` (FORMAT.md), since folders have no metadata file of their own and older versions read
every line of `folders.ini` as a folder. A folder covers everything inside it at any depth, notebooks moved
in later included (`graph_omits`: the notebook's own setting, or `store_folder_contains` of any listed folder);
inside a left-out folder a notebook's (or a folder's) row is greyed out and says "Left out of the Graph with
its folder". Renaming or moving a folder carries its line; removing it drops it.

- **Edges.** A left-out notebook goes into `graph_build_all`'s skip list, as a notebook the index cannot read:
  links to it are neither edges nor broken, and links from it do not exist. A page linked only with a left-out
  notebook is not linked in All (not drawn); one with other links stays, without that edge. Chosen over an
  anchor or a dashed edge to nowhere because the owner asked to omit it "from the graph view entirely": a
  line that leads nowhere would point at it.
- **The caches.** `graph-index.ini` keeps the notebook's own setting (`omit=1`, updated in memory when
  `notebook.ini` is written, like its name); the folder list is read when first needed and again after it
  is written (`libindex_note_omit` also moves the index's generation on, so an open Graph builds again).
  The panel's library totals count only the notebooks the Graph shows (so its orphan count stays the
  Graph's pages less its linked ones). `graph-all.ini` keeps a left-out notebook's positions as it keeps any
  notebook the model lacks, under GRAPH_ALL_NODES_MAX: shown again, it comes back where it was.
- **Its own Graph.** Opened from a left-out notebook, All leaves it out like any other. (The chip's This
  notebook showed it, and Other notebook... could pick it, until the chip was removed: section 11.)

**From the Library.** Two fingers down over the Library (whatever `gestures.ini` binds to `index-map`; the
Library honoured no gesture before), or the `index-map` key open the Graph of all notebooks with no notebook
open (a top bar **Graph** button, left of New, was built too and taken out the same day, the owner: "For now
just the drag to summon on the main menu."). The Library hosts its own MapView (`library.rs`), opened with an empty NotebookInfo and Nav;
`mapview_no_notebook` makes it:

- **No List or Pages tab** (`graph_navigation`: Back and the Graph tab). The List lists the open notebook's
  links, so it goes too. (It had no This notebook row in the chip's menu either; the chip is gone: section 11.) A picked notebook that does not read opens every notebook
  (never a view of the empty notebook); past the caps a toast says so and the Library stays.
- **Every node is another notebook's.** Open (a second tap, or the card's Open) closes the Graph, keeping
  its pick, view and selected page (`MapKeep`), and opens that notebook at that page
  (`editor_open_from_graph`). Back in that notebook with nothing before it (the `go-back` action, the Links
  bar's "Back to Graph" chip), and its Library button, save the page, close the notebook and return to the
  Library, which opens the Graph again as it was (`library_return_to_graph`, `mapview_restore`); links
  followed into other notebooks first come back as usual. Back on the Graph (or two fingers down again, or
  `go-back`/`close-menu`/`index-map`) returns to the Library.
- **Nothing assumes a notebook.** No EditorState is touched while it shows. Rename is greyed out (no node is
  the editor's), a broken anchor or List row repairs nothing, Open never yields a page of "this" notebook,
  the panel says "Pages: N across M notebooks", and the Library ticks keep the backlinks and library index
  caches as the editor's idle ticks do (no notebook may have been opened since Grimoire started). The layout
  children's ends are routed to the Library's Graph first. Unit tests: mapview
  `with_no_notebook_open_the_graph_is_every_notebooks_and_names_none`, libindex
  `a_notebook_or_folder_left_out_of_the_graph_is_no_node_and_no_edge`, test_graph_all
  `left_out_notebooks_and_folders`; scenarios `z_graph_lib_1_open` to `z_graph_lib_4_es`.

## 11. The Notebook chip removed (owner's ruling, 2026-11-03)

Testing on the Kindle, the owner wrote "you left on the all notebooks button btw - just getting to testing.
fold it in if you can", then, asked whether that meant moving its choices into View or removing the notebook
choice, ruled: **"remove it."** So:

- **The Graph always shows every notebook** (less those left out with the Library's "Leave out of the Graph").
  The chip, its menu, This notebook and Other notebook... are gone, with their strings ("All notebooks",
  "Other notebook...") and their Spanish. `MapResult::ScopeAll`, `ScopeThis` and `PickNotebook`,
  `mapview_set_scope` and the editor's remembered pick (`map_pick`) went with them; the List and Pages tabs
  come back to every notebook.
- **View stays as it was**: View: All / Around, and Fit after a pan or pinch.
- **"Reading notebooks: N of M"**, which sat beside the chip, is now on the panel's counts line, after
  "Nodes · Orphans · Links" (and broken links), while the index's first check runs.
- **One notebook's view is kept internally, for the caps only.** Past GRAPH_ALL_NODES_MAX linked pages
  (links are already capped by the index leaving notebooks out), the editor's Graph shows its own notebook
  with the toast "Too many linked pages for All notebooks: showing this notebook.", and the Library's Graph
  says "Too many linked pages to draw all notebooks." and opens the notebooks' browser to show one, as
  before. Chosen over drawing "the first notebooks up to the cap": that needs a new partial build of the
  library (which notebooks, in what order, and saying which were left out), where the fallback exists and is
  tested. Lane C's anchors are drawn only in that view now.
- **The plot keeps its top at 290 px** (the band the chip sat in stays empty), so nothing else on the screen
  moves.
- **Tests.** mapview `another_notebooks_node_opens_there_and_there_is_no_notebook_choice` and
  `with_no_notebook_open_the_graph_is_every_notebooks_and_names_none` check, from the editor and from the
  Library, that nothing answers where the chip was (`assert_no_notebook_chip`: no control, a tap there changes
  no scope, mode or tab, no `menu` probe, no `scope.*` control name). Scenarios that picked This notebook to
  check one notebook's Graph now check every notebook's (`z_map_host`, `z_map_0_make`, `z_map_tray`,
  `z_index_1`, `z_index_unified`, `z_notebook_links`, `z_page_management`); `z_links_1_backlinks` checks the
  pages that link across notebooks as nodes and opens one, `z_links_2_broken` the broken count and a broken
  link's List row; `z_map_all_1`, `z_graph_lib_1_open`, `z_graph_lib_3_omit`, `z_graph_lib_4_es` and
  `y_all_screens` check that nothing answers where the chip was (`map.ui_at.400.215` empty).
