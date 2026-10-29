# Phase 5 implementation brief: the Index map (Rust)

This brief is self-contained: with it and the repository you can build phase 5. It applies
`docs/index-hook/PLAN-V2.md` (the authority; Part S section 11 is this phase's design, 13.3 graph.ini, 14 e-ink), the
owner's brief in `docs/index-hook/README.md`, `docs/index-hook/CLOUD.md` and the contract `rust/PORTING.md`. Brief
decisions are marked in place and listed in section 15.

Line numbers are Rust lines at `c2333a9`; phases 2-4 move them, so search by the function name. Earlier phases' names
are cited from their briefs; verify each against the merged code first (1.1).

The owner's words for this phase, verbatim:

> Clicking the actual index button which will also be at the bottom now opens a more detailed view of the index,
> which has all pages linked as dots (orphans are progressively smaller or even better different shades/dithers or
> hollowed out) a la Navi org roam view or obsidian.

---------------------------------------------------------------------------------------------------------------

## 0. What phase 5 is

One Kindle build the owner installs and tests:

1. **The Index map** (`mapview.rs`, new; PLAN 11): a full-screen view where every page is a dot; named pages labelled;
   fill by how linked a page is (solid, half or quarter dithered, hollow); the open page double-ringed; satellites
   (unnamed, unlinked pages around their section's named page) and the 'Not linked yet' tray.
2. **The graph model and layout** `graph.rs`, merged from the pure branch and finished (1.2), with graph.ini as the
   layout's cache.
3. **Big maps in the background**: a new bgjob kind for layouts over 300 force nodes and for Tidy up.
4. **Interaction**: tap a dot for its card, tap again to go there; one-finger or pen drag pans; pinch zooms; All and
   Around this page; Rename (D3); Unlinked Show/Hide; Tidy up.
5. **Entries**: the Links bar's Index button, a Graph button on the Pages view's Index tab, and the `index-map` action.
6. **Also**: the bar's '%d pages link here' chip (opens the map around the page), '%d links here' counts on the Index
   tab's rows.
7. **Canvas**: `canvas_line_aa` rewritten row-span and bit-identical; `canvas_disc_pattern` (Bayer dithers).

### 0.1 Out of scope

| Not in phase 5 | When |
|---|---|
| Dragging dots, pinned positions (`pin.<pid>` lines are read and written back, never used) | v2 |
| Finger shapes in the map (PLAN G8.4) | not planned in v1 |
| Pen-button gestures | 6 |
| Hiding the map from Live view's screen stream | not planned |

---------------------------------------------------------------------------------------------------------------

## 1. Branch, base, and what must be true first

- Base: the finished phase 4 branch (`claude/rust-index-p4`) as the owner's session merged it (the map needs phase 2's
  final link data and phase 3's registry). Branch: `claude/rust-index-p5`. Never push to `main`, `claude/rust-port` or
  `claude/beta5-features`.
- Set up the machine with `docs/index-hook/CLOUD.md`.

### 1.1 Names this phase builds on (verify each first)

| Name | Where | What phase 5 needs |
|---|---|---|
| `NotebookInfo` (`page_ids`), `store_load_info` | store.rs:18, 878 | the model, the bgjob child |
| `Nav`, `NavLink` with phase 2's `area`, `nav_load`, `nav_set_name`, `nav_save`, `nav_section_of` | nav.rs:171, 371, 468; PHASE2-BRIEF 3.4 | the model, Rename |
| `pages_show`, `PagesView` (`index_tab`), `draw_index`, `PvResult` | editor.rs, pagesview.rs:34, 299 | the Graph button, the counts |
| the bar's `LbResult::Index` (opens the Pages view's Index tab) | editor.rs:3787 | now opens the map |
| `go_to_page`, the back list push rules | editor.rs; PHASE1-BRIEF 13.8 | open a page from the map |
| `bg_queue`, `BgKind` (`ImportFolder = 3`), `run_job`, the priority table, `bg_poll_kind` | bgjob.rs:13-304 | `BgKind::GraphLayout = 4` |
| `canvas_line_aa`, `canvas_lozenge`, `canvas_dash_poly` | canvas.rs:288, 218; PHASE2-BRIEF 6.4 | edges, badges, incoming edges |
| `font_draw_left_name`, the keyboard host (`app_keyboard_pointer_split`) | font.rs, app.rs | labels, Rename |
| `actions.rs` (Index group, `SRC_*`) | PHASE3-BRIEF 5 | `index-map` |
| `sheet_open`, `front_menu`, `editor_live_style` / `editor_pen_not_ink`, the menu family | editor.rs | the map is a menu; a pen drag is not ink |
| the corner: `status_square`, `page_in_front` | status.rs:380, 395 | the map is not the page: the corner is not live there (1.3) |

### 1.2 graph.rs from the pure branch: merge it, then finish it

Read `docs/index-hook/PURE-MODULES.md` on `claude/rust-index-pure-p4zqq5` first and take its newest commit if the graph
review is finished there; otherwise 7cbcf0f. Bring in only graph:

```
git checkout <pure commit> -- rust/src/graph.rs rust/tests/test_graph.rs rust/tests/graph_corpus
```

plus `pub mod graph;` and the prelude line by hand. The commit message names the source commit.

**The API as it is** (PLAN 11.3 assumed a surface; use these):

| Need | graph.rs |
|---|---|
| the model | `graph_build(&mut GraphModel, &NotebookInfo, &Nav, open_pid) -> i32`; `GraphModel { nodes, edges, force, tray, dmax, open, ncomp, laid }`; `GraphNode { pid, tier: GraphTier { Force, Satellite, Tray }, named, dup, linked, open, degree, links_to, linked_from, index_links, self_links, section, sat_rank, sats, comp, r, x, y, placed }`; `GraphEdge { a, b, dir: GRAPH_DIR_AB \| GRAPH_DIR_BA, mult }` |
| lookups | `graph_node_of(g, pid)`, `graph_neighbours(g, node)`, `graph_edge_between(g, u, v)`, `graph_sat_more(g, head)`, `graph_radius(d, dmax, named)` |
| the cache | `GraphIni { gen, has_gen, mode: GraphMode { All, Around }, show_unlinked, pos, pins, unknown, writable, file }`; `graph_ini_read(id, &mut ini)`, `graph_ini_write(id, &mut ini)`, the `_path` variants, `graph_ini_path` |
| the plan | `graph_apply_ini(&mut g, &ini) -> GraphPlan { Ready, Incremental, Full }`, `graph_layout_in_process(&g)` (at most `GRAPH_INPROC_MAX` = 300 force nodes) |
| layouts | `graph_seed(&mut g, pump)`, `graph_layout(&mut g, pump)` (pump every `GRAPH_PUMP_STEPS` = 2), `graph_place_new(&mut g, pump) -> i32`; `g.laid: GraphLaid { Unplaced, Seed, Done }` |
| writing | `graph_ini_store(&mut ini, &g) -> bool` (changed), then `graph_ini_write` |
| the child's work | `graph_layout_job(&mut g, &mut ini, path) -> i32` |
| Around | `graph_around(&g, centre, depth, &mut Vec<GraphRing>) -> i32`; `GraphRing { node, depth, parent, x, y }` |

**Open review items to close (commit 2, each with a test in `test_graph.rs`):**

1. *Incremental drift on maps of 600 or more nodes (high):* `graph_place_new` must leave every node outside the new
   pages' 2-hop neighbourhood bit for bit where it was (its own doc comment says so); a test on a 700-node corpus adds
   3 pages and compares every other position exactly (same host).
2. *A gen mismatch was not acted on:* with no new page but changed links, positions stay and `graph_ini_store` writes
   the new gen (the doc of `graph_apply_ini`); a test asserts the written gen.
3. *The open page could keep a stale seed position:* opening the map from a page that became a force node since the
   last visit places it by `graph_place_new`, never at a leftover seed.
4. *Satellites depended on which page was open:* d43da2e addresses it; a test opens the map from two pages and
   compares satellite positions exactly.
5. *PORTING rule 16:* `Vec::with_capacity`, `vec![..; n]` and pushes sized by the notebook use `try_reserve_exact`;
   `graph_build` returns -1 on failure (the map then says 'Not enough memory for the map.' and opens the List).
6. Update PURE-MODULES.md's graph row (and apply its proposed `BgKind::GraphLayout = 4`, section 6).

### 1.3 The clock corner does not apply over the map (brief decision)

Since the 2026-10-28 ruling the clock and battery show on the notebook page only, never on a sheet, a menu or the Pages
view, and its tap is live only there (status.rs `page_in_front`: the page in front with no sheet). The map is a
full-screen menu like the Pages view, so `sheet_open` covers it and the corner is neither drawn nor tapped there.
PLAN 11.5's "Back at x = z + 16" and 11.6's "the corner zone excepted" therefore do not apply: Back sits where the
Pages view puts its own, and a tap in the top-left corner over the map is the map's. `z_map_5_corner` asserts exactly
that (`settings.status_on` unchanged).

### 1.4 Pre-flight

`make rust-check` on the base; record the Linux-only failures.

---------------------------------------------------------------------------------------------------------------

## 2. Hard rules that apply to every commit

PHASE2-BRIEF.md section 2's rules 1 to 11 apply as written. Phase 5 adds:

12. No page loads to open the map: the model comes from `NotebookInfo`, `Nav` and links.ini only.
13. Never block ~100 ms: layouts pump (every 2 steps), drawing pumps every 64 edges; over 300 force nodes, or Tidy up,
    the layout runs in a bgjob child (fork without exec, niced, ending with `libc::_exit`, bgjob's rules; no thread).
14. Floats differ between arm64 and armv7 (fused multiply-adds): positions persist in graph.ini, tests assert
    invariants, never coordinates across hosts; app-side float code uses `fmla`, never `mul_add`.
15. The map canvas (screen size, about 4.6 MB) is allocated with `try_reserve_exact` at open and freed at close.
16. Everything drawn is 1-bit-safe, so a DU preview matches the GL16 settle and inversion keeps the marks distinct.
17. The `canvas_line_aa` rewrite is bit-identical: golden shots (the Gothic frieze included) must not move.

---------------------------------------------------------------------------------------------------------------

## 3. The view (`mapview.rs`, new; PLAN 11.1, 11.5)

```rust
pub struct MapView {
    pub open: bool,
    canvas: Canvas,                    // offscreen, the screen's size; compose blits it while open
    g: GraphModel, ini: GraphIni, id: FixedStr<STORE_ID_LEN>,
    zoom: f32, ox: f32, oy: f32,       // world to screen: sx = (x - ox) * zoom
    mode: GraphMode, around: Vec<GraphRing>, centre: i32,
    selected: i32,                     // node, -1
    layout: MapLayout,                 // Seed | Running (a bgjob child) | Done
    hits: Vec<MapHit>,                 // its own hit list: HitList holds 128 entries (ui.rs:842)
    labels: Vec<Rect>,                 // placed label boxes, for the greedy placement
    drag: Option<MapDrag>, last_input_ms: u32,
}
pub fn mapview_open(v: &mut MapView, a: &mut App, nb: &NotebookInfo, nav: &Nav, open_pid: u32, around: bool) -> bool;
pub fn mapview_close(v: &mut MapView);
pub fn mapview_draw(v: &mut MapView, a: &mut App, nav: &Nav, r: Rect);   // into v.canvas, culled to r
pub fn mapview_pointer(v: &mut MapView, a: &mut App, nav: &Nav, phase: i32, ev: &InEvent) -> MapResult;
pub fn mapview_pinch(v: &mut MapView, a: &mut App, ev: &InEvent);
pub fn mapview_tick(v: &mut MapView, a: &mut App, now: u32) -> MapResult; // idle swap-in of a finished layout
pub fn mapview_probe(v: &MapView, key: &str, out: &mut String) -> bool;
pub enum MapResult { None, Open(u32 /*pid*/), Close, Rename(u32), Pages, List }
```

Hosted by the editor like the Pages view (`EditorState.map: MapView`, taken out with `with_modal` for calls that need
the App; mapview never reaches back into the editor's state). It joins the menu family (`front_menu` returns
`SHORTCUT_INDEX_MAP` while it is open; its own action closes it; `pages` swaps to the Pages view; Back and
`close-menu` return to the page) and `sheet_open`. `Owner::Map` (numbered after the last owner of phase 2) takes its
pointer. A pen drag in the map is not ink: `editor_live_style` / `editor_pen_not_ink` cover `Owner::Map`.

**Screen** (upright; sideways the same with the tray on the right):
- Header, 190 px: Back (150 x 150) where the Pages view has its Back (1.3); the title 'Index' with a Graph | List
  switch (List: close the map and open the Pages view's Index tab); an 'All | Around' segment (2 x 300); Fit (existing
  msgid).
- Bottom row, 170 px: - and + (220 x 140); 'Unlinked: Show' / 'Unlinked: Hide'; 'Tidy up', shown once positions exist.
- Selecting a dot replaces the bottom row with a 320 px card: the name at 64 px (or 'Page %d'); 'Page %d. Links to %d,
  linked from %d.' (from `links_to`, `linked_from`); Open (existing msgid), Around it, Rename (existing msgid).
- The tray: 'Not linked yet (%d)' below the graph upright, at the right sideways, 40 px pitch, notebook order; more
  than 200 collapse to 'Not linked yet: %d pages. Show' (a tap expands; the expanded tray scrolls with the map's pan).
  Hidden with Unlinked: Hide, as are the satellites.

---------------------------------------------------------------------------------------------------------------

## 4. Opening, layout and the cache (PLAN 11.2, 11.3)

At `mapview_open`:

1. Allocate the canvas (`try_reserve_exact`); failure: toast 'Not enough memory for the map.' and open the List.
2. `graph_build(&mut g, nb, nav, open_pid)`; -1: the same refusal.
3. `graph_ini_read(id, &mut ini)` (a refused file: ignored and never overwritten this session, `ini.writable` false).
   `mode` and `show_unlinked` come from it (remembered per notebook; PLAN 11.6).
4. `match graph_apply_ini(&mut g, &ini)`:
   - `Ready`: `graph_place_new` if the open page needs a place (1.2 item 3);
   - `Incremental`: `graph_place_new(&mut g, &mut work_pump)`;
   - `Full`: if `graph_layout_in_process(&g)`, `graph_layout(&mut g, &mut work_pump)` (about 0.25 s on the Kindle for
     300 nodes); else `graph_seed(&mut g, &mut work_pump)`, `layout = Running`, and `bg_queue(BgKind::GraphLayout,
     Some(id), Some(graph.ini path), dpi, false)`.
   Then `graph_ini_store(&mut ini, &g)` and, when it changed and `ini.writable`, `graph_ini_write` (atomic).
5. The default view: All, centred on the open page at a zoom that fits its 2-step neighbourhood (`graph_neighbours`
   twice), or everything when every dot would still be at least 10 px; Around when `ini.mode` says so (centred on the
   open page).
6. Draw into the canvas, compose, `Full` (GC16; PLAN 14.3: a deliberate screen change).

**The finished background layout:** `mapview_tick` sees the job done (`bg_poll_kind` reports `GraphLayout`; the editor
forwards it), and only while the map is open and idle (no input for 500 ms) re-reads graph.ini, `graph_apply_ini`,
redraws and presents one `Ui`. If the map was closed meanwhile, nothing happens: the next open reads the new file.

**Tidy up:** a full layout whatever the plan: in-process (pumped) up to 300 force nodes, else the bgjob child; the
button greys while `Running`.

**Around:** `graph_around(&g, centre, 2, &mut around)`: only members are drawn, centred on the centre page; instant and
deterministic. Switching All/Around writes `mode` to graph.ini (when writable).

---------------------------------------------------------------------------------------------------------------

## 5. Drawing (PLAN 11.4, 11.7)

Order per frame, culled to the viewport, `work_pump` every 64 edges: edges, knockouts, dots, labels, badges.

- **Edges:** `canvas_line_aa` 2.5 px, plus 1 px per extra link (`mult - 1`) up to 5 px; with a selection, its edges at 5
  px, outgoing solid and incoming dashed (`canvas_dash_poly`); each under white knockout discs of radius r + 4
  (`GRAPH_KNOCKOUT`).
- **Dots** (radius `graph_radius(degree, dmax, named)` at fit, times the zoom; satellites `GRAPH_R_SAT` 6 px): degree 0
  a hollow 3 px ring; degree 1 a 25% Bayer dither inside a ring; 2-3 a 50% dither inside a ring; 4 or more solid
  (`canvas_disc_pattern(c, cx, cy, r, level)`, level 0..4 meaning 0, 25, 50, 100 %). A named orphan is hollow and
  labelled. The open page: a concentric double ring 6 px apart. The selected: an 8 px ring and its label as an
  inverted chip.
- **Index badge:** `index_links > 0`: a 16 px lozenge (`canvas_lozenge`) at the dot's upper right (links to The Index
  are never edges).
- **Labels:** named pages in bold 34 px, at most 320 px wide (`font_draw_left_name`), on a white knockout box; greedy
  placement in this order: the selected, the open page, the selection's neighbours, then named pages by degree; 4
  candidate spots each (right, left, above, below); a colliding label is skipped until zoomed; 'p. %d' (existing) for
  unnamed pages only at 2x and over, or in Around.
- **Satellites** over `GRAPH_SAT_DRAWN` (60) per section: 60 drawn plus a '+%d' label (`graph_sat_more`), all of them
  at 2x and over.
- **Cost:** about 50-70 ms for 100 nodes; the worst case (2000 force nodes, 4000 edges at fit) 1-2 s, pumped, shown
  behind the seed frame.

### 5.1 Canvas primitives (`canvas.rs`; PLAN 14.5)

- `canvas_line_aa` (canvas.rs:288) becomes row-span: per row, the covered span computed once and filled, instead of a
  per-pixel distance test over the bounding box; bit-identical to the old loop (a test over 300 random edges and
  widths compares pixel for pixel with the old function kept under `#[cfg(test)]`), about 40x faster on long edges.
- `canvas_disc_pattern(c, cx, cy, r, level)`: a filled disc whose interior is a 4x4 Bayer pattern on 2x2-pixel cells,
  gray 0 and 255 only; coverage within 1/16 of each level.

---------------------------------------------------------------------------------------------------------------

## 6. The background layout (`bgjob.rs`)

- `BgKind::GraphLayout = 4` (the pure branch's proposal): `id` the notebook, `path` its graph.ini. `run_job`: in the
  child, `store_load_info`, `nav_load`, `graph_build` (open pid 0: the open page is not kept), `graph_ini_read_path`
  (to keep mode, unlinked, pins and carried lines, and to honour a refused file), `graph_layout_job(&mut g, &mut ini,
  path)`; its return is the job's result. Niced as the other kinds; fork without exec; `libc::_exit`.
- Priority: after the jobs the user waits on (notify), before mirror PDFs, imports and compaction (PLAN 18: a PDF
  mirror must not hold up the map), in the priority table at bgjob.rs:304. A queued duplicate (same notebook) merges.
- `bg_cancel_notebook` cancels it with the notebook's other jobs (deleting a notebook).

---------------------------------------------------------------------------------------------------------------

## 7. Interaction (PLAN 11.6)

- **Tap a dot:** the nearest dot within max(r x zoom, 59 px = 5 mm) by the view's own hit test (satellites included,
  the tray's entries too); `Ui` of the old and new selection rects plus the card.
- **Tap the selected dot again, or Open:** `go_to_page` (GC16), close the map, push the origin on the back list (as a
  bar commit does: the bar's back list and `go-back` return).
- **Tap empty space:** deselect (`Ui` of the old selection and the bottom row).
- **One-finger or pen drag:** pan; an offset blit of the canvas by `Fast` at most every 180 ms of `t_ms`, then at the
  release a pumped re-render and one `Ui`. A pen drag raises feature A's capture-blocked flag (`Owner::Map`).
- **Pinch:** `editor_event` routes `PinchBegin/Move/End` to `mapview_pinch` while the map is open (unlike the Pages
  view). Zoom 0.25x-6x around the midpoint; a nearest-neighbour scaled `Fast` preview at most every 180 ms, then a
  re-render and `Ui` at PinchEnd. `- / +` step 1.5x; Fit resets to the default view. The touch context while the map
  is open: `scrolls_v = scrolls_h = false` (the map pans itself; a two-finger translate stays a swipe candidate, so
  `pages` swaps and the mirror rule still work), `zoom_ok = settings.pinch_zoom`.
- **Two-finger gestures:** as any menu: `pages` swaps to the Pages view, `links` swaps to the bar (committing nothing:
  the page did not change), `index-map` (its own) closes it.
- **Rename (D3):** the keyboard host over the map, 'Name in the Index' (phase 2's msgid), prefilled; Done: `nav_set_name`
  + `nav_save`; the label and the model update (`graph_build` again keeps positions: names change no position, `gen`
  ignores names); toast 'Page %d is in the Index as %s.' (phase 2's).
- **Around it:** `mode = Around`, `centre` = the selection.
- **Unlinked: Show / Hide:** toggles satellites and the tray; writes `show_unlinked`.
- **Back, close-menu, the action again:** close; GC16.

---------------------------------------------------------------------------------------------------------------

## 8. Entries and the extras (PLAN 11.1, 11.8)

- **The bar's Index button** (`LbResult::Index`, editor.rs:3787): commit a peek as today, drop the bar, open the map
  (centred on the shown page) instead of the Pages view's Index tab.
- **The Pages view's Index tab**: a Graph button in its header (`PvResult::Graph`); the editor closes the Pages view and
  opens the map. Each Index row gains a small '%d links here' count (links from any page to that entry's page; the
  count comes from one pass over `Nav` when the tab draws, not per row).
- **The action** `index-map` (MENU, Index group, now 8 rows: links, index-map, index, go-to-page, go-back, add-link,
  name-page, link-areas; label 'Index map'; sources KEY | PAD | TOUCH | SHAPE). `index` keeps its meaning (the list);
  a link to The Index still opens the list.
- **The bar's chip** '%d pages link here' (`linked_from` of the open page, computed from `Nav` when the bar opens;
  absent at 0): in the bar's header row; a tap opens the map in Around this page.

---------------------------------------------------------------------------------------------------------------

## 9. E-ink refresh choices (no new display call, no new wait)

| Event | Waveform |
|---|---|
| Open, close, Open a page, List, swap to Pages | Full (GC16) |
| Select, deselect | Ui of the small rects and the card |
| Pan, zoom while moving | Fast at most every 180 ms of `t_ms` |
| Pan or zoom released | Ui after a pumped re-render |
| Background layout swapped in (open and idle 500 ms) | one Ui |
| All / Around, Unlinked, Tidy up done in-process | Ui of the graph area |
| Rename typing | the keyboard host |

---------------------------------------------------------------------------------------------------------------

## 10. The runner and docs/SIM.md

No new commands (`waitbg` exists). Probes (`mapview_probe` through `editor_probe`): `editor.map` (`0` or `graph`),
`editor.map.nodes`, `.edges`, `.force`, `.satellites`, `.tray`, `.orphans` (degree 0 force nodes), `.selected` (page
number or empty), `.mode` (`all`, `around`), `.layout` (`seed`, `running`, `done`), `.node.PID` (`tier r x y visible`,
in screen px rounded, for invariants on one host only), `.labels` (how many drawn), `.zoom`, `.ui_at.X.Y` (`back`,
`graph`, `list`, `all`, `around`, `fit`, `minus`, `plus`, `unlinked`, `tidy`, `card.open`, `card.around`,
`card.rename`, `node.PID`, `tray.PID`, empty). SIM.md documents them, the header and bottom-row coordinates, and that
scenarios assert invariants. Selftest: `pass-rust.qs` `expect editor.map is 0`; `fail-rust.qs` `expect editor.map is
graph`.

---------------------------------------------------------------------------------------------------------------

## 11. Unit tests

- `test_graph.rs`: the items of 1.2.
- canvas: the row-span line equals the old loop pixel for pixel over 300 random edges and widths; `canvas_disc_pattern`
  coverage within 1/16 per level and 1-bit; a host timing guard (under 0.05 ms per long edge in release, skipped in
  debug); golden shots unchanged.
- `rust/tests/test_mapview.rs` (pure parts): the default zoom rule; label placement never overlaps and keeps the
  selected and the open page; the hit test picks the nearest within its radius; the tray layout for 10 and 250 pages
  upright and sideways.
- bgjob: the GraphLayout job in a child writes graph.ini atomically and a refused file is not overwritten (a test with
  the exporter's pattern for child jobs).
- `test_i18n.rs`: the strings of section 13 fit.

---------------------------------------------------------------------------------------------------------------

## 12. Scenarios

Every new file starts with `# rust-only`. Coordinates of dots differ between hosts: scenarios find a dot through
`editor.map.node.PID` (its x y on this host) and tap there, never at a recorded coordinate.

- **`z_map_0_make`** (a root): a 40-page notebook with 30 links among pages 1-25, 6 named pages, pages 26-40 unnamed and
  unlinked, two pages before the first name unnamed and unlinked (the tray), made with phase 2's ink-free making and
  key fixtures (`tests/fixtures/index-keys`).
- **`z_map_1`** (`# root: z_map_0_make`, `# env: GRIMOIRE_CHECK_FIT=1`): open from the bar's Index button: `editor.map
  is graph`, `editor.map.nodes is 40`, `.satellites`, `.tray is 2`, `.layout is done` (in-process), one `full`; shots
  upright and sideways; tap a node: `.selected`, one `ui`; tap it again: `editor.page` changes, `editor.map is 0`,
  `editor.back` grew by 1; `go-back` returns. Reopen: positions equal (`.node.PID` the same as before, same host),
  graph.ini exists with `gen`.
- **`z_map_1b_es`**: the same screens in Spanish, both styles, CHECK_FIT.
- **`z_map_2_around`**: Around it on a node with depth-2 neighbours: `.mode is around`, only members visible (`.node.PID`
  of a non-member reads `... 0`); reopening keeps Around (graph.ini `mode=around`).
- **`z_map_3_pan_zoom`**: a one-finger drag: `fast` presents then one `ui`; a legacy `pinch` in the map: `.zoom` changes,
  `fast` then `ui`; `+` and `-` step 1.5x; Fit resets; a pen drag: `editor.strokes` unchanged and nothing on
  `/live/pen` (as `y_live_ink` checks).
- **`z_map_4_big`**: a generated 2000-page notebook with 4000 links (generated by the scenario's root from a fixture
  script, not committed as pages): open: `.layout is seed`, `main.longest_unpumped_ms is ok`; `waitbg`, `wait 600`
  (idle): `.layout is done`, one `ui` for the swap; graph.ini written by the child.
- **`z_map_5_corner`**: with the clock on, a finger tap in the top-left corner over the map does what the map does
  there (Back: `editor.map is 0`) and `settings.status_on` is unchanged (1.3).
- **`z_map_6_rename`**: Rename an unnamed node: 'Add page %d to the Index...' reads as on the card; Done: the label
  shows, `editor.index_rows` +1, the toast; positions unchanged.
- **`z_map_7_entries`**: the Pages view's Graph button opens it; the `index-map` key opens and closes it; the bar's
  '%d pages link here' chip opens it in Around this page; the Index tab shows '%d links here' counts (a shot).
- **`z_map_8_nomem`**: with the canvas allocation failed through a sim hook `GRIMOIRE_SIM_MAP_NOMEM=1` (scenarios rule
  unset list, `SIM_HOOKS`): the toast and the List opens.
- `y_all_screens`: the map (graph, a selection card, Around, the tray collapsed), the Index tab's Graph button and
  counts, the bar's chip; EN and ES, both styles, upright and sideways, CHECK_FIT.
- Changed on purpose: scenarios where the bar's Index button opened the Pages view's Index tab (`z_bar_2c_index` and any
  other found by grepping for the Index button's tap or `LbResult::Index`'s probe): they now expect the map (or go
  through the map's List switch).
- Stay green: every earlier scenario; every golden shot (the line rewrite is bit-identical).

---------------------------------------------------------------------------------------------------------------

## 13. UI strings (English / Spanish)

| Where | English | Spanish |
|---|---|---|
| title | Index | Índice |
| switch | Graph / List | Grafo / Lista |
| segment | All / Around | Todas / Alrededor |
| bottom row | Unlinked: Show / Unlinked: Hide | Sin enlaces: Mostrar / Sin enlaces: Ocultar |
| bottom row | Tidy up | Ordenar |
| card line | Page %d. Links to %d, linked from %d. | Página %d. Enlaza a %d, enlazada desde %d. |
| card button | Around it | Alrededor |
| tray | Not linked yet (%d) | Aún sin enlaces (%d) |
| tray (`ng_`) | Not linked yet: %d page. Show / Not linked yet: %d pages. Show | Aún sin enlaces: %d página. Mostrar / Aún sin enlaces: %d páginas. Mostrar |
| satellites | +%d | +%d |
| refusal | Not enough memory for the map. | No hay memoria para el mapa. |
| action label | Index map | Mapa del índice |
| Pages view button | Graph | Grafo |
| Index tab count (`ng_`) | %d link here / %d links here | %d enlace aquí / %d enlaces aquí |
| bar chip (`ng_`) | %d page links here / %d pages link here | %d página enlaza aquí / %d páginas enlazan aquí |

Existing msgids used: 'Fit', 'Open', 'Rename', 'Back', 'Page %d', 'p. %d', 'Name in the Index', 'Add page %d to the
Index...', 'Page %d is in the Index as %s.'. Budgets in TRANSLATORS lines: header controls about 280 px at 34 px caps;
card line about 1500 px at 44 px; tray line about 1700 px at 34 px.

---------------------------------------------------------------------------------------------------------------

## 14. Docs, commit plan, the gate, owner acceptance

**Docs.** FORMAT.md: graph.ini byte for byte (header, `gen` FNV-1a 32 of the sorted force-node and edge sets, `mode`,
`unlinked`, `pos.<pid>=x,y` integer world units, `pin.<pid>` reserved, carried lines 255 B each and 16 KiB in all, the
256 KiB cap, a refused file never overwritten), "a cache: deleting it loses nothing", raw ZIP exports leave it out,
older versions never read it; SHORTCUTS.md `index-map`; DESIGN.md the map's screen, look and refresh table; SIM.md
section 10; KINDLE.md and TESTING.md the owner checks; PURE-MODULES.md graph merged.

**Commits** (each ends with CLOUD.md's attribution lines and leaves `make rust-check` green):

1. `graph: from claude/rust-index-pure-p4zqq5 (<commit>)` (1.2).
2. `graph: the review items closed; allocations with a fallback` (1.2 items 1-6).
3. `canvas: canvas_line_aa row-span (bit-identical), canvas_disc_pattern` (5.1).
4. `bgjob: GraphLayout, big and Tidy-up layouts in the child` (6).
5. `mapview.rs: the Index map (model, cache, layout policy, drawing)` (3, 4, 5, `test_mapview.rs`).
6. `editor: host the map (Owner::Map, the menu family, pinches, the bar's Index button, the index-map action)` (7,
   8's bar button and action, `z_map_0`, `_1`, `_2`, `_3`, `_5`, `_8`, l10n). Push.
7. `mapview: Rename, Unlinked, Tidy up, the background swap-in` (7, 4, `z_map_4_big`, `z_map_6_rename`).
8. `pagesview, linkbar: the Graph button, '%d links here', '%d pages link here'` (8, `z_map_7_entries`, l10n).
9. `y_all_screens and l10n-check: the map`; `docs: phase 5`. Push.

**The gate** is PHASE2-BRIEF 16.2's. graph.rs's layout floats differ between hosts: its tests assert invariants; a
Linux-only difference is reported, never edited.

**Owner acceptance on the Kindle:**

1. Open the map from the Links bar's Index button, or the Index tab's Graph button. Every page is a dot: well-linked
   pages solid, lightly linked ones dithered, unlinked pages hollow, small dots around their section's named page or in
   the 'Not linked yet' tray; the open page has a double ring; named pages are labelled.
2. Tap a dot for its card ('Page 12. Links to 3, linked from 2.'); tap it again to go there; Back (go-back) returns.
3. Drag with a finger or the pen to pan and pinch to zoom: rough previews while moving, crisp when you let go. Around
   it shows one or two links deep. Unlinked: Hide clears the small dots.
4. Your biggest notebook opens quickly on a rough placement and tidies itself once the layout finishes, without pen
   lag; the next time it opens as you left it.
5. Rename a page from its card: the Index has the new name.
6. The bar's 'N pages link here' chip opens the map around the page; Index tab rows show how many links point at each
   entry.

---------------------------------------------------------------------------------------------------------------

## 15. Brief decisions (fold into PLAN-V2 section 19)

1. graph.rs's real API (section 1.2's table) replaces PLAN 11.3's assumed surface; graph.ini's codec is graph.rs's.
2. The clock corner is not live over the map (status.rs `page_in_front`, the 2026-10-28 ruling): Back sits where the
   Pages view's does; PLAN 11.5's x = z + 16 and 11.6's corner exception are dropped; `z_map_5_corner` asserts the
   corner does nothing there.
3. `BgKind::GraphLayout = 4`, prioritised after notify jobs and before mirror PDFs.
4. The map keeps its own hit list (`HitList`'s 128 entries are too few).
5. With the map open the touch context says nothing scrolls (the map pans itself), so two-finger swipes act as menu
   gestures there; pinches are routed to the map.
6. A sim hook `GRIMOIRE_SIM_MAP_NOMEM=1` tests the out-of-memory path.
7. '%d links here', '%d pages link here' and the tray's collapsed line use `ng_` plurals.
8. Opening a page from the map pushes the origin on the bar's back list, as a bar commit does.
