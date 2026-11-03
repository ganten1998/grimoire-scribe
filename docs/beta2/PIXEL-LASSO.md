# Exact-area ("pixel") lasso and the live selection preview (2.0.0-beta.2, lane B3)

Status: built on `claude/b3-lasso`, Part A (the live preview, section 2) and Part B (exact area,
section 1); see "Build status" at the end for what differs from the text and the measurements. Base: `claude/b3-lasso` cut from `claude/beta2` at d6313b2
(the shipped lasso of `docs/beta2/LASSO-DESIGN.md`: `rust/src/lasso.rs`, `rust/src/editor/select.rs`,
`ActKind::Lasso`, `root/clipboard.qpg`).

The owner's words (2026-11-03):

> "The lasso tool is good, but it should show what's been selected in the area while drag is
> occurring and not just on letup. Also what about pixel lassoing?"

So: question 2 (a live preview while the lasso is drawn) is a request; question 1 (cutting strokes
along the lasso) is a question, answered here with a recommendation. Both must work with the pen and a
finger, and with a keyboard paired with the Kindle.

## 0. Summary

| Topic | Recommendation |
|---|---|
| What "pixel lasso" means here | **Exact area**: strokes that cross the lasso are split into inside and outside pieces at the brush's own dabs; inside pieces are selected, outside pieces stay. Pieces are ordinary strokes. |
| Exactness | Cut at a dab boundary: the pieces together draw the original stroke dab for dab (positions, radii, opacity, nib, charcoal dust). Only the per-dab opacity jitter of pencil, mechanical pencil and charcoal re-rolls after a cut, and the calligraphy nib re-starts its smoothing. A piece's cut end is the dab's own round (or nib-shaped) edge, as if the pen lifted there. |
| Erase strokes | Never cut, never selected. Originals stay whole in place; copies travel with the inside pieces exactly as today (carried erasure, floors). Floors keep working because the outside pieces take the original's place in the stroke order. |
| Undo | Cut plus move (or cut plus delete) is one `ActKind::Lasso` step with one patch: instant. Copy changes nothing on the page. |
| Format | None. No header bump, no tool id, no chunk, no settings key. Pieces are ordinary strokes readable by every version from 1.0.0-beta on. |
| Choosing the mode | Whole strokes stays the default. A toggle on the selection bar ("Whole strokes" / "Exact area") re-judges the same lasso at once; the choice lasts until Grimoire restarts. Alt (Option on a Mac keyboard) held when the lasso starts flips it for that lasso (as built: Shift as well; see 1.10); a bindable action `select-exact`. The live preview shows the cuts before the lift. |
| Raster selection | Not as a page object (reasons in 3). A "Save selection as picture" export is a BACKLOG option. |
| Live preview | Incremental and exact: the polygon closed from the current point is tracked as a running winding number per spine sample, updated by the small polygon each batch of new points adds, over a grid of stroke boxes. Selected ink gets a dithered halo (the house style of link outlines and the selection box), refreshed in small DU rects about 8 times a second; the lift's GL16 redraw restores grays. The lift then reuses the preview's answer. |
| Cost on the 1,090-stroke page (estimates) | Preview: under 1 ms to set up, about 0.5 to 2 ms of work per batch on the Kindle, the visible-spine work of the lift spread over the drawing. Exact-area lift: 20 to 150 ms more than today, only for strokes that cross the line. |
| Size | About 1,500 lines of code and 1,150 of tests; about 6.5 days in two parts that merge separately: A, the preview (2.5 d), then B, exact area (4 d). |

## 1. Question 1: exact-area lasso

### 1.1 What the code does today (checked at d6313b2)

- **Strokes are dab trails.** `brush_add` (brush.rs:616) stamps one dab at a stroke's first point,
  then walks each segment stamping a dab every `spacing_for(r) = max(0.15 r, 0.5)` px, carrying the
  remainder (`b.carry`) into the next segment. A dab is `interp(prev, s, d / len)`: position,
  pressure, tilt and time all linear between the two samples. There is no end cap, taper or
  per-stroke buffer: a stroke ends with its last dab, which lies within one spacing of the last
  point, and its visible end is that dab's own edge (round, or the nib's shape for the fountain
  pen's italic nib, the calligraphy pen's broad edge and the highlighter's vertical tip).
- **Per-dab state that is not position:** `dab_index` (jitter: `hash2(seed, idx)`, only for tools
  whose look has `jitter > 0`: pencil 0.10, mechanical pencil 0.06, charcoal 0.12, all scaled by
  `grain`), and the calligraphy nib's smoothing (`nib_c`, `nib_s`, a 1.5 mm exponential follow of the
  pen's lean, `NibKind::BroadEdge` only). Charcoal dust hashes absolute x and y with the seed
  (brush.rs:954); pencil tooth is absolute position. Opacity compensation (`share`) depends on the
  dab's own radius only.
