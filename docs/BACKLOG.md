# Backlog

Work that was decided, deferred or found but not done yet. Keep this current: when an item ships,
move it to "Done" with the version; when something is deferred, add it here with who decided and
why. Newest decisions first within each section.

## Found on the owner's stress pages (2026-11-05): candidates for 2.0.0, the owner decides

From STRESS TEST 2 (648 strokes, 153,588 points, 2.8 MB) and STRESS TEST 3 (698 strokes, 163,950 points,
3.0 MB) in the owner's "2.0b2 DEMO", written on the Scribe on 2026-11-04 and 05 with 2.0.0-beta.2 builds.
The record and the evidence are in the maintainer's lab folder (stress-tests-2026-11-05/RECORD.md).
Writing stayed fast at any size (median pen to screen 0.6 to 0.9 ms); the costs are elsewhere:

- **A page written from blank never gets a kept drawing while it is written.** render_page_full keeps one
  only after an open that took 1 s or more, the idle autosave only for a page whose open was slow, and the
  previews job only runs with Pages shown. So the first revisit draws the page whole: 11.6 s for STRESS TEST 2,
  28.4 s for STRESS TEST 3 (beside an export), 3.6 s for a 1 MB page. Proposed: at the idle autosave or on
  leaving the page, keep the drawing of any page of 512 KB or more whose canvas is whole.
- **The PDF mirror export redraws every page from strokes at each close after a change**: 51.4 s and 48.6 s
  for this 5-page notebook, the second for five strokes on page 1. It could reuse current kept drawings, or
  redraw only the pages that changed.
- **Closing from a page other than page 1 draws page 1 again from its strokes** for the Library card: 2.9 s
  from STRESS TEST 3, 208 ms from page 1 itself.
- **Write amplification at the idle autosave**: 40 small strokes (about 31 KB of ink) wrote 6.5 MB (the page,
  its preview and its kept drawing), and a kept drawing was written twice in 19 s.
- **Zoom steps on the stress pages took 0.56 to 0.66 s each**, with frame copies of 0.57 to 0.61 s; a zoom on
  a lighter page minutes later had none. Not explained yet.
- **A stroke-eraser move on STRESS TEST 2 took 2.1 s** at 2.38 MB, its screen update 2 s late (as the CHANGELOG's
  about 3 s for a page-wide marker stroke on the first stress page).
- **"Loading page..." and undo are not logged**, so a record cannot say whether the notice showed or undo ran.
- **Limits, by extrapolation only**: nothing caps strokes or points on a page; the nearest limit is the 120 s
  heartbeat for a page drawn whole, reached at roughly 13 to 15 MB of STRESS TEST 3's mix (about 35 minutes
  of pen contact on one page). Points are kept at the pen's full rate, about 6.3 KB a second of contact.
- **As fixtures**: real device data with strokes of up to 7,028 points (the synthetic dense fixture has 10 to
  129), but 2.8 to 3.0 MB each and the owner's own drawings, so adding one is the owner's call.

## The Terminess diet (2.0.0-beta.2): choices for the owner

The owner chose Terminess with Terminus's text and the Material Design icons only, with the Apache-2.0
notice (assets/README.md). Left open by the builder and the reviews, 2026-11-04:

- **Embedded bitmaps kept.** Dropping them saves about 305 KB a weight (0.6 MB of the app), but FreeType
  (KOReader) would then draw outlines instead of Terminus's own pixel bitmaps at 12 to 32 px.
- **Nerd Fonts' glyphs at ordinary code points are gone** (box drawing and blocks U+2500-U+259F, the
  heart, the lightning, the menu sign, the power symbols, the heavy angle brackets): a name shows
  without them, KOReader draws them from its fallback fonts. Terminus 4.49.3's own box drawing, blocks
  and heart (OFL) could be merged back from upstream, in Terminus's shapes rather than those beta.1 drew.
- **No visible link to the fonts' licenses on the share pages.** `/font/licenses.txt` is served, each
  font's answer names it (`Link: rel="license"`) and the pages' style says where it is; a footer link
  would need a translated label.
- **The published 2.0.0-beta.1 still offers the full fonts** with only the OFL text (no CC BY
  attribution for Font Awesome and Codicons, and the unlicensed Font Logos), until it is withdrawn, its
  notes amended, or beta.2 replaces it.

## The keyboard scheme, the Pages view and the Graph's keys (2.0.0-beta.2): choices for the owner to confirm