- **No splitting exists anywhere.** The precise eraser is an erase stroke (restore toward paper or
  floor), not geometry; the stroke eraser removes whole strokes. There is no model to reuse, so the
  cut below is new, and it is built on the brush's own walk so it cannot drift from what is drawn.
- **The enclosure rule** (lasso.rs `encloses`) is length-weighted: a stroke is selected when at least
  half of its visible spine length (samples at its points and at the middles of pieces at most 1 mm
  long) has a non-zero winding in the lasso's crossing table; under 1.5 mm visible, by the centroid
  of its visible samples.

### 1.2 The cut: at a dab boundary, so the pieces draw the original

For a candidate ink stroke S (bbox meets the lasso, not entirely inside or outside it, see 1.6):

1. **Dab walk.** Run S's brush dry (`b.dry = true`, as `stroke_tiles` does) and log each dab: its
   sample (the `PenSample` the brush stamped), its radius, and the index of the segment it lies on.
   The only change to brush.rs: in `stamp`'s dry branch, push the sample to an optional
   `BrushState.dab_log: Option<Vec<PenSample>>` (set by hand after begin, like `dry` and `mask`).
   The segment index is known to the caller, which calls `brush_add` once per point and counts the
   dabs each call logs. Nothing changes when the log is `None`, which is every draw.
2. **Classify** each dab by its centre: inside = `Lasso::inside(m.x, m.y)` (the same non-zero
   winding as the selection). Runs of equal class are the pieces. A stroke whose dabs are all inside
   is selected whole; all outside, untouched.
3. **Build a piece for dab run [j0, j1):**
   - **Start:** if j0 = 0, S's first point (the piece is a prefix). Else the logged sample m(j0)
     itself: the brush's own interpolation, so the piece's first dab is the original's dab j0.
   - **Middle:** S's points strictly after the start and before the end.
   - **End:** if j1 is the last dab, S's remaining points. Else E, the point at arc length half a
     spacing (`spacing_for(r(j1 - 1)) / 2`) past dab j1 - 1 along S's polyline. The original's next
     dab (j1) sits a whole spacing past j1 - 1, so the piece stops after j1 - 1 with no f32 doubt about
     a dab landing exactly on its last point.
   - **Quantize** the new points to file precision at once (pressure to 1/65535, tilt to 0.01 degree,
     as the point record stores them; t_ms interpolated), so the page drawn live, saved and read back
     is the same bytes.
   - Same tool, params, ext (colour, tip variant; ext[1] is 0 on ink) and seed as S (1.5).
4. **Why it is exact.** The prefix piece replays S's first segments with the same carry, so its dabs
   are S's dabs 0 .. j1 - 1. A later piece starts with a dab at m(j0) (same sample: same radius,
   opacity, nib angle target), then sets its carry to `spacing_for(r(j0))`, exactly the distance S's
   brush went on to its next dab, along the same line: the same positions from there on.
   Mathematically identical; in f32 a coordinate can differ in its last bit, so a rare pixel may
   differ by one gray level (the same caveat, and the same counted test, as a lasso move in
   LASSO-DESIGN.md 2). The pieces sit together at S's place in the stroke order and deposit in S's
   dab order, so where two pieces' dabs overlap at a cut (the lerp deposit is order-dependent) the
   order is S's too.
5. **What is not exact, and how much:**
   - **Jitter re-rolls** on every piece but the prefix (its `dab_index` restarts at 0). About
     `2r / spacing = 13` dabs cover each pixel on the spine, so the per-dab jitter averages out: the
     builder measures the largest and mean pixel difference of a cut pencil and charcoal stroke's
     union against the original and the test pins them (expected: one or two gray levels on a few
     pixels). Ballpoint, fineliner, fountain pen, marker, highlighter and calligraphy have no jitter.
   - **The calligraphy nib** of a later piece starts at the pen's lean at m(j0) instead of the
     smoothed angle, and catches up within about 1.5 mm. Visible only where the lean changed in the
     1.5 mm before the cut. With calligraphy's default `tilt_size = tilt_opacity = 0`, the first
     sample's tilt could be rewritten so its nib target equals the smoothed angle; that is an option
     for stage B6 if the measured difference shows (the test reports it), not part of the base.
   - **Cut ends.** Each piece ends with its last dab's edge: up to one dab radius past the lasso line
     (half the stroke width: 0.15 to 0.25 mm for a ballpoint, 2.5 mm for the 5 mm highlighter, 0.9 mm
     for calligraphy). This is what a vector cut can be. A straight edge along the lasso line would
     need a clip shape stored with the stroke, which no version reads (a format change and older
     versions would draw the whole stroke): rejected.

### 1.3 What happens to a page (move, cut, delete, copy)

The selection, in exact mode, holds a **cut plan**: the ids of whole-selected strokes, and for each
crossing stroke its id and its dab runs (inside or outside). The page does not change at the lift:
only a gesture that changes the page applies the plan, inside its own undo step.

- **Move, Cut, Delete** (inside `step_begin` / `step_end`, as now):
  1. **Splice:** each crossing stroke S at index i is replaced by all its pieces, in dab order, at i
     (outside and inside pieces interleaved as along S). Every other stroke keeps its relative order.
  2. The selection is now the whole-selected strokes plus the inside pieces (by index). From here
     the shipped code runs unchanged: `lasso_lift` (carried erasure and floors), take the selection
     off, redraw the old place with `page_render_tiles` over the removed strokes' tiles, and for a
     move `lasso_place` at the end of the page.
  3. The old place comes out right by itself: the outside pieces are still on the page, at S's
     place, drawing S's outside dabs, so the tile redraw draws exactly what remains.
- **Copy** (no undo step, the page must not change): splice the pieces in, lift, and splice the
  originals back, in one function with no drawing, saving or yielding in between (a debug assertion
  checks the page's stroke ids are restored). That is O(crossing strokes), not a copy of the page
  (cloning the dense page's 300k points would cost 30 to 60 ms on the Kindle). Copy's existing
  pruning of entirely erased ink (`stroke_visible`) applies to pieces like any stroke.
- **Paste** is unchanged: the clipboard holds ordinary strokes.
- **Read-only pages:** Copy works; Move, Cut and Delete are refused as today.

### 1.4 Erase strokes and floors

- **Erase strokes are never cut and never selected**, in either mode (LASSO-DESIGN.md 2.1 stays:
  white ink is erasing, not an object). The originals stay whole where they are.
- **Floors still hold.** An erase stroke e affects ink with index in (f(e), idx(e)). The outside
  pieces of S take S's place, so every relation S had with an erase stroke or a floor marker (before
  it, after it, inside a scope) holds for each piece. Markers are found by position, not number, so
  inserting pieces moves nothing.
- **Carried erasure** is computed by the shipped `lasso_carried` on the spliced page: erase strokes
  after an inside piece, in scope, whose dab tiles meet it. Copies travel whole, as now. A whole copy
  reaching past the inside piece is harmless inside the group (over no group ink it restores pixels
  already at the floor). At the destination, `lasso_place` decides plain or scoped as now; a long
  carried eraser over destination ink is scoped more often than a trimmed one would be, which only
  matters to older versions' fidelity (LASSO-DESIGN.md 4).
- **Optional trim (BACKLOG, not built):** cut carried erase copies to the dab runs within reach of
  the group's ink, with the same dab cut (erase strokes are drawn by the same brush). Fewer scoped
  floors and a smaller clipboard. Each trimmed piece keeps the copy's layer byte and comes after its
  marker, so floors are unaffected.

### 1.5 Seeds, links, ids

- **Seeds.** Every piece keeps S's seed. Charcoal dust is keyed by position and seed, so outside
  pieces keep their exact specks; jitter re-rolls either way (1.2). Inside pieces get their final
  seed from `lasso_seeds` at placement, as any group ink: kept when free on the destination, else
  fresh. On a move within the page the outside pieces still hold S's seed, so the moved piece gets a
  fresh one (its dust re-rolls, as a moved word's tooth already does).
- **Shared seeds** between pieces of one stroke are allowed by the format and already occur
  (`lasso_seeds`: "strokes that shared a seed keep sharing one"). They mean the pieces of one
  stroke count as one stroke to a 2.0.0-beta.5 ink link made from one of them later, which matches
  what they were.
- **Linked ink is never cut.** A stroke whose seed is a link's seed on the page (a 2.0.0-beta.5 ink
  link, whose box is its strokes) is judged whole in exact mode too, by the shipped all-or-nothing
  rule: cutting it would split a link's box between two places. Ink inside a link **area** may be
  cut; the area rules (half of the 8 x 8 grid inside, or every ink stroke whose box middle is in the
  area selected) run on the spliced view, where inside pieces count as selected ink.
- **Ids.** Pieces are new boxed strokes (new `stroke_id`s). The originals belong to the undo step.

### 1.6 Which strokes are cut, cheaply

The crossing table answers "is rect R wholly inside" and "wholly outside" in one binary search per
row of R: no crossing in (R.x0, R.x1) on any row, and the winding at R.x0 non-zero (inside) or zero
(outside). A stroke whose bbox (which includes the brush's reach) is wholly inside is selected
whole; wholly outside, skipped. Only strokes whose bbox straddles the lasso line get the dab walk.
`Lasso::rect_inside(r) -> Some(true | false) | None` is new (about 30 lines, unit-tested against
brute force).

### 1.7 Undo

One gesture, one step. `ActKind::Lasso` already has what is needed: its `added` sub-action is an
Add whose `put_back` inserts strokes at ascending indices, and nothing there requires them to be
contiguous. Only the push changes:

- New `history_push_lasso_at(h, p, indices, removed, added_at, links)`: `removed` are the crossing
  originals and the whole-selected strokes with their indices before the gesture (ascending);
  `added_at` are the final indices (ascending) of the outside pieces and of the placed group (move),
  or of the outside pieces alone (cut, delete). The inside pieces were created and removed inside the
  gesture and never appear in history. `history_push_lasso` becomes a call of it with the last n
  indices.
- Undo: take off `added` (back to front by id), put back `removed` at their old indices. Redo: the
  reverse. Both swap the step's one patch: instant on the dense page. `bbox_of` covers both sets
  for a trimmed patch's redraw.
- `history_layers_in_use` is unaffected (pieces are ink, layer 0).

### 1.8 Old versions

The pieces are ordinary ink strokes of known tools with in-range parameters and points on the
page. Every version from 1.0.0-beta reads, draws and re-saves them, and draws them the same way
(the brush is frozen by `test_golden`). Nothing about exact mode adds layered pages or chunks.
Untouched pages are untouched. The FORMAT.md change is one sentence under Moving between versions
("an exact-area lasso writes ordinary strokes: the pieces of a cut stroke").

### 1.9 Performance (estimates; the builder measures and logs them)

Scale (LASSO-DESIGN.md 8 and the BACKLOG measurements): the dense page redraws in about 1.2 s on
the Kindle; the headless Mac harness moved a word on a generated 1,000-stroke page in 12 ms (old
place) plus 22 ms (new place); the Kindle runs these paths about 5 to 8 times slower.

| Step | Light page | 1,090-stroke page |
|---|---|---|
| `rect_inside` over the candidates | < 1 ms | 1 to 3 ms |
| Dab walk of straddling strokes (about 50 to 100 ns a dab on the Mac; 0.5 px to 4.5 px spacing) | < 2 ms | 20 to 150 ms for a lasso through a paragraph (100 to 500 strokes) |
| Splice and piece building | < 1 ms | < 10 ms |
| Lift, place, redraw | as today | as today, plus the pieces' tiles (inside the old-place tiles already redrawn) |
| Copy's splice-and-restore | < 1 ms | < 5 ms |
| Undo/redo | patch swap | patch swap |
| File growth | 64 bytes per new stroke record plus 18 per new point | the same: a paragraph cut adds a few hundred records, about 30 KB |

Log line: `lasso: exact cut=%d pieces=%d walk_ms=%u` beside the existing `lasso: move ...`.

### 1.10 Choosing whole strokes or exact area ("no surprises")

Options considered:

| Option | For | Against |
|---|---|---|
| A setting only | no UI | invisible: a lasso that cuts a neighbour's descender with no visible cause is the surprise the owner rules out |
| A gesture or modifier only (pen button, Alt) | fast for those who know it | undiscoverable on its own |
| **A toggle on the selection bar, re-judging the same lasso** | visible every time a selection exists; reversible before anything changes; works with a pen tap and a finger | one more button on the bar |

**Recommendation:**

- **Whole strokes stays the default**, so the lasso everyone has works exactly as shipped.
- The bar gets a toggle button showing the current mode: "Whole strokes" or "Exact area". Tapping
  it re-judges the **same lasso** (Select mode keeps the lifted lasso's points while the selection
  lasts) and redraws the selection's halo and box at once. Nothing on the page changes until Move,
  Cut or Delete, so switching is free.
- The choice lasts until Grimoire restarts (in memory, no settings key). Every new lasso shows the
  mode on the bar and, in exact mode, the cut marks in the live preview (2.6) before the lift, so a
  remembered mode cannot cut by surprise. Whether to persist it in settings.ini is owner question 1.
- **A keyboard:** Shift held when the lasso starts flips the mode for that lasso only; Alt (Option on
  an Apple keyboard) does too, on a keyboard paired with the Kindle (see "What differs" below). The
  editor tracks the modifiers held (`hidkeys_mods_held`); pen events carry no modifier bits of their
  own, so it is the held state at the lasso's first point that counts.
- A bindable action `select-exact` (actions, Gestures sheet, Settings > Controller > Buttons, pen
  shortcuts, keyboard shortcuts): Select mode in exact area, or the toggle when Select mode is on.
- New strings through `tr()` with TRANSLATORS comments and Spanish in po/es.po: "Whole strokes"
  ("Trazos enteros"), "Exact area" ("Área exacta"), the action's name and description. The bar
  button is about 220 px at 44 px; `make l10n-check` confirms the fit.

## 2. Question 2: the live preview while the lasso is drawn

### 2.1 What the user sees

- As the lasso is drawn, the ink that the lasso, closed from the pen back to its start, would
  select **now** gets a dithered halo: a 50% checker band about 3 ui px wider than the ink on each
  side, with the ink itself kept on top. Ink that would no longer be selected loses it. The closing
  chord is not drawn (the preview shows its effect).
- In exact mode the halo covers only the inside runs of a crossing stroke, and each cut gets a short
  solid tick across the ink (the ink's width plus 6 ui px), so the user sees where strokes will be
  cut before lifting.
- At the lift the selection appears as today (box and bar) and the halo stays on the selected ink,
  redrawn in grays, so the selection says exactly what it holds.

Why a halo: it is the house style of the selection box and link outlines (`canvas_dither_poly`,
`canvas_dither_rect`), it is pure black and white so a DU refresh shows it faithfully, it works at
any zoom (a hollowed-out core, the alternative, vanishes on ink thinner than 3 screen px), and it
never hides the ink. Bounding boxes per stroke were rejected: on handwriting they overlap into a
mesh that says little about which strokes are in.

### 2.2 The incremental rule: exact, cheap

The preview must give the answer the lift gives, or it would mislead. The lift's answer depends on
the polygon closed from the last point to the first, so every new point changes the closing edge,
and in principle the membership of any stroke. The key fact:

> Going from points p0..pn to p0..pm, the closed polygon loses the edge pn -> p0 and gains the
> chain pn -> pn+1 -> ... -> pm -> p0. The winding number anywhere changes by the winding of the
> small closed polygon D = (p0, pn, pn+1, ..., pm). Outside D nothing changes.

D is a thin fan: the new stretch of lasso plus two chords to p0. Only samples inside D need work.

- **Same arithmetic as the lift.** `Lasso::new` computes each edge's crossing of each pixel row's
  middle with one f32 expression and sums integer windings. The builder moves that per-edge row
  crossing into one function used by both `Lasso::new` and the preview, so the preview's winding of
  a sample is the same integer sum the crossing table gives, contribution for contribution: a
  removed edge subtracts exactly what it once added. (Lassos taller than `LASSO_ROWS_MAX`, 8192 px,
  cannot occur on a page; points off the page are handled identically.)
- **Same samples.** Each candidate stroke's spine samples (`spine`) and its visible set (the
  `hides` test against later erase strokes in scope) are computed by the shipped functions, lazily,
  the first time D's box meets the stroke's box.
- **Exact sums.** Sample lengths become fixed point (u32, 1/64 px) in both the preview and
  `lasso_select`, so "at least half of the visible length inside" is decided on equal integers both
  ways (today's f32 sums can only differ from this at an exact tie). Short strokes are judged by one
  pseudo-sample at their visible centroid, computed by the shipped expression.
- **State per stroke:** inside length (u64), total judged length, the selected flag. A sample whose
  winding goes from 0 to non-zero adds its length, back to 0 subtracts it. A stroke whose flag flips
  joins the batch's redraw list. 2.0.0-beta.5 ink links' all-or-nothing rule runs per seed group on
  the same sums. Link areas are judged at the lift, as today.
- **Spatial grid.** At the lasso's start: a grid of 64 px cells over the page listing the ink
  strokes whose box meets each cell (1,090 boxes: under 1 ms). Each stroke's samples are kept in
  chunks of 32 with a chunk box, so a sample test is skipped a chunk at a time. D's edges are walked
  over the grid to visit only the cells D touches (a thin fan touches about its length over 64
  cells), each stroke once per batch.
- **Batches.** Lasso points arrive every 2 ui px (`lasso_keep`), hundreds a second. The preview
  folds the points gathered since the last batch into one D every 120 ms (`PREVIEW_MS`), and at once
  when the pen pauses. A batch costs one thin fan however many points it has.
- **A full lasso.** When `lasso_keep` halves the points (past `LASSO_MAX`, 4096), the polygon changes
  everywhere: the preview recomputes the windings of the samples it has from a fresh crossing table
  (the cost of today's lift once; rare: about 8,000 to 12,000 screen px of lasso).
- **The lift reuses it.** After the last batch (which includes the final closing edge), the preview
  state is the selection: `lasso_done` takes it and skips `lasso_select`, saving the lift's 20 to
  40 ms on the dense page. Debug builds and a property test assert they agree.

### 2.3 Exact mode in the preview

The same windings give inside and outside runs of spine samples (1 mm apart). The halo covers the
inside runs; a tick is drawn where a run changes, halfway between the two samples. The real cut at
the lift uses dabs (1.2), so a previewed cut can sit up to 0.5 mm from the final one, and a stroke
that grazes the lasso by less than 1 mm can be cut at the lift without a previewed tick. Both are
documented in the user docs; the final selection's halo, drawn at the lift, is exact.

### 2.4 Drawing it on e-ink

- The halo is an overlay drawn by `sel_draw` in state `Lasso` (beside `sel_draw_lasso`), so every
  `compose(a, r)` puts it back: it is never baked into the page canvas. For each selected stroke
  meeting r: `canvas_dither_poly` along its spine in screen px, width the ink's screen width plus
  2 x 3 ui px, then the page canvas's ink pixels in that band are put back over it (a min of the
  two), so ink stays solid.
- Each batch: the screen boxes (plus halo) of the strokes that flipped are merged greedily into at
  most 6 rects (boxes within 32 px join; beyond 6, the closest pair joins). Each rect: `compose`, then
  `app_invalidate(r, Fast)`. The lasso's own segments keep their DU refreshes as now.
- DU shows black and white only, so gray ink inside a preview rect is posterized until the lift,
  exactly as along the lasso line today. The lift's existing GL16 redraw of `lasso_rect` is widened
  to the union of every preview rect, so grays come back in that one refresh.
- DU wear: each preview rect's halo area goes to `du_wear_add`, like the drag outline.
- Presents: at most one batch every 120 ms, so the preview adds at most about 8 refresh rounds a
  second however fast the pen moves. Scenarios count `display.presents` with a fixed clock line.

### 2.5 Cost on the 1,090-stroke page (estimates)

| Step | Mac | Kindle (5 to 8 times) |
|---|---|---|
| Grid at the lasso's start | < 0.2 ms | < 1 ms |
| One batch: a thin fan 2,000 px long touches about 60 cells, about 100 strokes, a few chunks each: about 10k sample tests | about 0.1 to 0.3 ms | about 0.5 to 2 ms |
| Lazy spines and visible sets (at most the lift's total, spread over the drawing) | 3 to 6 ms in all | 20 to 40 ms in all |
| Halo drawing per flipped stroke (a dither poly over its points) | about 0.02 ms | about 0.1 ms |
| At 8 batches a second | under 1% of a core | about 1 to 2% of a core |

The pen's own path is untouched; the preview runs in the editor's pointer handling between pen
events and yields with `work_pump` if a batch runs long.

## 3. A real raster selection (copying pixels as an image): why not

Considered honestly:

- **For:** literally "pixel" lassoing; pixel-exact edges along the line; trivially exact across any
  tool, jitter and nib.
- **Against:**
  - Grimoire pages hold strokes. An image object on a page would be a new kind of data: a tool id
    (none may be 10 or more, and a tool id cannot carry pixels) or a chunk that every older version
    skips, so a pasted picture would vanish in 1.0.0-beta to 2.0.0-beta.1 and could be dropped by
    their re-save. Photos exist only as a page's paper (FORMAT.md, Photos), not as objects.
  - The pixels include the paper: ruled lines, grids and image paper would be copied with the ink.
    Separating ink from paper is the approach LASSO-DESIGN.md rejected (about 95% of inked rule
    pixels changed).
  - Erasing breaks: erase strokes restore toward paper or a floor; a pasted picture would need its
    own rendering slot and floor rules, and the stroke eraser could not take part of it.
  - It is fixed at 300 dpi: fine 1:1, blurred when scaled to fit another page size or exported larger;
    the vector pieces stay sharp.
  - The visible result of the vector cut is the same for the ink, without any of the above.
- **Where raster is right:** taking a selection out of Grimoire (sharing it). That is an export, not a page edit: render the lifted
  group (with its carried erasure) over white or transparency through the existing export path.
  Recorded as a BACKLOG option: "Save selection as picture" (owner question 3).

## 4. Tests

**Unit (lasso.rs, history.rs, brush.rs)**

1. Dab log: count, positions and radii equal a real draw's dabs (`dab_index`) for every tool and tip,
   including the 20,000 px corrupt-segment skip and a one-point stroke; `dab_log: None` draws byte for
   byte as before (the goldens).
2. Cut exactness: for every tool and tip, random strokes and random lassos, the pieces drawn in
   order over paper equal the original's render: byte-equal (allowing the counted rare one-level f32
   pixel) for tools without jitter; prefix pieces byte-equal for all tools; for pencil, mechanical
   pencil and charcoal the largest and mean differences pinned; calligraphy's nib difference reported.
3. Quantization: a page with pieces saved and read back draws the same bytes as before the save.
4. `rect_inside` against brute force on random rects and loopy lassos.
5. Cut plan: linked ink (beta.5 seed) never cut; area links on the spliced view; erase strokes and
   markers never cut; a stroke grazed by one dab gives a one-dab piece.
6. `history_push_lasso_at`: undo and redo of exact Move, Cut and Delete restore exact stroke
   identities and order; non-contiguous `added`; the patch swap equals a re-render; one step each;
   Copy pushes nothing and leaves the page's ids and order unchanged.
7. Preview equals lift: for 500 random lassos (loops, figure 8s, back-tracking, points off the page,
   a lasso past `LASSO_MAX`) on fixture pages including erased ink and beta.5 links, the preview's
   final selection equals `lasso_select`, after any batching.
8. Fixed-point sums: a stroke exactly half inside is decided the same both ways.

**Scenarios (headless, `expect editor.live_matches_render is 1` after each step)**

- `lasso_preview_presents`: a lasso drawn slowly across a paragraph; probes (new) report the preview
  count and the halo rects; `display.presents` bounded with a fixed clock line; the lift's GL16 rect
  covers every preview rect.
- `lasso_preview_leave`: a lasso that takes a word in and then excludes it again: the halo appears
  and goes; the final selection excludes it.
- `lasso_exact_move`: a line of writing cut through the middle of a word in exact mode, moved;
  pixel hashes: the left part unchanged in place, the right part at the new place; undo and redo.
- `lasso_exact_erased`: a precise-eraser gap inside the cut piece travels with it; the outside piece
  keeps its own gap (the original erase stroke stays).
- `lasso_exact_copy_paste`: the source page byte-identical after Copy (page file and canvas); paste
  on another page.
- `lasso_exact_toggle`: the bar toggle re-judges the same lasso; Alt held at the lasso's start (key
  events) flips the mode for that lasso only; `select-exact` action.
- `lasso_exact_dense_timing`: the dense fixture, a lasso through a paragraph; the log line present.
- Spanish strings: `make l10n-check`.

**Revert checks** (each new test must fail without its change)

- Start later pieces at a fresh point instead of the dab sample m(j0): test 2 fails.
- Drop the quantization: test 3 fails.
- Recompute the preview's winding with its own f32 expression instead of the shared one: test 7 fails
  on some lasso (the builder keeps the seed that shows it).
- Push the exact gesture with contiguous `added`: test 6 and `lasso_exact_move` fail.
- Make the preview refresh every point: `lasso_preview_presents` fails.

## 5. Plan for one builder (about 6.5 days; about 1,500 lines of code and 1,150 of tests)

**Part A: the live preview (about 2.5 days; merges on its own, it answers the owner's request)**

| Step | What | Size |
|---|---|---|
| A1 (0.5 d) | Shared per-edge row crossing in `Lasso::new`; fixed-point sample lengths in `lasso_select`. Tests 4 (brute force) and 8. | 80 + 120 |
| A2 (1 d) | `LassoLive` (new `rust/src/lasso_live.rs`, pure): grid, lazy spines and visible sets, batched D, windings, per-stroke sums, beta.5 groups, rebuild after halving, final selection. Test 7. | 350 + 300 |
| A3 (0.75 d) | Editor: feed batches from `lasso_step` (`PREVIEW_MS`, pause flush), halo overlay in `sel_draw`, rect merging, DU refreshes and wear, lift reuse and the widened GL16, halo kept in Selected. Probes, scenarios `lasso_preview_presents` and `lasso_preview_leave`. | 180 + 150 |
| A4 (0.25 d) | Docs (user docs on the preview, CHANGELOG line, BACKLOG), one full gate at the tip. | 40 |

**Part B: exact area (about 4 days)**

| Step | What | Size |
|---|---|---|
| B1 (0.5 d) | `dab_log` in brush.rs (dry branch only); `stroke_dabs` in lasso.rs. Test 1. | 40 + 80 |
| B2 (1 d) | `stroke_cut` (runs, start and end points, quantization) and `Lasso::rect_inside`. Tests 2 to 4. | 200 + 250 |
| B3 (0.5 d) | `lasso_cut_plan` (whole, crossing, linked never cut, area links on the spliced view). Test 5. | 120 + 120 |
| B4 (0.25 d) | `history_push_lasso_at`. Test 6. | 40 + 80 |
| B5 (1 d) | Editor: the plan in `SelMode`, splice for Move, Cut and Delete, splice-and-restore for Copy, exact-mode preview (inside runs and ticks), log line. Scenarios `lasso_exact_*`. | 250 + 250 |
| B6 (0.5 d) | UI: bar toggle re-judging the kept lasso, Alt at the lasso's start, `select-exact` action (actions, gestures, controller, pen and keyboard shortcuts), strings with Spanish (`make l10n`). The calligraphy nib option only if test 2 shows it. | 120 + 60 |
| B7 (0.25 d) | Docs: user docs (exact area, the cut ends, the preview's 0.5 mm), DESIGN.md (Select mode), SHORTCUTS.md (`select-exact`), FORMAT.md's Moving between versions sentence, TESTING.md (scenarios), BACKLOG (below, with who decided what), CHANGELOG line; revert checks; one full gate. | 60 |

Gate: `make -j15 check RTARGET=$PWD/target-lane GRIMOIRE_TEST_PORT=8910 SCEN_JOBS=5`, reported with
failing names; a known flaky scenario named, rerun once, reported.

To the BACKLOG (decided by this design, 2026-11-03, owner to confirm):

- Trimming carried erase copies to the group's reach (1.4).
- "Save selection as picture" export (3).
- Persisting the lasso mode in settings.ini (owner question 1).
- A calligraphy nib seed for pieces after a cut, if measured visible (1.2).

## 6. Owner questions (none blocks building)

1. Should the lasso mode (Whole strokes or Exact area) be remembered across restarts? Recommended:
   no, it lasts until Grimoire restarts, and whole strokes is the default.
2. In exact mode, ink carrying a 2.0.0-beta.5 ink link is never cut (it moves whole or stays). Agreed?
3. A raster "Save selection as picture" for taking ink out of Grimoire (BACKLOG): wanted?

Contributor content: none was read for this design, and no injected instructions were seen.

## Build status (lane B3 builder, 2026-11-03)

**Part A, the live preview: built.** `rust/src/lasso_live.rs`, `rust/src/editor/select.rs`
(`preview_fold`, `preview_tick`, `sel_draw_halo`), `lasso.rs` (`edge_rows`, `edge_cross`,
`judged_samples` with fixed-point lengths, `lasso_enclosed`, `lasso_select_from`). Two changes from the
text above, both small: the batch window starts at the first point waiting (so a pause folds a stretch
once, whatever the timing of the previous fold), and the old closing edge is subtracted with its own
forward expression (a test finds edges whose crossing computed from the other end lands an ulp apart and
puts a sample exactly there).

Measured (Mac, release, `cargo test --release --test test_lasso_live -- --nocapture`), a lasso round a
paragraph drawn a point every 2 px and folded every 24 points:

| Page | Setup | Batches | All batches | Worst batch | Mean | The lift's own select (now skipped) |
|---|---|---|---|---|---|---|
| The owner's 1,090-stroke page (p72.qpg, 131,437 points; 498 strokes selected) | 0.20 ms | 72 | 5.66 ms | 0.33 ms | 0.08 ms | 1.6 ms |

The Kindle runs these paths about 5 to 8 times slower: about 0.4 to 0.6 ms a batch, worst about 2.5 ms.

**Part B, exact area: built.** `lasso.rs` (`stroke_dabs`, `stroke_cut`, `stroke_cuttable`,
`Lasso::rect_inside`, `lasso_cut_plan`, `plan_splice`, `plan_unsplice`), `brush.rs` (`dab_log`,
`brush_tilt_for_nib`), `history.rs` (`history_push_lasso_at`), `select.rs` (the plan, the mode button,
`select-exact`, Alt), `hidkeys.rs` (`hidkeys_mods_held`), the simulator's `mods` line. What differs:

- The calligraphy nib seed (1.2, an option in the text) is built: a later piece of a broad-edge stroke
  starts with a point leaning so the piece's nib starts at the stroke's smoothed nib, and a second point
  at the same place (no length, so no dab) with the pen's own tilt. A calligraphy stroke whose tilt widens
  or lightens it is not cut but judged whole (the seed would change its first dab's size).
- Linked ink and uncuttable strokes are judged by the whole-strokes rule within the exact plan, and so
  are short strokes (under `LASSO_SHORT_MM` of what shows: i-dots, accents), which the preview judges by
  the middle of what shows: the plan cutting them at their dabs took ink the halo never showed, or part
  of a dot whose whole halo showed (review, 2026-11-03).
- The Buttons sheet row for `select-exact` is left out (its Editing list is full); shortcuts.ini binds it.
- Shift as well as Alt flips a lasso's mode (review, 2026-11-03).
- On the Kindle the modifiers are read from `hidkeys_mods_held` (modifier keys make no key event there): every attached keyboard's
  modifiers together, kept by input_evdev.rs after each device's key events, resyncs and detaches (a
  keyboard that went away holds none; a page turner's resync never lets go of the keyboard's Alt).

Measured (Mac, release, `tests/test_lasso_cut.rs`):

| | Largest pixel difference of a cut stroke's pieces against the stroke | Pixels that differ |
|---|---|---|
| Ballpoint, highlighter (both tips), fountain pen, fineliner, marker | 1 level | 10 to 105 of 0.35 to 0.93 million |
| Calligraphy (tilt changes only the nib) | 5 levels | 397 of 244,110 |
| Pencil, mechanical pencil, charcoal (jitter re-rolled) | 54, 10, 53 levels | 2 to 4 % (mean 0.11 to 0.39 of a level) |

The exact lift on the owner's page, a lasso through a paragraph: 206 strokes cut into 976 pieces, 385
taken whole: the plan takes 23.4 ms (the whole-strokes enclosure 1.7 ms); splicing in and out 0.19 ms.
The Kindle runs about 5 to 8 times slower: about 120 to 190 ms, inside the 20 to 150 ms estimated for the
walk plus the plan's other work.