Built from the owner's words of 2026-11-03 for a keyboard paired with the Kindle. Left out (owner to confirm):
a keyboard focus in the Library (Left/Right turn the grid's screens instead); a Delete key in the Pages view;
the Kindle noticing a keyboard while shortcuts are off.

The keyboard scheme:

- Owner: "Escape for CMD S for saving anything not saved anywhere": read as Ctrl+S (Cmd+S on an Apple
  keyboard) saves now (everywhere; its "Saved" toast went in the owner's toast pass of 2026-11-04), and Esc
  at the Library's top asks "Exit Grimoire?" (Yes and No). The Exit button still leaves at once.
- Owner: "Left and right do what you'd expect but up and down do nothing here or anywhere else for now":
  Left and Right turn the Library's screens (no focus ring); Up and Down are unbound but in the Graph, and the
  count pad no longer steps with them (its - and + do).
- Esc inside a Library folder goes up to the top first, then asks. Esc on a page with nothing in front goes
  back to the Library, as the toolbar's Library button (saving first).
- R opens the page's naming keyboard as the action does today, back to the page after; O acts in the Library
  only. PageUp and PageDown turn pages on a Kindle keyboard too.
- Not done here: Enter and the Left/Right highlight in the Delete question (Esc answers Keep it; only Exit
  Grimoire? has a highlight).

The Pages view (the owner: "CMD N prompts for a page number with numpad immediately and if enter is pressed or
confirm via touch, 1 will be the default so it will create a new page." "CMD + F says page number or name, but
it needs its own dialog copy in the pages view that doesn't just drag you out to the notebook while you
input"):

- **Ctrl+A also turns Multi-select on**, as the Select all button does (a later plain tap then toggles one
  tile instead of picking it alone); Select none or Multi-select gives the plain tap back.
- **The count pad opens at 1**, the toolbar's + held included.
- **Esc during a drag cancels it** for `close-menu` too, since a drag otherwise swallowed the key: the pages
  stay where they were.
- **Esc on a tile's card closes the card alone for `close-menu` too** (a file that keeps
  `key.Escape=close-menu` keeps that); clearing the ticks stays `back`'s alone.

The Graph's keys (the owner: "Add left right up down positioning ONLY FOR GRAPH VIEW - with - + zoom ... V will
change the view toggle, and I need it to be View: All and View: Adjacent or similar so the wording isn't
jumbled"):

- **The view button after a pan always switches**: two taps (the other view and back, each fitted), or the
  fit-page key.
- **Esc in the Graph**: the selection, then the Graph. A file that keeps `key.Escape=close-menu` closes the
  Graph.
- **Old shortcuts.ini files** that bind none of the Graph's actions get its arrows and V in memory (as the
  D-pad's lines), so an edited file from an earlier build (an unedited starter map is upgraded) still pans in
  the Graph; deleting every Graph line on purpose brings them back the same way. The zoom keys are the file's
  own (the new starter map has - and +).
- **Not built**: the selection is not kept when the Graph closes.

## Next: 2.1 (deferred by the owner, 2026-10-31)

- **Moving a folder onto a folder of the same name: a merge dialog** (owner, 2026-11-04, in the review of every
  toast; a feature of its own, not part of that pass). Today the move is refused with "Couldn't move folder. Is
  one with that name already there?"; the owner: it "should open a MERGE dialog, not an error".

- **PDF planners with their hyperlinks, imported without KOReader** (owner, 2026-11-03; next after the
  2.0.0-beta.2 release candidate). A user asked to use a PDF planner whose pages link to each other. Two parts:
  1. Links: each PDF link (its box and the page it jumps to) becomes a Grimoire link area pointing at the
     notebook page that holds the target page (source.ini's map); a planner is imported whole so every target
     exists; web links are left out at first.
  2. Owner ruling: imports are no longer gated by KOReader. Grimoire itself finds PDFs placed in a folder it
     watches (as it already does for page images in grimoire/import/<Name>/) and imports them, links included.
     That needs a PDF renderer in Grimoire on the Kindle: choose by licence and size (PDFium, BSD-3, keeps every
     licensing option open; MuPDF is AGPL-3.0; poppler GPL), cross-built for the Kindle, with page rendering in
     the background job so the editor never blocks. KOReader's "Open this book" path stays as a second way in.

- **Cross-platform tablet bridge** (owner, 2026-10-31; started, not finished: the weekly limit). One small
  dependency-free Rust tool for Windows (InjectSyntheticPointerInput PT_PEN), macOS (CGEvent tablet subtype)
  and Linux (uinput pen tablet), reading Grimoire's own Live view pen stream (/live/pen, pairing code; no SSH),
  plus a device-side Tablet mode so the pen draws no ink while it drives the computer. tools/scribe-tablet.c
  (Mac, over KOReader's SSH, round 1) is the stopgap; plan in the rc.12 session notes.
- **The KOReader library entry for Grimoire** as an option in the KOReader sheet (owner, 2026-10-31).
- **Stroke eraser sweeps on dense pages**: batch a sweep's redraws (needs an input-peek API) and cache brush
  checkpoints (the eraser lane's notes, rc.12).
- **Custom Paper builder.** A picker tile that builds custom versions of ruled, dot grid, graph
  paper, cross grid, isometric, hexagon and Seyes paper (spacing, line weight, margins). Needs a
  new paper namespace in the page format (docs/FORMAT.md); older versions show such pages as
  blank "Newer paper".
- **Smart-shape snapping to geometry.** A snapped shape's corners, ends and edges stick to other
  shapes, lines and points already on the page.
- **Open Amazon's own Scribe notebooks in Grimoire.** Native notebooks are folders in
  `/mnt/us/.notebooks/` holding an unencrypted SQLite file `nbk` (tables `fragments`,
  `fragment_properties`, `local_delta_fragments`, `local_action_fragments`, `capabilities`; the
  stroke data likely Amazon Ion). Verify against real notebooks on the device before trusting any
  format notes; the etienneunai fork's converter assumes a 1024-byte header these files do not
  have. Credit the fork if any of its findings hold.

## Exact-area lasso and the live preview (beta.2 lane B3): deferred

Decided by the lane B3 design (docs/beta2/PIXEL-LASSO.md, 2026-11-03) and its builder, owner to confirm:

- **Trimming carried erase copies** to the reach of an exact-area group's ink (design 1.4): fewer scoped
  floors and a smaller clipboard. Copies travel whole today, which is exact inside the group.
- **"Save selection as picture"** (design 3, owner question 3): a raster export of a selection. A raster
  selection as a page object was rejected (no tool id or chunk older versions keep).
- **Remembering the lasso mode in settings.ini** (owner question 1): it lasts until Grimoire restarts.
- **A `select-exact` row in Settings > Shortcuts > Buttons**: its Editing list is full (11 rows); bind it in
  shortcuts.ini or gestures.ini meanwhile (builder, 2026-11-03).
- **Cutting a calligraphy stroke whose tilt widens or lightens it**: its nib's smoothing restarts at a cut
  and only where the tilt changes nothing but the nib can a piece start it where it was, so such a stroke
  is taken whole (builder, 2026-11-03: measured up to 249 levels off near the cut otherwise).
- **Jitter after a cut**: pencil, mechanical pencil and charcoal draw a cut part's per-dab opacity jitter
  afresh (the dab count restarts; no format field holds it): measured up to 54 levels in single pixels,
  0.39 of a level on average over the stroke's box. A per-piece jitter offset would need a format field.
- **Shift flips one lasso's mode, as Alt does** (builder, 2026-11-03, after review; owner to confirm the
  key), on a keyboard paired with the Kindle.
- **TRANSLATORS comments xgettext drops** (builder, 2026-11-03, found in review): xgettext keeps a comment
  only on the line right above the one holding `n_(`/`tr(`. The two lasso actions' comments are fixed (and
  `test_actions` checks them in po/grimoire.pot); about 50 older ones are not, among them the other 11 in
  actions.rs (the Buttons sheet's groups and rows, above `ai(`, `nm(` or `ActionGroup {`) and many in
  library.rs above `csnprintf!(` or `menu_row(`. Not this lane's strings; a sweep for its own change.
- **The exact-area preview's ticks** sit halfway between judged samples (1 mm apart), up to 0.5 mm from the
  final cut at a dab; a stroke grazed by less than 1 mm may be cut without a previewed tick (design 2.3).

## Lasso (beta.2 lane B): deferred (decided by the lane B design, 2026-11-02, owner to confirm)

- **Floor-aware compaction and dead-layer reclaim.** Pages with floors (ext[1] != 0) are left as they are
  by every version's compaction, this one included; `page_droppable` would need to keep markers and pass
  floors through the pieces path (wired in render_from, not yet reachable). Layers are not reclaimed, so a
  page holds at most 255 scoped groups before a paste there is refused.
- **Per-group tooth origin.** Pencil, mechanical pencil, charcoal and white-pencil grain belongs to the
  paper (absolute tooth, absolute charcoal dust): a moved pencil word shows the tooth of its new spot.
  Pixel-identical grain after a move would need per-group data.
- **"Remove erasure" on a selection, and white-out as a movable object** (owner question 1): today a lasso
  around white ink alone selects nothing; uncovering stays Undo.
- **A faster old-place redraw on dense pages** (the stroke eraser's slow path; measured on a generated
  1,000-stroke page in the headless harness: move 12 ms old place + 22 ms new place on the Mac).
- **An index target across notebooks** (with lane C): a pasted link to the Index is left out with a toast.
- **A dragged-ink preview** instead of the bare outline while dragging.
- **Preview stamps of layered pages** (design 3.4): Pages previews are keyed on the page file's stamp, so a
  preview an older version made of a layered page shows its flat look until the page changes. Kept
  drawings (`.full`) already carry the `|L` tag.
- **The C tree** (src/page.c) has no floor branch: the Rust tree is the shipping one; the C tree draws a
  scoped group flat, as 1.1 to 2.0.0-beta.1 do.
- **Enclosure tuning on real handwriting**: half the visible spine, 1.5 mm short-stroke rule; tested on
  synthetic strokes only.

## In progress

- **2.0.0-beta.2** (owner, 2026-11-02; briefs docs/beta2/BRIEFS.md): all four lanes merged into
  claude/beta2, each reviewed and gated on its own; not on a Kindle yet. Move to... and Copy to
  notebook... (lane A); lasso Select with Copy, Cut, Delete, Paste and drag, scoped erase strokes and
  root/clipboard.qpg, linked items copied with their links (lane B, docs/beta2/LASSO-DESIGN.md); backlinks,
  Graph anchors, broken targets, Back across notebooks (lane C); the macOS CI flakes and the Bluetooth
  helper stop (lane E); Live view PiP in Firefox 159+. Study: docs/beta2/JAPANESE.md.
- **The panel's page count does not fit four-digit pages** (b3-padd2 agent, 2026-11-03, found by the fit check):
  "1995 / 2000" needs 162 px and the toolbar's page chip has 112, so it is cut short in a notebook of 1000 pages
  or more (z_pages_add_6_toolbar_limit stays on page 1 for that reason). A smaller size for long counts, or the
  current page alone, would fit.
- **Pages: the count pad's - and + do not repeat when held** (b3-padd agent, 2026-11-03; the brief made it
  optional): one step per tap, and Up and Down on a keyboard repeat only as the keyboard sends repeats
  (Grimoire ignores key repeats).
- **Pages: drag shows the gap bar only, never makes room** (b2-pv agent, 2026-11-02): the tiles do not
  shift aside while a page is dragged (each shift would redraw the grid on e-ink); the bar between two
  tiles says where it lands. Revisit after trying it on a Scribe.
- **Copy to notebook...: a picture still waiting to be fitted** (lane A agent, 2026-11-02): a page whose
  `bg/<pic>.source.png` has not been fitted yet (a KOReader page of another size, seconds after it came)
  is copied without its picture and draws white there. Copy the source too and queue its fitting in the
  destination if it ever matters.
- **Live view smoothness.** rc.10 replaced the playout (smooth through stalls); still to measure on
  the device with the owner. Ideas left: a 'Wi-Fi is stalling' hint when lateness stays high,
  decode/compose/average in a Worker, 4-bit tiles on a congested link. The intermittent lag measured so far was the Kindle's 2.4 GHz link
  stalling (5 GHz was clean). When it lags: check the band (`iw dev wlan0 link`) and a
  `ping -i 0.1` from the Kindle while it happens; the page's Latency readout shows playout pinned
  at 20 ms during bursts.

## Built, not installed

- **USB file access hardening** (outside this repo, `~/KindleLab/volume`, branch `rc8`, worktree
  `~/KindleLab/volume-rc8`): a Kindle-side controller that survives crashes and logs why it stops,
  never holds the power service awake, the Mac mount behaving like a drive (no `._` files, unmounts
  when the Kindle leaves), and a read-only guard on Grimoire's live files while it runs. Install
  with the owner present: `install_mac.sh`, then `install_usb_daemon.sh` on the Kindle.

## To investigate

- **Two checks fail on a Linux host** (2026-11-03, on a Void Linux x86_64 machine; both the same at
  5d3e6fe; they matter before `GRIMOIRE_CI_LINUX` is turned on): `tests/scenarios/y_status_corner.qs` line 82 sees a second present at 632,2250 595 x 110
  (a toast's place) after the corner is tapped back on; `rust/tests/test_templates.rs`'s
  `card_drawn_again_with_its_picture` finds no template job started (`template_install` finds the picture
  file already there, so the card is never drawn again). On the Mac both pass. The Kindle is Linux too:
  whether either is a Kindle bug or a host difference is not known yet.

- **CI on Linux runners** (the CI fan-out, 2026-11-02): build-host, unit, the scenario shards and l10n
  can run on ubuntu-24.04 (repository variable `GRIMOIRE_CI_LINUX=true`), where jobs start sooner and
  far more run at once than on macOS (5 at most on a free plan). Off until a run passes there: the
  headless runner and the test binaries were cross-compiled for Linux (armv7 musl) on the Mac, never run.
  host-checks stays on macOS until share_check.sh's studio picture check has ImageMagick 7 (`magick`)
  there; package stays for the clean-build comparison with the maintainer's Mac.
- **y_all_screens.qs is the longest scenario** (77 s here, about 112 s on the CI runner): it is a shard's
  critical path. Split it by screen group (as z_shape_writing was) if shards get shorter than it.
- **The package job installs cargo-zigbuild from source each run** (about two minutes): a cached build or a
  release binary checked by its sha256 would save that; actions/cache was not added (a new action to pin).
- **make check's host checks still run one after another** after the scenarios (share server, Live view,
  Bluetooth, the late-timer scenarios): they share one port; giving each its own would let them run beside
  l10n-check.

- **Issue #11's refreshes on the Scribe** (claude/b2-refresh, 2026-11-02, no device access then). Check on
  the device: (1) a toolbar drag's release flash (a Clean: GC16 with flash over the trail alone, a mode the
  display had not sent before) flashes only that strip and leaves no copy of the toolbar; (2) a word erased
  with the stroke eraser leaves no trace a page turn would wipe, or, if it does, how many words before it
  shows (the threshold is a quarter of the screen, `DU_WEAR_QUARTERS` in display.rs); (3) the flash after a
  quarter's erasing waits for the pen to leave and never meets the next stroke. The diagnosis assumes the
  driver's GL16 partial update drives only pixels whose value changed (as on i.MX EPDC); if the Scribe's
  hwtcon drives every pixel of a GL16 region, the erase traces are something else. The log says
  `display: cleaning fast refreshes' traces` at each clean.
- **Pinch and pen-button zoom leave DU traces too** (claude/b2-refresh, 2026-11-02, not counted): their
  frames redraw the whole screen by DU and end with a grayscale refresh ("settle without a flash", an
  earlier ruling), so they wear the screen as erasing does. Left out of the wear count so zooming never
  flashes; owner to decide whether a zoom should add to it.

- **The Bluetooth helper's command line on the Kindle** (found 2026-11-02, the beta.2 release-fix lane;
  no device access then). From its build files, kindle-hid-passthrough 3.18 is a wrapper that execs
  `dist/ld-linux-armhf.so.3 --library-path dist dist/main.bin --daemon` from its folder, so
  `pgrep -f kindle-hid-passthrough` cannot find it and bluetooth.sh now finds Grimoire's copy by its
  folder. Confirm on the device with the helper running: `tr '\0' ' ' < /proc/<pid>/cmdline`.
- **Other scenarios timed by the real time between their lines.** `GRIMOIRE_SIM_LATE_MS` shows them
  (`make scenarios SCEN_ENV=GRIMOIRE_SIM_LATE_MS=60`); make check runs only the two that failed on the
  release runner that way (`LATE_SCENARIOS`). v_gestures_4e_single failed once there too (2.0.0-beta.0).

- **The stroke eraser on very dense pages.** Taking a stroke out of the middle still draws what was
  under it again (undo and redo of it are instant since rc.10). On a 1,090-stroke page that can take
  seconds a sweep.
- **Brushes on the Kindle.** A dense page still takes about 30 s to draw the first time (marker 23 s,
  charcoal 10 s of it). Exact speedups left: a per-dab table for core pixels (no sqrt, no powf), the
  graphite path's per-pixel divisions.
- **Times past 2^32 ms (49.7 days of uptime).** A touch gesture crossing the wrap of the frame clock
  fails (found by a unit test); the 2^31 case is fixed in rc.10.
- **The KOReader plugin as a book (owner idea, 2026-10-31).** Installed as a file KOReader's library
  shows, removed by long-press > Delete like any book, instead of a plugin folder. Needs research on
  KOReader's file browser and a precedent before building.

- **Kindle Click (Amazon's Bluetooth page turner, 2026).** Grimoire accepts any input device that
  sends ordinary key codes, so it works if the firmware exposes it as an input device while
  Grimoire runs. Test: pair it in the Kindle's settings, list `/proc/bus/input/devices`, then try
  Settings > Shortcuts > Buttons.

## Deferred from beta.2 lane C (links between notebooks; lane C's decisions, 2026-11-02, for the owner to confirm)

- **A page linked only from other notebooks has no dot in its own Graph.** The brief asked to keep the
  linked-only layout and the map's shape stable, and incoming links change without this notebook being
  edited (another notebook saved), so they never make a page a layout node: its anchor box says
  "-> p. N" instead of drawing a line. Making such pages nodes is a one-line change in graph_build, at the
  cost of the map moving when another notebook changes. Amended by the Graph of all notebooks (graph
  lane, 2026-11-02): in All notebooks such a page has its dot (an incoming link from any notebook makes it
  linked); the item stays open for one notebook's view only, whose graph.ini never depends on other
  notebooks being saved.
- **The C tree has no backlinks** (Rust only, PORTING.md). A notebook it changes is read again at the next check.
- **Linked from lists at most 64 rows** (LB_FROM_MAX) and the cache at most 40000 links between notebooks
  (backlinks_partial says while a notebook is left out; nothing shows it yet). A notebook left out is kept
  as "not read" and taken in by itself once another notebook's links make room.
- **Broken links are display-only**: nothing removes them by itself (a notebook restored from the trash
  makes them work again). A "remove every broken link" action was not built.

## Deferred from beta.2's Graph of all notebooks (proposed by docs/beta2/GRAPH-ALL.md, 2026-11-02; the owner decides)

- **The Graph as a place of its own, beta.2 graph lane's choices (2026-11-03; the owner to confirm).** Built
  from the owner's words of 2026-11-03 (docs/beta2/GRAPH-ALL.md section 10). Choices the lane made: (1) a
  left-out notebook's links are not drawn at all, as links to a notebook that cannot be read are not (the
  page with such a link stays linked only through its other links); (2) in All a left-out notebook's pages
  are not drawn even when it is the open notebook (its own This notebook view, which showed it, went with
  the Notebook chip); (3) the Graph from the Library has no List tab either (it lists the open notebook's links,
  and none is open); (4) the editor's Library button, from a notebook the Graph opened, also returns to the
  Graph (the Library as it was left), and one more Back reaches the cards; (5) the Library's top bar got a
  **Graph** button, left of New, taken out again by the owner the same day ("The graph button if there is
  to be one shouldn't be at the top -- we had good visual soundness with 5 icons. For now just the drag to
  summon on the main menu."): the top bar has its five icons, and `index-map` still works over the Library; (6) a notebook in a left-out
  folder shows a greyed "Left out of the Graph with its folder" row (the folder's menu brings it back); a
  notebook's own setting under a left-out folder is kept for when the folder comes back.
- **"Entirely" left out, for the owner to rule** (graph lane review, 2026-11-03). The owner asked to omit a
  notebook or folder "from the graph view entirely". With the Notebook chip removed, a left-out notebook still
  shows only past the Graph of all notebooks' caps: as boxes at the edge of another notebook's own Graph (its
  anchors), and in the notebooks' browser the Library's Graph opens there (which lists every notebook and
  draws the one picked). Whether either should go is the owner's call.
- **The Library's Graph past the caps** (beta.2 graph lane review, 2026-11-03). Past the Graph of all notebooks' caps the Library's Graph opens the notebooks' browser alone (the lane's
  choice, the reviewer's finding, kept when the Notebook chip was removed; the owner may prefer something
  else, such as the largest notebook, or the first notebooks up to the cap).
- **Left out of the Graph, not done** (graph lane, 2026-11-03): no "Show in the Graph" from inside the Graph
  itself (the setting lives in the Library's menus only); a notebook left out keeps its anchors in other
  notebooks' own Graphs (one notebook's view is unchanged); no starter key for `index-map` in a new
  shortcuts.ini (existing files are never rewritten).
- ~~Orphans in All notebooks.~~ Decided by the owner (2026-11-03): "the view for unlinked pages is beyond
  unnecessary, only linked pages should be shown, and from all notebooks". The orphan build (9a1bfef) was
  taken out again; graph-all.ini's lines from it are dropped when read. Not to come back.
- ~~The Notebook chip.~~ Decided by the owner (2026-11-03, testing on the Kindle): "you left on the all
  notebooks button btw - just getting to testing. fold it in if you can", then, asked whether to fold its
  choices into View or remove the notebook choice: **"remove it."** The chip, its menu, This notebook and
  Other notebook... are gone; the Graph is always every notebook's (less those left out), View is as it was,
  "Reading notebooks" is on the panel's counts line. One notebook's view stays only as the fallback past the
  caps (docs/beta2/GRAPH-ALL.md section 11). Not to come back; remembering a pick is moot.
- **Renaming another notebook's page from the Graph** (writing its nav.ini): Rename is off there.
- **Back reopening the Graph after following a node**: built for the Graph opened from the Library (owner,
  2026-11-03: the Graph as its own place); the editor's own Graph still does not (Back there returns to the
  page the node was followed from), nor do anchors.
- **Filters** (a folder, a search; "find a page in the graph" is below). (A Notebook filter was built and
  removed by the owner's ruling of 2026-11-03.)
- **Folding backlinks.ini into graph-index.ini** (one scan instead of two) once both have shipped.
- **Building the first-ever index in a bgjob child** instead of idle ticks, if the device shows the idle
  fill too slow for big libraries (the log says what a check cost: "libindex: N of M notebooks read again").
- **The library index waits for the backlinks check** (graph lane, 2026-11-02): one cache's work a tick, so
  after a notebook opens the Graph's index is checked once backlinks.ini's check is through. Fine on the
  host; to watch on a big library on the Kindle.
- **Pathological chains lay out long** (graph lane, 2026-11-02): a notebook whose pages link only in a long
  chain (page 1 to 2 to 3...) is laid out as a long thin line in either Graph (the seed's rings; measured on
  the host, a 200-page chain's edges are 4 to 15 rest lengths). Real notebooks were not seen to do it; a
  chain-aware seed is possible.

## Not taken from the RC8 review (revisit when there is room)

- Live view: decode, compose and average in a Worker with OffscreenCanvas; optional 4-bit gray
  tiles when the link is congested.
- Graph: find a page in the graph and Around history. (Other notebooks as anchors: built in beta.2, lane C.)
- Remove the retired status-bar code (`status_reserved()` is always 0) and the graph's satellite
  and tray code unused since RC5 (about 250 lines in mapview.rs).
- A tree-wide rustfmt pass over leftovers of an earlier mechanical reformat.
- Wording left as is: "Somewhere Else" (the only Title Case label), and the RC1 changelog section
  that later RCs reversed.

## Ideas only (from a survey of other Scribe note apps, 2026-11-03; the owner decides, nothing scheduled)

Ideas, not plans: each needs the owner's go and its own design. No code is taken from anywhere.
- **Editable PDF export:** a book page's or a notebook page's ink as real PDF annotations, so other readers can edit it.
- **A restorable trash with a timer:** deleted notebooks and pages kept for 30 days, with Restore.
- **Search over page names** (and, much later, over recognised handwriting).
- **Resize and rotate in the lasso:** the natural next step after beta.2's move, copy and paste.
- **Typed text boxes** on a page, with the keyboard.
- **In-app guide cards** with a "Show me" that walks through a gesture on the screen.
- **A "Test pen and touch" screen** that bug reports can attach (pressure, tilt, offsets, touch slots).
- **An issue form** that asks for the device model, firmware and Grimoire build.
- **More languages from community translations:** the gettext plumbing is there (docs/TRANSLATING.md).

## Product

- Send the fork outreach note (draft on the owner's Desktop) and keep the README's "Name, logo and
  forks" section current.
- Optional pay-what-you-want supporter path ($0 allowed); not set up.
- Promote a quiet 2.0.0 release candidate to 2.0.0 stable (the stable KPM feed updates every 1.2.0
  install).

## Done

- 2.0.0 (unreleased, v21/i16; issue #16, reported by nebuff: "Toolbar disappears in live view when a popup menu
  (settings, display settings) is opened then comes back after closing menu"): the toolbar's Live view layer
  stays where the panel is under every menu, sheet and bar of the notebook, holding what the Kindle shows there
  (the toolbar, and a menu's part over it), and is none only under the Pages view, the Graph and the notebooks'
  browser, which cover the screen (docs/DESIGN.md, Live view). In the headless runner the layer went as the
  Display sheet or the Paper sheet opened, a setting changed or not (76,089 pixels of the toolbar's rect wrong in
  the browser beside the Display sheet); Settings over the notebook, and its own Display sheet, were already
  right, the frame then holding the whole screen. tests/scenarios/y_live_toolbar_menus.qs checks it with two new
  probes, `live.panel` and `live.stale.X.Y.W.H` (docs/SIM.md).
- 2.0.0-beta.2 (unreleased, b2g/del; the owner, 2026-11-05: "hold del long enough in keyboards to quick delete",
  "low ish delay but not too low", "repeat rate high so it feels snappy", "not too high"): the on-screen keyboard's
  Backspace auto-repeat, from the RC8 review's list. Delete held repeats on every keyboard after 400 ms, every
  65 ms, never queueing a refresh behind the panel (docs/DESIGN.md, Text and the keyboard). Left as it is: a
  hardware keyboard's held Backspace on a count pad deletes one digit, as `hidkeys_event` drops the kernel's
  autorepeat for every key (text fields take no typed keys at all, only Esc); repeating it would let autorepeat
  into the path every shortcut shares, for a count of a few digits.
- 2.0.0-beta.2 (unreleased, claude/b3-pv3; the owner's rulings of 2026-11-03, verbatim): (a) "The graph button if
  there is to be one shouldn't be at the top -- we had good visual soundness with 5 icons. For now just the drag
  to summon on the main menu." The Library's top bar is back to its five icons (Exit, Settings, Share, Folder,
  New); two fingers down over the Library still open the Graph, and so does the key bound to `index-map`.
  (b) "Make the pages undo mapped to the same undo flag that the controller sees for it in pages, just keep them
  seperate to each other. That solves it right? And add the redo button." While the Pages view shows its grid,
  the `undo` and `redo` actions (keys, controller buttons, gestures) are its Undo and Redo of the last page move
  or batch of new pages; on the page they stay the page's own. The undo action works with pages ticked too (the
  bottom row only hides its Undo button then, for Move to...'s place). Redo is one step: an Undo makes it, a new
  move or batch drops it, and it applies only while the pages are the list the Undo left. The bottom row shows
  Redo beside Undo's place, in Copy to notebook...'s (disabled with nothing ticked anyway), with nothing ticked.
  A batch's Redo writes the same pages again with the same ids at the same places (each file the blank page the
  batch wrote: the Undo only took them back while they were still that; an image paper listed again in
  bg/pages.ini); a file in the way that is not that blank page refuses it. (c) "Also no need to have the page
  outline show when dragging, just show the black bar." A dragged tile shows only the gap's black bar: the same
  gap refreshes nothing, a new gap refreshes the bar's old and new places, each on its own fast refresh. (d)
  "Batch added pages should have whichever page layout the SELECTED page has - and fall back to the open page if
  none are selected." A batch takes the paper of the page it goes after: in the Pages view the last page ticked
  (with several ticked, the last in page order, which is where the batch goes), else the open page; the
  toolbar's count pad, which goes after the open page, its paper. A tap on New page is a batch of one through
  the same code, so it follows the same rule. (A document picture's paper still falls back to the notebook's,
  store_new_page_paper.) The paper is read from the page file's first 20 bytes (store_page_paper), not its strokes.

- 2.0.0-beta.2 (unreleased, claude/b3-padd2; owner 2026-11-03, "make the page add button not enter the page when
  its made -- and add the long press dialog to the toolbars page adder -- two birds one stone. swiping still works
  for new pages and nav so."). The b3-padd2 agent's reading: "the page add button" is the Pages view's + (New
  page), the button whose tap entered the page it made; "the toolbars page adder" is the toolbar's next-page
  arrow, which turns into a + on the last page; and "swiping still works for new pages and nav" says the editor's
  way of adding and going to a page stays as it is. So: a tap on the Pages view's + adds one page as a batch of
  one (its toast, "Added 1 page: it is page N.", its Undo in the bottom row) and the view stays, on the new
  page's screenful, its tile labelled "Page N (new)" while Undo would take it back (drawn with the grid: no
  refresh of its own). The toolbar's + held 600 ms (the Pages view's hold), or tapped with Shift, Ctrl, Alt or
  Cmd held on a keyboard, opens the same count pad along the bottom over the page; Done adds the pages right
  after the open page (on the last page, the only place the + shows: the end, where its tap adds one), the
  editor stays on its page, and a toast says how many, from which page, and "Undo them in Pages": the batch is
  the notebook's Undo record (pv.add_undo), offered in the Pages view's bottom row under the batch's rules (the
  editor's Undo is the page's own history). Cancel, Esc, Back, Close menu or a tap outside closes the pad with
  nothing added; a full notebook says so instead of opening it; a contact made while a batch is written is let
  go to its lift, as in the Pages view. The toolbar's plain tap still adds a page and goes to it: it is the same
  code as the next-page key and the two-finger swipe (panel_act P_NEXT), which the owner said keep working, so it
  was left alone; owner to say if the tap should stay on the page as well. The `new-pages` action (86, no row in
  the Buttons sheet, which is full: bind it in shortcuts.ini, e.g. `key.n=new-pages`) opens the toolbar's pad on
  any page, the pages going after the open one. The `new-page` action still adds one page and opens it. A
  notebook full of pages: New page tapped in the Pages view says the notebook is full (it said "Could not add a
  page"). Not done: no default key for `new-pages` (Ctrl+N stays `new-page`).

- 2.0.0-beta.2 (unreleased, claude/b3-padd; owner 2026-11-03, "make the page adder an instant addition with
  a modifier which prompts a text field for a number of additions to add. a number pad will work fine here,
  and a -/+ at the bottom"): + in the Pages view adds one
  page at once (as before); held 600 ms (a held tile's time), or tapped with Shift, Ctrl, Alt or Cmd held
  on a keyboard, it opens a count pad: the number pad with a - and + row and an "Add N pages" Done (off at 0), at most what
  the notebook still holds (2000 in all; a toast when the count meets it). A hardware keyboard types in it
  (digits, Backspace, - + and the keypad's, Up and Down, Enter, Esc). One store operation (one notebook.ini
  write; a failure adds none), on New page's paper and at New page's place; annotated notebooks take them as
  New page's note pages. The b3-padd agent chose the Undo: yes, as Move's (the bottom row's Undo, in Move
  to...'s place) while the pages are still blank, none was saved or named or linked since and the open page
  is not one of them; otherwise not offered. After review (b3-padd agent, 2026-11-03): from 25 pages the pad
  goes and "Adding N pages..." shows before the files are written (one refresh; 15 to 25 ms a page on a
  Kindle, so up to about a minute), and a contact made while they were written is let go down to its lift
  (pv.quiet_until), so no queued tap lands on the grid drawn after. No soft cap below the notebook's 2000:
  the toast says it is working. Undo is now not offered either once a page of the batch is named or linked
  (pagesview_add_in_use, the check Undo itself makes), so it never shows only to refuse. A batch on a picture
  paper whose notebook.ini fails unlists its pages from bg/pages.ini in one write, not one a page. The b3-padd
  agent read "a -/+ at the bottom" as under the digits, with the full-width Done below them as on every number
  pad; owner to confirm, or say if - and + should sit under Done.

- 2.0.0-beta.2 (unreleased, claude/b2-pv; owner rulings 2026-11-02, "Move to can be an undo step, yes. Yes
  drag to reorder after holding."): a page move is one undo step, offered as Undo in the Pages view's bottom
  row (in Move to...'s place, nothing ticked) until the pages change again or the notebook closes; the
  editor's Undo button stays per page. Drag a held tile to a gap (deferred by lane A, now done): an outline
  and a gap bar on fast refreshes (the outline taken out in claude/b3-pv3: the bar alone), the screen turns over the top bar or the bottom row, one undo step.

- 2.0.0-beta.2 (unreleased, claude/b2-fhold): **no panel button for Select** (owner ruling 2026-11-02: the
  panel has no room for one). The finger's way in instead: a finger held still on the page off a link for
  the link hold (0.6 s) arms a lasso, shown by a small dashed ring; drawn on without lifting it selects as
  the pen's lasso does, with the same bar; lifted without moving it pastes the clipboard there (nothing when
  it is empty). A hold on a link still opens its card. Guards (no surprises): a palm (TOUCH_F_BROAD, or
  touchgest's palm and rest rules, which end the finger with a cancel) never arms, nor does a hold with
  the pen in range at any point from the down (or gone under 500 ms before it); pen activity after the
  arm drops the lasso, its ring cleared, the lift ignored. No switch: no Settings page lists finger behaviours
  of this kind (the Gestures sheet binds actions to gestures). Select also stays in the tool picker and the
  bindable `select` action.

- 2.0.0-beta.2 (unreleased, claude/b2-refresh): issue #11. A toolbar drag's release cleans all of its trail
  with one Clean refresh (a flash of that rect alone, a new display mode), and draws again every place it went
  (a drag that went round left the page between its ends dithered); a Select drag's end does the same in
  grays. DU wear (display.rs DuWear): the stroke eraser, the precise eraser and white ink, and a dragged
  selection's bar and outline count what fast refreshes turned back to paper; past a quarter of the screen,
  at the next pause, one Clean of where it was. Test probes `display.stale.X.Y.W.H` and `editor.wear`.

- 2.0.0-beta.2 Graph as its own place (owner, 2026-11-03; graph lane, docs/beta2/GRAPH-ALL.md section 10):
  orphans taken out of the Graph of all notebooks again (the owner: "beyond unnecessary"; graph-all.ini's
  orphan lines dropped when read); Leave out of the Graph for a notebook (notebook.ini graph=0) and a folder
  (root/graph-omit.ini, nested folders and notebooks moved in included); the Graph opened from the Library
  (two fingers down, the index-map key; the top bar Graph button it also had was taken out again, the
  owner's ruling of 2026-11-03: "The graph button if there is to be one shouldn't be at the top -- we had good
  visual soundness with 5 icons. For now just the drag to summon on the main menu.") with no notebook open, no List or Pages tab
  and no This notebook, a node opening its notebook and Back returning to the Graph.
- 2.0.0-beta.2 the Notebook chip removed (owner, 2026-11-03: "remove it"; docs/beta2/GRAPH-ALL.md section 11):
  the Graph is always every notebook's, no This notebook or Other notebook..., "Reading notebooks" on the
  panel's counts line; one notebook's view only past the caps.

- 2.0.0-beta.2 Graph of all notebooks (owner ruling 2026-11-02; claude/b2-graph, docs/beta2/GRAPH-ALL.md):
  the Graph opens on every linked page of every notebook, islands per notebook with captions, a Notebook
  chip for this notebook or another, root/graph-index.ini (every notebook's pages, names and links, a cache
  kept on idle ticks) and root/graph-all.ini (the layout by notebook and page id), a new bgjob kind for big
  layouts, new notebooks placed as blocks beside the kept layout.

- 2.0.0-beta.2 (unreleased, claude/b2-relfix): the release runner's two scenario failures fixed (touch
  lines stamped by the script's clock, frames without waits, `make scenarios-late`); bluetooth.sh stops
  and takes back the helper in Grimoire's folder by its folder, recorded or not, with its cursor overlay.
- 2.0.0-beta.2 lane C (links between notebooks, polish; on claude/b2-links, not yet merged): Linked from
  rows in the Links bar and a count on the Pages card, from root/backlinks.ini (a cache); Graph anchors for
  other notebooks' pages, both ways; broken links (page or notebook deleted) dashed, "Page gone" /
  "Notebook gone", one tap to their card (Change where it goes, Delete link); Back across notebooks keeps
  the page, zoom and offset and that notebook's back list.

- 2.0.0-rc.12: folders movable; keyboard apostrophe and symbols; Pages Rename/Delete in the held page's menu and flicks instead of arrows; Graph pen tap selects; Smallest shape is the real floor; ellipses and parallelograms snap; Corner roundness; dithered link outlines; stroke eraser tiles; lost-lift recovery and input diagnostics; ink picker; clock medium; Live view Brightness.
- 2.0.0-rc.11: What's new under the bat (look C, picked by the owner): the owner's note (`docs/WHATSNEW.md`)
  and hand signature with the bat (`assets/signature.pgm`, from the notebook "2.0b1 sig"); opens once after an update.
- 2.0.0-rc.10: instant undo and redo on any page; dense pages open from their kept drawing; the
  heartbeat goes on through long drawing; no background redraw of the open page; faster exact
  brushes on the Kindle; Live view playout smooth through Wi-Fi stalls; gestures past 24.9 days of
  uptime.

- 2.0.0-rc.9: one-finger page turns; pen taps in the Graph; papers in three rows with cross grid,
  isometric, hexagon and Seyes; Custom music 8-line tab, presets and new default; Smallest shape
  slider; Live view picture in picture for Firefox; the toolbar kept in Live view beside the tool
  picker.
- 2.0.0-rc.8: taps no longer swallowed near the pen off the page; sliders keep their value on a
  cancelled touch; Live view rail, size control and picture in picture tiers; steadier, faster
  graph; Pages and Graph untangled.
