# Phase 2 implementation brief: ink-free links and the long-press card (Rust)

This brief is self-contained: with it and the repository you can build phase 2. It applies
`docs/index-hook/PLAN-V2.md` (the authority; section numbers below, such as 9.3 or 13.1, are its sections), the owner's
rulings in `docs/index-hook/README.md` (R1, R2, D1, D3 are the ones phase 2 serves), `docs/index-hook/CLOUD.md` and
the contract `rust/PORTING.md`. Its style follows `docs/index-hook/PHASE1-BRIEF.md`. Where this brief is more precise
than the plan it says so in place ("brief decision"). Section 18 lists every brief decision so the owner's session
can fold them into PLAN-V2 section 19, as was done for phase 1. Until then, a marked brief decision wins for phase 2
code and anything else follows the plan; raise a disagreement with the owner's session rather than guessing.

All paths are relative to the repository root. Rust line numbers are at `claude/rust-index-p2-brief` (f6a5328: phase 1
up to its commit 14, the Links bar, Find and the eye not yet in). They drift, and phase 1's commits 15-21 move them
further, so search by the function name. Phase 1's later names (the bar, Find, the box cache) are cited from
PHASE1-BRIEF.md sections 12-15, where they were designed; verify each against the finished phase 1 code first
(section 1.1).

The owner's words for this phase, verbatim:

> if users want visible borders, they can draw them with a new smart tool system

> add a dotted border that shows link bounds when long pressing on a link, and options in that dialog to delete
> link or rename while still seeing the notepad

> long press to change name or delete any link

---------------------------------------------------------------------------------------------------------------

## 0. What phase 2 is

Phase 2 is one Kindle build that the owner installs and tests. It contains:

1. **links.ini v2** (R1, PLAN 13.1): an `area.` line beside each link's unchanged `link.` line. A link is
   (page id, seed, target, area). The area is invisible: never in the page, thumbnails, PDF page image, share page or
   sleep image.
2. **Area-first boxes and smallest wins** (PLAN 9.2, 6.5): a link's box is its area, or for a link with no area
   the box of its ink (the beta.5 rule). The smallest box under the finger wins.
3. **Migration of the owner's beta.5 links** (R1, PLAN 9.8): each inked link's stroke box becomes its area, at page
   load and on idle ticks. Its ink stays on the page as ordinary ink.
4. **Link edits in undo** (PLAN 9.7): making, deleting, retargeting and redrawing a link are one undo step each,
   and a ×N group from the bar is one step.
5. **Making a link without ink** (R1, PLAN 6.4, 9.3): the link-mode strip, a dashed lasso that is never ink, a
   dotted rectangle until the pick, and **Link to...** (the bar's frame in pick mode) with New page..., The Index,
   Recent targets, the Index entries, Search and Choose from the pages....
6. **Find PICK mode** (PLAN 8.1, 8.2): Link to...'s Search, over the page, with the keyboard docked away from the
   area.
7. **The long-press card** (R2, PLAN 6.3, 9.5, 9.6, `linkcard.rs`): hold a link on the page and a heavy dotted border
   shows its bounds, then a small card beside it offers Rename in the Index (or Add page N to the Index..., D3),
   Change where it goes..., Redraw the area and Delete link. The page stays in view throughout.
8. **The bar's long press** (R2, D1, PLAN 7.9): holding a bar row opens the same card. A ×N row renames the entry,
   or retargets or deletes all its N links on this page in one undoable step.
9. **The Pages-view hold path retired** (PLAN 9.6): `link_hold`'s picker, `PG_UNLINK`, `PG_TO_INDEX` and `pick_had`.
10. **Copy page keeps links** (PLAN 9.9).
11. **The first-link tip** (`hints` bit 1).

### 0.1 Explicitly out of scope (later phases; do not build them now)

| Not in phase 2 | Phase |
|---|---|
| Settings > Gestures, recording, finger shapes, gestures.ini writes, `actions.rs`, every-toggle assignment, three-finger gestures, the hover rule relaxed | 3 |
| Smart shapes (a clean visible border), `shape.rs`, the Shapes panel button, `canvas_slash`, `ActKind::Replace` | 4 |
| The Index map, the bar's Index map chip, `%d pages link here`, the map's Rename | 5 |
| Pen-button gestures | 6 |
| Polygon or freehand areas (an area is an axis-aligned rectangle) | not planned |
| Hiding the card, strip or lasso from Live view's screen stream (PLAN 20, open question 3) | not planned |

Known in this build (say so in the owner notes):
- A visible border around a link is freehand ink until phase 4 brings Smart shapes.
- Clear page clears ink only: the page's links stay (they are not ink). Delete them from the card or the bar.
- Rename is not in undo (it changes the notebook's Index, not the page; PLAN 19).
- Creating a page with New page... is not undone by Undo: Undo removes the link, the page stays.

---------------------------------------------------------------------------------------------------------------

## 1. Branch, base, and what must be true first

- Base: the finished phase 1 branch `claude/rust-index-p1` (all of PHASE1-BRIEF.md's commits 1-21, reviewed by the
  owner's session). Branch: `claude/rust-index-p2` (or the name your task gives). Never push to `main`,
  `claude/rust-port` or `claude/beta5-features`.
- Set up the machine with `docs/index-hook/CLOUD.md`.

### 1.1 Phase 1 names this phase builds on (verify each before writing code)

PHASE1-BRIEF.md sections 12-15 designed these. Find each in the merged code (`grep -n`), note any difference, and
adapt only the call sites named in this brief. If one is missing, stop and ask the owner's session.

| Phase 1 name | Where | What phase 2 needs from it |
|---|---|---|
| `LinkBoxes`, `boxes_get`, `EditorState.boxes`, its `stale` flag | editor.rs (PHASE1 12) | marked stale by link edits and migration; its entry numbers give a bar row's seeds |
| `canvas_dash_rect(c, r, thick, on, off, halo)` | canvas.rs (PHASE1 12) | the heavy border and the dotted rectangle; the exact band it paints (for the strips, 8.2) |
| `LinkBar`, `LbEntry`, `LbResult`, `linkbar_build`, `linkbar_layout`, `linkbar_draw`, `linkbar_pointer`, `linkbar_probe` | linkbar.rs (PHASE1 13.1) | gains pick mode (Link to...) and the row hold |
| `Owner::Bar = 11`, `Owner::Outside = 12` | editor.rs (PHASE1 13.5) | `Owner::Outside` is reused by the card and Link to...; new owners are numbered after them |
| the peek state, `bar_commit`, the peek cancel, the Back chip's peek clear | editor.rs (PHASE1 13.6, 13.7) | New link commits first; the row hold cancels a peek to the origin without closing the bar |
| phase 1's silent cancel of a chip press held past 600 ms | editor.rs or linkbar.rs (PHASE1 13.5) | replaced by the card (8.10) |
| `FindSheet`, `FsResult`, `findsheet_open`, `findsheet_update`, `findsheet_draw`, `findsheet_pointer` | findsheet.rs (PHASE1 14) | gains PICK mode |
| `app_keyboard_pointer_split` | app.rs (PHASE1 14) | Find PICK types through it as GO does |
| `PvResult::GoTo` and how `PG_JUMP` behaves while `pv.picking` | pagesview.rs (PHASE1 14) | Choose from the pages... still picks in the Pages view (9.1) |
| `editor_pen_not_ink` or the extension of `editor_live_style` | editor.rs (PHASE1 1.1) | extended for the lasso, the card, Link to... and the strip (2, rule 13) |
| `P_LINKS`, `IconKind::LinkPlus`, `IconKind::Bubbles` | editor.rs, ui.rs (PHASE1 13.3, 15) | unchanged |
| `store_load_page_pumped` | store.rs (PHASE1 13.9) | the idle migration's reader |
| `z_bar_0_make` and the scenarios that start from it | tests/scenarios (PHASE1 20.1) | converted to ink-free making (13.2) |

### 1.2 The feature contracts as phase 1 left them

- **Feature A (Live view fast ink).** In this tree the capture-blocked rule is derived from `editor_live_style`
  (editor.rs:2234): it returns with `enabled = 0` when `sheet_open(s)`, `s.link_mode`, or an owner other than
  None, Draw and Erase. Phase 2 keeps that shape (rule 13 of section 2).
- **Feature B (the clock corner).** `corner_zone(a)` = `status_zone(a)` (editor.rs:3723) and `corner_tap(a, x0, y0,
  moved, cancelled)` (editor.rs:3732), decided at the lift. A finger down longer than 600 ms is never a corner tap
  (`TAP_MS`, status.rs:26), so a hold that opens the card never toggles the clock. Its probe is
  `settings.status_on`. With the indicator drawn the zone grows (`status_zone` unions it), so every placement below
  uses `corner_zone(a)`, never a fixed 142 px.

### 1.3 The beta.5 links fixture

Migration is a one-way rewrite of the owner's links.ini (PLAN 18), so it is tested on files written by the inked
flow, byte for byte as the device writes them. Phase 2 removes that flow, so the fixture is made once, before the
removal:

1. In a separate worktree of the finished `claude/rust-index-p1`, build the headless runner.
2. Run the generator `tests/fixtures/beta5-links/make.qs` (committed for the record, not in tests/scenarios; it
   uses only phase 1 commands) as the scenarios rule runs a file: `GRIMOIRE_ROOT=<dir> GRIMOIRE_SCRIPT=... GRIMOIRE_LANG=en`.
   It makes one notebook of 4 pages, page 3 named 'Árbol', and on page 1:
   - link A: `pen line 300 1000 900 1200 30 0.6 0.6`, to page 3;
   - link B: `pen line 300 1400 900 1500 30 0.6 0.6`, to page 2;
   - link C: `pen line 1000 1000 1500 1200 30 0.6 0.6`, to the Index, then the ring covered whole by the precise
     eraser (its stroke stays, hidden);
   - link D: a ring linked to page 4, then its ring undone (a dormant link: no ink);
   and on page 4 (never opened by the migration scenario) a big ring linked to page 2 drawn AFTER a small ring
   inside it linked to page 3 (beta.5 follows the big one there; smallest wins follows the small one).
3. Copy `<dir>/notebooks/<id>` into `tests/fixtures/beta5-links/notebooks/<id>` with its `links.ini`, `nav.ini`,
   `notebook.ini` and page files. Add `tests/fixtures/beta5-links/README.md` saying how it was made and from which
   commit. The phase 1 links.ini bytes equal the C beta.5's (PORTING rule 1), so the fixture is what f9d13f5 writes.

A copy of a real device notebook is better still. Only the owner's session may provide one (no device access, ever,
CLOUD.md). If it does, add it as `tests/fixtures/beta5-device/` with its own scenario, `z_link_migrate_device`.

### 1.4 Pre-flight

- Run `make rust-check` on the base before any change, and record which checks fail on this Linux machine
  (section 16.2).

---------------------------------------------------------------------------------------------------------------

## 2. Hard rules that apply to every commit

1. No em dashes (U+2014) in code, comments, docs, UI or commit messages. Check with
   `grep -rn $'\u2014' rust/src docs po tests` before each commit.
2. Every UI string in English and Spanish, in the same commit: `tr` / `n_` / `c_` / `nc_` / `ng_` with a
   `// TRANSLATORS:` line directly above carrying the width budget; formats through `cformat!` / `csnprintf!`;
   po/es.po edited, `make l10n` where it works (PORTING "Packaging" 4: do not commit its output wholesale), then
   `msgfmt --check --no-hash --endianness=little -o po/es.mo po/es.po`; .po and .mo committed together; every new
   surface visited in `y_all_screens` upright and sideways in both styles under `GRIMOIRE_CHECK_FIT`; a changed
   meaning gets a new msgid. Retired msgids (section 14.3) stay in po/es.po until `make l10n` marks them obsolete:
   never hand-delete a translation.
3. Never change the `.qpg` format: no header version bump, no tool id >= 10, no ext flag, no chunk holding page
   coordinates. Areas live in links.ini only.
4. Never add an `MXCFB_WAIT_FOR_UPDATE_SUBMISSION` wait or a second wait on one display marker. Add no display calls:
   screens use `app_invalidate` (app.rs:1506) and `app_flush` (app.rs:1523); `app_press_feedback` (app.rs:1617) is the
   one immediate DU; a separate `app_flush` only where this brief states one.
5. The main loop never blocks ~100 ms. The idle migration reads through `store_load_page_pumped` and migrates one page
   per idle tick; `nav_link_migrate` makes one pass over a page's strokes (3.6).
6. Pen mode tolerates only DU: no GL16 and no toast while the pen is down. The lasso never turns pen mode on
   (`display_pen_mode`) and never uses the live ink style (`display_live_ink`): it is not ink.
7. PORTING rule 1, byte-exact: a links.ini whose links have no area is written byte for byte as beta.5 writes it
   (header included, brief decision 3.1); every existing qlog line unchanged, new lines with `qlog!` and printf
   formats; every existing `app_probe` key and value unchanged except those listed in section 11.3 as deliberately
   changed; existing runner commands keep their event sequences. PORTING rule 2 governs everything else; the
   deliberate changes are listed in section 13.4.
8. links.ini is written whole and atomically (`write_file_atomic`, util.rs:350), and never when it did not read
   (`links_writable` false). Migration never removes or changes ink.
9. Timing: `now_ms` and `t_ms` are wrapping u32 compared with `wrapping_sub`. The card's and the bar's holds are
   timed in `editor_tick` (like `LINK_HOLD_MS`, editor.rs:22), the idle migration only when `a.input_drained`.
10. Global state: short borrows; never call `work_pump()` or another module while holding a `thread_local` borrow;
    no global App pointer. The card, Link to... and Find are taken out of `EditorState` for a call that also needs the
    App (`with_modal`, editor.rs:337), and their own functions never reach back into the editor's state.
11. Memory: data-sized allocations (the Index entries of Link to..., the lasso, link records) use
    `try_reserve_exact` with a stated fallback.
12. The clock corner's one-finger TAP is never claimed by anything this phase adds: every swallowed tap outside the
    card, Link to..., Find PICK and the strip asks `corner_tap` first, and no new surface meets `corner_zone(a)`.
13. A pen contact that will not become ink raises feature A's capture-blocked flag: the lasso (`Owner::Lasso`), pen
    taps on the strip (`Owner::Strip`), the card (`Owner::Card`), Link to... (`Owner::LinkTo`) and their swallowed
    pen outside (`Owner::Outside`). Replace `s.link_mode` in `editor_live_style` by `s.lm.state != LmState::Off`,
    and make `sheet_open` cover the card and Link to... (6.1, 8.5). If phase 1 added `editor_pen_not_ink`, extend it
    the same way.
14. The pen always wins over a finger, as in phase 1; while making a link the next pen contact on the page is the
    lasso, whatever the tool (eraser end, side button and Erase while held included).
15. A card flow never shows the Pages view (R2). Only New link's Link to... offers Choose from the pages....

---------------------------------------------------------------------------------------------------------------

## 3. Data and format: links.ini v2 (`nav.rs`, docs/FORMAT.md)

### 3.1 Grammar

```
# Grimoire: links drawn on pages
link.12=0a1b2c3d,p7
area.12=0a1b2c3d,410,1220,380,140
link.12=0b00c0de,index
link.15=00000042,p12
area.15=00000042,96,300,1500,212
```

- `link.<pid>=<seed>,<target>`: today's grammar and parser, unchanged (`link_parse`, nav.rs:310).
- `area.<pid>=<seed>,<x>,<y>,<w>,<h>`: the link's area, an axis-aligned rectangle in page px at the notebook's page
  size (`store_notebook_page_size`, store.rs:705), the same coordinates as the page's strokes.
  - `<pid>`: as in a `link.` line (decimal, 1 to 4294967295).
  - `<seed>`: 1 to 8 hex digits, either case, as in a `link.` line.
  - `<x>`, `<y>`, `<w>`, `<h>`: 1 to 4 decimal digits each, nothing else (no sign, no space);
    0 <= x, 0 <= y, 1 <= w, 1 <= h, x + w <= 6000, y + h <= 6000 (`NAV_AREA_MAX = 6000`, the exporter's page bound,
    export.rs:154).
- **Writer.** For each link kept (`link_kept`, nav.rs:733), in (pid, seed) order: its `link.` line, then, when it has
  an area, its `area.` line, `area.%u=%08x,%d,%d,%d,%d\n`. Then the carried unknown lines, as today.
- **Header** (brief decision): the existing first line `# Grimoire: links drawn on pages` stays. PLAN 13.1's example
  shows `# Grimoire: links on pages`, but keeping the bytes means a file whose links have no area is byte-identical to
  beta.5's, so `links_save_prune_and_carry` (test_nav.rs:243) keeps its expectation and a round trip through
  f9d13f5 changes nothing. The comment's wording is a comment; FORMAT.md explains that a link is no longer ink.

### 3.2 Reading

- `link_line` (nav.rs:358) gains an `area.` branch before the carry:
  - a line that parses as above goes into a load-time list `(pid, seed, Rect)` (a `Vec`, `try_reserve`; more than
    `NAV_LINKS_MAX` area lines make the file not read, as more than 4000 links do); a later line for the same pid and
    seed replaces the earlier one;
  - a line that starts with `area.` and does not parse is carried verbatim with `carry` (nav.rs:267), so it counts
    toward the 255 B / 16 KiB limits; its link falls back to the ink box.
- After the last line, `nav_load` (nav.rs:371) attaches each area to its link (`link_slot`). An area whose pid and seed
  have no link is not kept: it is counted in a new memory field `Nav.areas_dropped: i32`, and the next successful
  `nav_save_links` logs it once and resets the count:
  `qlog!("index: %d link area(s) with no link left out", n)` (new line).
- **Read cap**: links.ini gets its own cap, `NAV_LINKS_FILE_MAX = 512 * 1024`. `read_sidecar` (nav.rs:212) and
  `read_open_sidecar` (nav.rs:230) take the cap as a parameter: nav.ini passes `NAV_FILE_MAX` (256 KiB, unchanged),
  links.ini `NAV_LINKS_FILE_MAX`. `nav_save_links` checks the text against `NAV_LINKS_FILE_MAX` in place of
  `NAV_FILE_MAX`.
- **A file that does not read** (too big, a NUL, not a regular file, a symlink, past a limit) keeps today's rule:
  `links_writable = false`, no links in memory, never overwritten. Then nothing migrates and nothing can be made,
  changed or deleted; the names go on working.
- **Use-time clamping.** An area is used clamped to its page (`{0, 0, p.w, p.h}`); one left empty after a page-size
  change means no area, so the link falls back to its ink box (and is dormant without ink). The stored value is not
  rewritten by the clamp.

### 3.3 Versioning, and what older versions do with the file

links.ini has no version number (brief decision: none is added). A version adds kinds of lines, and every version
since beta.5 carries lines it does not read. A later version that needs another kind of area adds its own key
(`shape.<pid>=...`, for example) and this version carries it. The rows below go into FORMAT.md's "Moving between
versions" table and the Links section:

| Reader | What it does with a v2 links.ini |
|---|---|
| 1.0.0-beta to 2.0.0-beta.4 | never reads links.ini (the file stays as it is) |
| beta.5 builds (the device's f9d13f5, the C `claude/beta5-features`) | reads every `link.` line; a link with an area but no ink does nothing there; migrated links keep working through their ink, which migration never removes; carries `area.` lines as unknown lines up to 16 KiB (about 430 links); past that, or past 256 KiB, the file does not read there, so links are read-only there and the file is never overwritten |
| this version | areas first, the ink box for links without one; writes `area.` lines |

A beta.5 build that deletes a link leaves its `area.` line carried; this version drops it as an orphan at its next
save. A beta.5 build that changes a link's target keeps the area line, which still matches the pid and seed.

### 3.4 `NavLink` and the nav.rs API

```rust
pub const NAV_LINKS_FILE_MAX: usize = 512 * 1024;
pub const NAV_AREA_MAX: i32 = 6000;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]        // derives unchanged (Rect is Copy + Eq)
pub struct NavLink {
    pub pid: u32,
    pub seed: u32,
    pub target: FixedStr<NAV_TARGET_LEN>,
    /// the link's area, page px (meaningful when has_area)
    pub area: Rect,
    pub has_area: bool,
}
// Nav gains: pub areas_dropped: i32

/// Sets or clears page pid's link `seed`'s area. -1: not writable, no such link, or an area outside the format's range.
pub fn nav_link_set_area(nav: &mut Nav, pid: u32, seed: u32, area: Option<Rect>) -> i32;
/// Page pid's link `seed`, if it has one.
pub fn nav_link_get(nav: &Nav, pid: u32, seed: u32) -> Option<NavLink>;
/// Puts a whole link back (target and area, undo's restore). A new one obeys NAV_LINKS_PER_PAGE and NAV_LINKS_MAX.
pub fn nav_link_put(nav: &mut Nav, l: &NavLink) -> i32;
/// How many links of page pid have no area.
pub fn nav_links_legacy(nav: &Nav, pid: u32) -> i32;
/// Gives each link of page pid that has no area the box of its ink (3.6). How many got one; -1 when not writable.
pub fn nav_link_migrate(nav: &mut Nav, p: &Page, pid: u32) -> i32;
/// Every link of page `from` onto page `to` (same seed, target, area). -1 at a limit: nothing copied.
pub fn nav_link_copy(nav: &mut Nav, from: u32, to: u32) -> i32;
```

- `nav_link_set` (nav.rs:540) keeps its signature and behaviour: on an existing link it changes the target and keeps
  the area; a new link has no area until `nav_link_set_area`.
- `nav_link_remove` (nav.rs:556) removes the area with the link.
- `link_put` (nav.rs:497) inserts `NavLink { pid, seed, target, ..Default::default() }`. Every struct literal of
  `NavLink` gets `..Default::default()` (the `link` closure in test_nav.rs:235 is one; a mechanical edit, not an
  expectation change).
- `nav_save_links` (nav.rs:740) writes the area lines (3.1) and logs `areas_dropped` (3.2).

### 3.5 Boxes and the hit test (PLAN 9.2, 6.5)

- A new private `fn link_hit_box(l: &NavLink, p: &Page, min_px: f32) -> Option<Rect>`: the area clamped to the page
  when `has_area` and not empty, else the ink box (`link_box`, nav.rs:603, unchanged), then made at least `min_px` a
  side around its centre with `link_box`'s own expansion (the `fmla` lines, then `floor` / `ceil`), so an area and the
  ink box it came from give the same padded box within 1 px.
- `nav_link_areas` (nav.rs:663) returns `link_hit_box` for each link that has one, in links.ini order, as today. The
  eye, the bar (through `LinkBoxes`) and the PDF's /Link annotations (export.rs:172) follow with no change to their
  code.
- `nav_link_at` (nav.rs:683): among the links whose padded box holds (x, y), the one with the smallest box area
  (`w as i64 * h as i64`) wins; a tie goes to the lower seed (links are sorted by seed, so replace the best only on a
  strictly smaller area). This replaces "the stroke drawn last wins". Its doc comment changes to say so.
- The existing asserts of `links_follow_their_boxes` (test_nav.rs:180) happen to agree with smallest wins (the later
  box there is also the smaller); keep them, change the comment, and add a case where the later box is the bigger one
  (12.1).

### 3.6 `nav_link_migrate` (PLAN 9.8)

For every link of page pid without an area:
- area = the UNPADDED box of the points of the page's ink strokes with its seed (erase strokes ignored, as
  `link_box` ignores them), rounded outward to whole px (`floor` of the minimum, `ceil` of the maximum), each side at
  least 1 px, clamped to `{0, 0, min(p.w, 6000), min(p.h, 6000)}`;
- no such stroke: unchanged (the link stays dormant, legacy, and is pruned only with its page);
- one pass over `p.strokes`: collect the page's legacy seeds (at most 64, `NAV_LINKS_PER_PAGE`, kept sorted) with one
  box accumulator each, and for each ink stroke find its seed by binary search. Never one pass per link (a dense page
  with 64 links would cost 64 passes);
- strokes are read, never changed; the caller decides when to save.

Never use the 7 mm grown box: that would silently change every tappable area (PLAN 18).

### 3.7 Limits

| Limit | Value | Where |
|---|---|---|
| links in a notebook | 4000 (`NAV_LINKS_MAX`, unchanged) | a file with more does not read |
| links on a page | 64 (`NAV_LINKS_PER_PAGE`, unchanged) | making refuses with a toast (6.2) |
| area line values | 0 <= x, y; 1 <= w, h; x + w, y + h <= 6000 | else carried as an unknown line |
| links.ini size | 512 KiB to read and write (`NAV_LINKS_FILE_MAX`); about 82 B per worst-case pair, 4000 links about 330 KB | else does not read / does not save |
| unknown lines | 255 B each, 16 KiB together (unchanged) | |
| nav.ini | 256 KiB (unchanged) | |

---------------------------------------------------------------------------------------------------------------

## 4. Undo for link edits (`history.rs`, `editor.rs`, PLAN 9.7)

### 4.1 Types and API (history.rs)

```rust
pub enum ActKind { Add = 0, Erase = 1, Paper = 2, Clear = 3, Link = 4 }

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct LinkRec { pub seed: u32, pub target: FixedStr<NAV_TARGET_LEN>, pub area: Rect, pub has_area: bool }

/// One undo step's link changes on page pid: each record is (before, after) for one seed; None = no link.
#[derive(Clone, Debug, Default)]
pub struct LinkEdit { pub pid: u32, pub records: Vec<(Option<LinkRec>, Option<LinkRec>)> }

// Action gains: pub link: Option<Box<LinkEdit>>

/// Records a link step ALREADY applied to the Nav and saved. No records: no step. Clears redo; the limit applies.
pub fn history_push_link(h: &mut History, e: LinkEdit);
/// The kind of the step an undo (redo) would apply, None for none.
pub fn history_undo_kind(h: &History) -> Option<ActKind>;
pub fn history_redo_kind(h: &History) -> Option<ActKind>;
/// The link records of the step on top of the redo (undo) stack: the one history_undo (history_redo) just moved.
pub fn history_moved_link(h: &History, undone: bool) -> Option<&LinkEdit>;
/// Drops the step on top of that stack (a link step whose save failed).
pub fn history_drop_last_redo(h: &mut History);
pub fn history_drop_last_undo(h: &mut History);
```

- `apply` (history.rs:210) gains `ActKind::Link => *dirty = RECT_EMPTY`: the page is untouched. `history_undo` and
  `history_redo` (history.rs:247, 255) keep their signatures and move a Link step between the stacks like any other,
  returning true; the editor then applies its records to the Nav (4.2). This keeps one stack order for ink and links.
- Link steps live in the page's own `PageHistory` (the page the links are on), memory only, and count toward the
  500-step limit; the limit drops their records with them.
- Helpers on `LinkRec`: `fn link_rec(l: &NavLink) -> LinkRec` and `fn rec_link(pid: u32, r: &LinkRec) -> NavLink`
  (in editor.rs, so history.rs stays free of nav).

### 4.2 Applying a step (`undo_redo`, editor.rs:2303)

```text
undo_redo(a, undo):
  settle_now; refuse_readonly                                // as today
  h = cur_hist; ip = page_ix
  kind = if undo { history_undo_kind } else { history_redo_kind }
  if kind != Some(ActKind::Link) -> today's ink path, unchanged
  history_undo / history_redo (moves the step; d is empty)
  e = history_moved_link(h, undo)
  for (before, after) in e.records: want = if undo { before } else { after }
      Some(r) -> nav_link_put(rec_link(e.pid, r))  |  None -> nav_link_remove(e.pid, seed)
  if any put failed or nav_save_links fails:
      nav_load (back to the file), migration_reset (5.4)
      history_drop_last_redo (undo) / history_drop_last_undo (redo)
      toast 'Could not save the links: that change was not undone.' / '... was not redone.' (3000 ms)
      boxes stale; repaint_panel(Ui); return
  links_dirty = false; s.changed = true (the notebook changed: the PDF mirror runs at close); boxes stale
  if settings.link_areas: compose and invalidate (Ui) the union of every record's padded boxes, before and after, on screen
  else: toast 'Link change undone.' / 'Link change redone.' (2000 ms): nothing visible changed (PLAN 12.3's rule)
  repaint_panel(Ui)
```

- The page file is not touched, so `page_dirty` is not set (`mark_changed` is for ink).
- Redo of a link to a page deleted since: the save leaves it out (`link_kept`), so it does nothing, as PLAN 9.7 asks.
- Undo is refused while the bar, the card or Link to... is up (`sheet_open`), so a link step never applies under them.

### 4.3 What is one step

| Edit | Records |
|---|---|
| make a link (6.7) | (None, Some(new)) |
| delete from the card (8.9) | (Some(old), None), one per link: N from a ×N row |
| change where it goes (8.7) | (Some(old), Some(old with the new target)), one per link |
| redraw the area (8.8) | (Some(old), Some(old with the new area)) |
| rename (8.6) | not a step (a notebook-wide name, PLAN 19) |
| migration (5) | not a step (invisible, and the ink box it copies stays) |
| Copy page's links (9.2) | not a step (Copy page is not undoable) |

A refused or failed edit pushes no step: push only after `nav_save_links` returned 0.

---------------------------------------------------------------------------------------------------------------

## 5. Migration in the editor (PLAN 9.8)

### 5.1 State

`EditorState` gains:
- `links_dirty: bool`: migrated areas are in memory and not yet in links.ini;
- `migrate_done: Vec<u32>`: page ids tried this session (migrated, dormant only, read-only, or unreadable), cleared at
  notebook open and close and by `migration_reset`;
- `links_try_at: u32`: when the next flush may be tried after a failure;
- `links_fail_logged: bool`.

### 5.2 At page load

In `page_at` (editor.rs:477), after a page is read into the session cache (the first visit only), and only when
`!page.readonly && nav.links_writable && nav_links_legacy(nav, pid) > 0`:
`if nav_link_migrate(&mut self.nav, page, pid) > 0 { self.links_dirty = true; self.boxes.stale = true }`, then push
pid on `migrate_done`. (`page_at` is an `EditorState` method; `self.nav` and `self.hist[e].page` are disjoint fields.)
The open page at notebook open comes through `page_at` too, so the owner's links are areas before the first tap.

Read-only pages are never migrated (brief decision): a page from a newer Grimoire may not show all its strokes, so its
ink box could be wrong. Its links keep the ink-box fallback.

### 5.3 On idle ticks

In `editor_tick` (editor.rs:4094), at most one page per tick, when all hold: `s.open`, `a.input_drained`,
`s.owner == Owner::None`, `!s.pinching`, `!sheet_open(s)`, no making in progress (`s.lm.state == Off`), and
`nav.links_writable`:
- the first page in notebook order whose links include one with no area and whose id is not in `migrate_done`;
- in the session cache: migrate it there;
- else read it with `store_load_page_pumped(&s.nb, i, s.pgw, s.pgh, &mut p)` (PHASE1 13.9), migrate `p`, and drop
  it (it is not put in the session cache, to keep memory flat);
- push its id on `migrate_done` whatever happened (a read failure, a read-only page, a dormant link), so no page is
  tried twice.

### 5.4 Flush, and a rollback

- `fn links_flush(a: &mut App)`: when `links_dirty && nav.links_writable`, `nav_save_links`; success clears
  `links_dirty`; failure logs once per session `qlog!("index: link areas left for a later save")` (new line), sets
  `links_try_at = now + 2000` and keeps the flag.
- Called (it returns at once when nothing is dirty): in `go_to_page` after `save_page`; in `bar_commit` after its save;
  in the 2 s idle autosave branch of `editor_tick` (a separate check: `links_dirty` and `now - links_try_at >= 0`,
  since a migrated page is not `page_dirty`); in `close_notebook` before `bg_mirror_pdf`, so the PDF reads the
  areas; in `editor_leave` and `editor_suspend`.
- Every link edit saves the whole file, migrated areas included, and clears `links_dirty` on success.
- `fn migration_reset(s)`: after any `nav_load` rollback (a failed edit), clear `links_dirty` and `migrate_done` and
  mark the boxes stale: the file's state is in memory again, and the idle ticks migrate what is still legacy, pages in
  memory included.

---------------------------------------------------------------------------------------------------------------

## 6. Making a link without ink (`editor.rs`, `linkbar.rs`, `canvas.rs`; R1, PLAN 6.4, 9.3)

### 6.1 State

`EditorState.link_mode: bool` (editor.rs:268) is replaced by `lm: LinkMaking`. `link_seed: u32` (editor.rs:270) stays
only until the card replaces `link_hold` (commit 11): until then the Pages-view hold path still reads it.

```rust
#[derive(Clone, Copy, Default, PartialEq)]
enum LmState { #[default] Off, Armed, Lasso, Pick, Naming }

#[derive(Default)]
struct LinkMaking {
    state: LmState,
    /// Redraw the area of the link with this seed (8.8), instead of making one
    redraw: Option<u32>,
    /// the card's Change where it goes (8.7): the seeds to retarget (N from a bar row); empty when making
    retarget: Vec<u32>,
    /// came from the bar's New link or a bar card: the bar comes back afterwards
    from_bar: bool,
    /// the lasso, page px, at most 4096 points, and the spacing a new point needs (screen px, doubles when full)
    pts: Vec<(f32, f32)>,
    spacing: f32,
    /// the dash phase at the last point, so the lasso drawn piece by piece matches the one compose draws whole
    phase: i32,
    /// screen px: what the lasso has drawn so far
    lasso_rect: Rect,
    /// page px: the area, from the lift until the pick
    area: Rect,
    /// the strip and its Cancel
    strip: Rect,
    hits: HitList,
    press: Press,
    /// New page...'s naming keyboard
    kb: Keyboard,
}
```

- `sheet_open` (editor.rs:2228) adds `|| s.lt.open || s.card.open` (Link to... and the card). The strip (Armed) is
  deliberately NOT a sheet: while armed the page stays fully interactive (pinch zoom and two-finger pan to circle
  precisely, finger taps on links, page turns), and only the next pen contact on the page is the lasso.
- `touch_context` (editor.rs:951): `zoom_ok` also requires `!(s.lt.open || s.card.open)`; `keyboard_open` adds
  `s.lm.kb.open || s.card.kb.open`.
- Probes: `editor.link_mode` is `1` while `Armed` or `Lasso`, `editor.strip`, `editor.lasso`, `editor.link_area`,
  `editor.linkto*` (section 11).

### 6.2 Arm, and the refusals

`fn link_arm(a, from_bar: bool, redraw: Option<u32>)` replaces `link_start` (editor.rs:2490). Openers:
- the `add-link` action (the existing branch at editor.rs:3174: armed or picking, it cancels; otherwise it arms);
- the bar's New link (`LbResult::NewLink`): commit first if peeking (`bar_commit`), close the bar, arm with
  `from_bar`;
- the Index tab's Add a link (`PvResult::Link`, editor.rs pages_event): `pages_hide(a, -1)`, then arm;
- the card's Redraw the area (8.8).

Refusals, in this order, each a toast and nothing armed:
1. a read-only page: `refuse_readonly` (editor.rs:1429, its existing toast);
2. `!nav.links_writable`: 'Links could not be read in this notebook.' (3000 ms);
3. a new link (not a redraw) on a page with `NAV_LINKS_PER_PAGE` links: 'This page has %d links: delete one to add
   another.' (3000 ms, %d = 64).

Arming: `state = Armed`, lay out the strip, `compose(strip)` (and the closed bar's rect when the bar folded into it),
`app_invalidate(.., Ui)`. No toast: the strip says what to do. Arming disarms nothing else; opening any menu disarms
(6.10).

### 6.3 The strip

- Upright: x 20, 1820 x 130, its bottom 20 px above the screen edge (sideways 2440 x 130). The text 'Draw around what
  links.' (or, for Redraw, 'Draw the link's new area.') in 44 px bold at x + 32, centred vertically; a Cancel button of
  260 x 100 at the right, 15 px in.
- Drawn by `compose` (after the panel and sheets, where the bar draws) while `Armed` or `Lasso`.
  `a.toast_floor = strip.y` after it.
- A down inside the strip (finger or pen) is `Owner::Strip`: Cancel inverts at the down (`app_press_feedback`) and
  cancels at the up if `press_is_tap`; elsewhere on the strip nothing happens. A pen there never starts the lasso.

### 6.4 The lasso (`Owner::Lasso`)

In `editor_event`'s down dispatch (editor.rs:3833 onwards) while `state == Armed`, after the strip's own test: a pen
down that is not on the panel becomes `Owner::Lasso` (a finger down is handled as today: panel, link tap or hold, a
second finger a gesture). A pen with the side button pressed sets `btn_used_for_ink` as a stroke does.

- **Begin:** `state = Lasso`; `pts` = the down's page point (`to_page`, editor.rs:1033); `spacing` = 2 screen px;
  `phase = 0`.
- **Each PenMove:** skip the point when it is within `spacing` screen px of the last kept one. When `pts` reaches 4096,
  keep every other point and double `spacing`. Draw the new segment into `a.screen` with
  `canvas_dash_poly(&mut a.screen, &seg, ui_px(4.0), ui_px(12.0), ui_px(8.0), 0, phase)` (black, no halo), clipped to
  the page's screen rect; a segment whose box meets the panel or the strip is recorded but not drawn (compose draws the
  lasso under them anyway). `app_invalidate(segment box, RefreshMode::Fast)`: the loop's one flush per pass presents
  one DU per input batch, as live ink does. Nothing touches `page_canvas`, the page, the history or the PDF.
- `compose` draws the whole lasso (`state == Lasso`) as an overlay after the eye's boxes and before the panel, so any
  recompose keeps it.
- **New primitive** (canvas.rs, PLAN 14.5):
  `pub fn canvas_dash_poly(c: &mut Canvas, pts: &[(f32, f32)], thick: i32, on: i32, off: i32, halo: i32, phase: i32) -> i32`:
  dashes of `thick` px along the polyline, `on` then `off` px measured along the path, starting `phase` px into the
  pattern, returning the phase at the end; a halo of `halo` px of white on both sides when `halo > 0`; 1-bit only
  (gray 0 and 255); respects `c.clip`. Phase 3 adds its dot mode.
- **E-ink:** DU frames only while the pen is down. Never pen mode, never the live ink style (rule 6), so
  `display.pen_mode` and `display.ink` stay 0 and `display.last_fast` never reads `ink` for the lasso.

### 6.5 The lift

At PenUp or PenLeave (`fn lasso_end(a)`):
- r = the points' box in page px (`floor` / `ceil`), clamped to the page.
- **Too small** (`r.w` and `r.h` both under `LINK_BOX_MIN_MM * dpi / 25.4`, editor.rs:24): `state = Armed` (the mode
  stays), `compose(lasso_rect)` + `Ui`, toast 'Too small for a link: draw around what links.' (existing msgid, 3000 ms).
- **Redraw** (`redraw = Some(seed)`): 8.8.
- **New link:** `state = Pick`, `area = r`. `compose(lasso_rect ∪ strip)`: the dashes and the strip go, and compose
  draws the dotted rectangle (while `Pick` and `retarget` is empty) around `page_to_screen(area)` with
  `canvas_dash_rect(.., ui_px(3.0), ui_px(8.0), ui_px(8.0), ui_px(2.0))`. `app_invalidate(lasso_rect ∪ strip ∪
  rectangle band, Ui)` and `app_flush(a)` (a stated separate flush: one GL16). Then Link to... opens (6.6) with its own
  GL16, so the two never merge into one union across the screen.

### 6.6 Link to... (the bar's frame in pick mode, `linkbar.rs`)

`EditorState` gains `lt: LinkBar`, a second `LinkBar` in pick mode (brief decision: the bar's own `lb` keeps its
origin and back state, and phase 1 code reads `lb.open` as "the bar is up"). Owner for presses on it:
`Owner::LinkTo`.

```rust
#[derive(Clone, Copy, Default, PartialEq)] pub enum LbMode { #[default] List, Pick }
#[derive(Clone, Copy, Default, PartialEq)] pub enum PickKind { #[default] NewPage, Index, Recent, Entry }
#[derive(Clone, Copy, Default)]
pub struct LbPick { pub kind: PickKind, pub target: FixedStr<NAV_TARGET_LEN>, pub page: i32 }

// LinkBar gains:
//   pub mode: LbMode,
//   pub picks: Vec<LbPick>,                       // every chip, all screens
//   pub top: bool,                                // docked at the top
//   pub current: FixedStr<NAV_TARGET_LEN>,        // the target marked (Change where it goes), empty when new
//   pub no_pages: bool,                           // opened by the card: Choose from the pages... is not offered
//   pub hidden: bool,                             // under Find PICK, New page...'s keyboard or the Pages view
//   pub held: i32, pub down_at: u32,              // the row hold (8.10), -1 none

pub fn linkbar_build_pick(lb: &mut LinkBar, nb: &NotebookInfo, nav: &Nav, recent: &[FixedStr<NAV_TARGET_LEN>],
                          open_pid: u32, current: &str, no_pages: bool) -> i32;   // -1: no memory for the entries
pub fn linkbar_layout_pick(lb: &mut LinkBar, aw: i32, ah: i32, top: bool, top_y: i32);
// LbResult gains: Card(i32) (List mode, 8.10), Pick(i32) (chip i of picks), Search, Choose
```

`linkbar_draw`, `linkbar_pointer` and `linkbar_probe` serve both modes.

**Chips, in this order** (PLAN 9.3 step 5):
1. New page... (`PickKind::NewPage`);
2. The Index (`index`);
3. up to 4 Recent targets (6.6.1);
4. the Index entries: every named page in Index order (`nav_index`, nav.rs:454), the open page included.

`picks` is a `Vec` sized with `try_reserve_exact`. When it cannot be allocated, Link to... offers only chips 1-3 and
the tool row (Search reaches every page), and logs `qlog!("index: no memory to list the Index for a link")` (new
line).

**Layout** (logical px at 300 dpi, `ui_px`): the bar's frame (PHASE1 13.3) without the X and the hint line.
- Upright: x 20, 1820 wide; 16 px padding, the 36 px grip strip, a 100 px header ('Where does the link go?' at the
  left in 44 px bold, the pager '‹ 1/3 ›' of two 118 x 100 buttons at the right when there are more than 6 chips); R
  rows of 2 chips of 878 x 124 (24 px column gap, 16 px row gap), R = 1..3, 6 chips a screen; a 20 px gap; a 140 px
  tool row; 20 px padding. Height = 316 + 140 R.
- Tool row: Search / Choose from the pages... / Cancel, three buttons of 580 x 140 with 20 px gaps; from the card
  (`no_pages`) Search / Cancel, two of 880 x 140.
- Sideways: 2440 wide, 3 columns of 784 x 124, R <= 2, tool buttons 786 (three) or 1190 (two).
- **Dock** (PLAN 9.3 step 5): at the TOP, its top at `corner_zone(a).y + corner_zone(a).h + 8` (150 px with the
  zone at rest), when the centre of the area's screen rect (for a retarget, of the bordered boxes' union) is at
  `y >= 0.55 * ah` (the lower 45% of the screen); else at the bottom, 20 px above the edge.
- **Chip:** the bar's chip with its 64 px disc slot holding an icon instead of a number: `IconKind::Plus` (New
  page...), `IconKind::Notebook` (The Index), `IconKind::Clock` (a recent target), nothing for an Index entry; the
  label from x + 96 (the target's Index name in 44 px bold cut with `font_draw_left_name`, 'Page %d' regular for an
  unnamed page, 'New page...', 'The Index'); 'p. %d' in the 150 px right column for a page. The current target (a
  retarget) gets the selected chip's 8 px frame and `IconKind::Check` in its slot; the pager opens on the screen that
  holds it.
- `compose` draws Link to... after the bar's place (it is never up with the bar) and sets `a.toast_floor` to its top
  when docked at the bottom.

**Pointer** (`Owner::LinkTo`, as the bar's): a chip or button inverts at the down (`app_press_feedback`) and acts at
the up if `press_is_tap`; the pager recomposes the chip grid (GL16 of its rect). A down outside Link to... is a tap
outside: a finger becomes `Owner::Outside`, decided at the lift (a cancel up: nothing; `corner_tap`: the clock only;
otherwise Cancel, 6.10, the tap swallowed); a pen cancels at the down and `Owner::Outside` swallows its stroke.

#### 6.6.1 Recent targets

- `EditorState.recent: Vec<FixedStr<NAV_TARGET_LEN>>`, newest last, at most 8, memory only, cleared at notebook close;
  pushed with each successful pick or retarget whose target is a page.
- The chips: from `recent` newest first, then the back list (`s.back`, newest first, as `p<pid>`); leaving out the
  open page, `index`, targets that resolve to no page (`nav_link_page` < 0) and repeats; at most 4.

### 6.7 Picking (`fn link_pick(a, target: &str)`, `target` = `index` or `p<pid>`)

For a new link (`retarget` empty, `redraw` None):
1. seed = `nav_new_seed(&nav, page, pid, now_ms().wrapping_mul(2654435761) ^ nstrokes)` (nav.rs:708; `nstrokes` the page's stroke count): unique among the
   page's ink seeds and link seeds, so a link never takes in ink and new ink never joins a link.
2. `nav_link_set(nav, pid, seed, target)`, `nav_link_set_area(nav, pid, seed, Some(area))`, `nav_save_links`. On any
   failure: `nav_load`, `migration_reset`, toast 'Could not save the link.' (existing msgid), end the making.
3. On success: `history_push_link(cur_hist, LinkEdit { pid, records: vec![(None, Some(rec))] })`;
   `links_dirty = false`; `s.changed = true`; push the recent target; boxes stale; `panel_stale` (Undo is enabled now).
4. `state = Off`; `compose(rectangle band ∪ lt.rect)`, `app_invalidate(.., Ui)` (the rectangle goes unless Areas is
   on, which now draws the new box); close `lt`; `repaint_panel(a, Ui)`.
5. Toast (2500 ms): with `hints` bit 1 clear, 'Linked. Tap it with a finger to go there; hold it for more.' (4000
   ms), then set bit 1 and `settings_save`; otherwise 'Linked to %s.' (a named page, its name), 'Linked to page %d.' (an
   unnamed page) or 'Linked to the Index.'.
6. `from_bar`: the bar comes back on the open page with the new entry (the bar's `open_menu(SHORTCUT_LINKS)` path:
   build, layout, compose, GL16), invalidated in the same pass.

A chip pick calls `link_pick` with its target. New page... goes to 6.8, Search to Find PICK (7), Choose from the
pages... to 6.9, Cancel to 6.10.

### 6.8 New page... (PLAN 9.3 step 7)

- `state = Naming`; `lt.hidden = true`; `keyboard_open(&mut s.lm.kb, tr("Name the new page"), b"")`, letters page,
  docked away from the area (`keyboard_set_dock`, 7.1: at the top exactly when Link to... is). Typing goes through
  `app_keyboard_pointer` (app.rs:1632) and `app_keyboard_tick` (app.rs:1663), as every keyboard does.
- Done: `store_add_page(&mut s.nb, npages, s.nb.paper)` (store.rs:1603, the end of the notebook, the notebook's
  paper). Failure: toast 'Could not add a page' (existing msgid), back to Link to.... Success: the new pid is the last
  page id; a non-empty name with `nav.writable` gets `nav_set_name` and `nav_save` (a failure toasts 'Could not save
  the page name.', existing, and the page stays unnamed); `s.changed = true`; then `link_pick(a, "p<new pid>")` with the
  toast 'Linked to the new page %d.' in place of step 5's. The editor stays on its page.
- Cancel (or a tap outside the keyboard): no page is added, back to Link to... (`lt.hidden = false`, GL16 of the
  keyboard area and Link to...).
- `!nav.writable`: no keyboard; the page is added unnamed at once.
- The page added is not undone by Undo (Undo removes the link; section 0.1).

### 6.9 Choose from the pages... (PLAN 9.3 step 5)

- Offered only when Link to... comes from New link (`!lt.no_pages`).
- `lt.hidden = true`; `with_pv(a, |pv, a, nb, nav| pagesview_pick(pv, a, nb, nav, index))` (no `had` parameter any
  more, 9.1); `compose_all(a, Full)`; toast 'Where does the link go? Tap its page.' (existing msgid, 3000 ms).
- `pages_event`: `PvResult::Pick` closes the Pages view (`pages_hide(a, -1)`) and calls `link_pick(a, "p<pid of
  pv.target>")`; `PvResult::Close` while picking returns to Link to... (`pages_hide(a, -1)`, `lt.hidden = false`; the
  full compose shows the page, the dotted rectangle and Link to...).

### 6.10 Cancel, and leaving

- **Cancel** (`fn link_cancel(a, toast: bool)`, replacing today's `link_cancel`, editor.rs:2543) from: the strip's or
  Link to...'s Cancel; a tap outside Link to...; the `add-link` action again; Close menu; `go-back` while picking (as
  the bar takes it); another menu's action (cancel first, then that menu, section 8.11).
  - `state = Off`; `compose(strip ∪ lasso_rect ∪ rectangle band ∪ lt.rect)` + `Ui`; close `lt`.
  - toast 'No link made.' (existing msgid, 2500 ms) for a new link; silent for a retarget or a redraw (nothing was
    being made; the border goes);
  - `from_bar`: the bar comes back.
- **Silent cancels** (no toast, no bar): a page change (`go_to_page` already clears link mode, editor.rs:1485), any
  menu opening (`open_menu`, `pages_show`), the card opening on another link, `editor_leave`, `editor_suspend`,
  `close_notebook`, a rotation (`editor_enter` with a size change). A cancel during `Lasso` (suspend) ends the lasso
  first.

### 6.11 Removed

`link_box_drawn` (editor.rs:2508), `link_box_too_small` (editor.rs:1808; replaced by the box test in `lasso_end`), the
link-mode branches of `draw_end` (editor.rs:1937-1946 and 1979-1981), `link_cancel`'s undo of the box, `link_to`
(editor.rs:2525; replaced by `link_pick`), `link_stop`'s callers (it becomes `link_cancel(a, true)`) and the old toasts
(14.3). Inked making never ships past phase 2 (PLAN R1).

---------------------------------------------------------------------------------------------------------------

## 7. Find PICK mode and the keyboard dock (`findsheet.rs`, `keyboard.rs`; PLAN 8.1, 8.2)

### 7.1 The keyboard dock (keyboard.rs)

- `Keyboard` (keyboard.rs:29) gains `pub dock_y: i32`: 0 docks at the bottom, as today; otherwise the modal's top y.
- `pub fn keyboard_set_dock(kb: &mut Keyboard, top_y: i32)`. `keyboard_open` (keyboard.rs:328) sets `dock_y = 0`, so
  every existing keyboard stays at the bottom; a caller that docks calls `keyboard_set_dock` after `keyboard_open`.
- `modal_rect` (keyboard.rs:239) takes the dock: `dock_y > 0` gives `y = screen.y + dock_y`, else today's expression,
  byte for byte. `keyboard_draw` (keyboard.rs:382) passes `kb.dock_y`.
- Docked at the top, the modal starts at `corner_zone(a).y + corner_zone(a).h + 8` (150 px at rest), never in the
  zone. Upright the modal is 980 px tall, so it ends at 1130; SIM.md notes that its keys sit 1330 px higher than the
  documented bottom-docked coordinates upright (1480 - 150).
- `toast_floor` follows the keyboard only when it is docked at the bottom.

### 7.2 Find PICK

- `FindSheet` gains `pub mode: FindMode` (`#[derive(Default)] enum FindMode { #[default] Go, Pick }`) and
  `findsheet_open_pick(f: &mut FindSheet, a: &mut App, top_y: i32)` (0 = bottom).
- Opened by Link to...'s Search: `lt.hidden = true`; the keyboard docks away from the area, at the top exactly when
  Link to... is docked at the top. The rows card sits above the keyboard modal when it is at the bottom (as GO mode),
  and below it (its top at `kb.area.y + kb.area.h + 16`) when it is at the top.
- Rows are GO mode's, with one change: a digits row reads 'Link to page %d' instead of 'Go to page %d'; 'There is no
  page %d (%d pages).' and 'No page names match.' are unchanged.
- A row tap, or Done on the first tappable row, gives `FsResult::Go(i)`; in PICK mode the editor closes Find and calls
  `link_pick(a, "p<pid of i>")`: no page change, no back-list push.
- Cancel, a tap outside (the corner excepted: `corner_tap` first), Close menu or `go-back` in PICK mode return to Link
  to... (`lt.hidden = false`; `compose(rows ∪ kb.area ∪ lt.rect)` + `Ui`), not out of the making.
- `editor.find` reads `pick`; `editor.modal` reads `find`.

---------------------------------------------------------------------------------------------------------------

## 8. The long-press card (`linkcard.rs`, new, editor-hosted; R2, D1, D3; PLAN 6.3, 7.9, 9.5, 9.6)

### 8.1 Types and API

```rust
#[derive(Clone, Copy, Debug, Default, PartialEq)] pub enum CardRow { #[default] Rename, Retarget, Redraw, Delete }
#[derive(Clone, Copy, Default, PartialEq)] pub enum LcResult { #[default] None, Row(CardRow), Changed }

#[derive(Default)]
pub struct LinkCard {
    pub open: bool,
    /// the page the links are on (the open page) and their seeds: 1 from the page, N from a bar row
    pub pid: u32,
    pub seeds: Vec<u32>,
    pub target: FixedStr<NAV_TARGET_LEN>,
    /// nav_link_page: a page index, NAV_TO_INDEX, or -1 (a page gone, another version's target)
    pub page: i32,
    pub readonly: bool,
    pub from_bar: bool,
    pub rows: [CardRow; 4], pub nrows: usize, pub enabled: [bool; 4],
    /// page px: the padded boxes with the heavy border; `border` while it is drawn (also during a retarget or redraw)
    pub boxes: Vec<Rect>,
    pub border: bool,
    /// screen px: everything the border covers (for closing), and the card
    pub band: Rect,
    pub rect: Rect,
    /// Rename's keyboard
    pub kb: Keyboard,
    pub hits: HitList, pub press: Press,
}

pub fn linkcard_set(c: &mut LinkCard, pid: u32, seeds: &[u32], nb: &NotebookInfo, nav: &Nav, readonly: bool, from_bar: bool);
/// Pure: where a w x h card goes beside `anchor` (8.4). Unit-tested.
pub fn linkcard_place(w: i32, h: i32, anchor: Rect, screen: Rect, panel: Rect, corner: Rect) -> Rect;
/// Pure: above the bar, in the held chip's column.
pub fn linkcard_place_above(w: i32, h: i32, chip: Rect, bar: Rect, screen: Rect) -> Rect;
pub fn linkcard_draw(c: &mut LinkCard, a: &mut App, nb: &NotebookInfo, nav: &Nav) -> Rect;
pub fn linkcard_pointer(c: &mut LinkCard, a: &mut App, ph: i32, ev: &InEvent, dirty: &mut Rect) -> LcResult;
pub fn linkcard_probe(c: &LinkCard, key: &str, out: &mut String, cap: usize) -> bool;
/// The heavy border around r (screen px): canvas_dash_rect(c, r, ui_px(5), ui_px(20), ui_px(14), ui_px(3)) and solid
/// 24 px corner Ls of the same thickness.
pub fn linkcard_border(c: &mut Canvas, r: Rect);
/// The four strips that border covers, without overlaps: top and bottom the full width, left and right between them.
pub fn linkcard_strips(r: Rect) -> [Rect; 4];
```

Register `pub mod linkcard;` in lib.rs beside `linkbar`, and its prelude line. `EditorState` gains `card: LinkCard`;
the editor takes it out for each call that needs the App (`with_modal`). New owners (editor.rs:64):
`Lasso = 13`, `Card = 14`, `LinkTo = 15`, `Strip = 16`.

### 8.2 Opening from the page, and the border's refresh

`link_hold` (editor.rs:2677) becomes `card_open_link(a)`, still fired by `editor_tick` at 600 ms within the 30 px slop
(editor.rs:4119, `LINK_HOLD_MS`):
1. `link_held = true` (the lift then does nothing, as today).
2. The link under the landing point (`link_under`, smallest wins). None: return.
3. A making in progress is cancelled silently (6.10).
4. `linkcard_set(.., seeds = [seed], readonly = page.readonly, from_bar = false)`; `boxes` = that link's padded box
   (`nav_link_areas` for its seed).
5. **The border by DU** (PLAN 9.5): for the box, `r = rect_inflate(page_to_screen(box), ui_px(8.0))`;
   `linkcard_border(&mut a.screen, r)`; then for each of `linkcard_strips(r)`: `app_invalidate(strip, Fast)` and
   `app_flush(a)` (four stated separate flushes). One flush of all four would union them into the whole box and DU
   would posterize the gray ink inside; the strips hold only black and white pixels, so DU shows them exactly, and
   they do not overlap, so the driver runs them side by side (about 0.2 s).
6. **The card by GL16:** place it (8.4), `linkcard_draw`, `app_invalidate(card.rect, Ui)`: the loop's flush
   presents it alone, after the strips.
- The finger is still down; its lift ends `Owner::Link` with `link_held`, so nothing follows. The page stays in view
  (R2): no Pages view, no full refresh.
- `compose` draws the border of every box in `card.boxes` while `card.border` (after the eye's boxes, before the
  panel) and the card itself last (above the bar), so a recompose keeps both.
- A hold in the corner zone opens the card: a finger down longer than 600 ms is never a corner tap (1.2).

### 8.3 Size and rows

The card is 780 px wide. From the top: 24 px padding; a 110 px title row (the target's name in 44 px bold cut with
`font_draw_left_name`, then 'p. %d' at 34 px, and '×%d' for a ×N row); the rows, each a `ui_button` of 732 x 118 with
an icon, 12 px apart; for a ×N row the line 'To change one, hold it on the page.' (34 px, 60 px tall, 12 px above it);
24 px padding. Height: 4 rows 666 px; 3 rows 536; a ×N card 608 (3 rows) or 478 (2 rows); read-only 218. The same
size sideways.

| Case | Title | Rows (icon) |
|---|---|---|
| a link to a named page | its Index name, 'p. %d' | Rename in the Index (Rename), Change where it goes... (Link), Redraw the area (Pencil), Delete link (Trash) |
| a link to an unnamed page | 'Page %d', 'p. %d' | Add page %d to the Index... (Rename), then the same three |
| a link to the Index | 'The Index' | Change where it goes..., Redraw the area, Delete link (no Rename row, PLAN 6.3) |
| a page gone, or another version's target | 'Page not here' | Rename in the Index (disabled), Change where it goes..., Redraw the area, Delete link |
| `nav.ini` unwritable | as above | the Rename row disabled |
| a bar row with N > 1 links | as above, '×%d' | Rename in the Index / Add page %d to the Index... (the entry, D3; absent for the Index), Change where they go..., Delete %d links, then the line (no Redraw, D1) |
| a read-only page | as above | no rows; the line 'This page is read-only.' |

A disabled row is drawn `ButtonStyle::Disabled` and has no hit. Delete is last (destructive last, PLAN 19).
TRANSLATORS budget for the rows: 'about 700 px at 44 px'.

### 8.4 Placement (`linkcard_place`, PLAN 9.5)

`anchor` = the border's outer rect on screen (the union of `r` over the boxes), margin m = 20 px, gap 24 px.
1. Candidates in order: right of the anchor (`x = anchor.x + anchor.w + 24`, `y = anchor.y`), left, below
   (`x = anchor.x`), above. Each is clamped into the screen inside the margin on its free axis.
2. A candidate that meets the panel slides horizontally to the nearer side of the panel that stays on screen.
3. Accept the first candidate that is on screen, does not cover the anchor, does not meet the panel and does not meet
   `corner`. If none, the first that is on screen, does not cover the anchor and does not meet `corner` (the card may
   cover part of the panel: it draws above it). If none, the right candidate clamped to the screen and moved down to
   `corner.y + corner.h + 8` when it meets the zone (it may cover part of the box).

From the bar, `linkcard_place_above`: `y = bar.y - 16 - h`, `x = chip.x` clamped to `[20, aw - w - 20]`. The bar is
at most 736 px and the card 666, so it always fits above the bar and below the corner zone, upright and sideways
(1860 - 20 - 596 - 16 - 666 > 150).

### 8.5 Pointer routing while the card is up

In `editor_event`'s down dispatch, in this order: Find, the card, Link to..., the bar (phase 1), then the rest.
- **Inside `card.rect`** (finger or pen): `Owner::Card`. A row inverts at the down (`app_press_feedback`) and acts at
  the up if `press_is_tap` (`LcResult::Row`). While Rename's keyboard is up, presses on it go through
  `app_keyboard_pointer`.
- **Outside**, finger: `Owner::Outside`, decided at the lift: a cancel up does nothing; `corner_tap` toggles the clock
  and the card stays (PLAN 15.3's z_link_card); otherwise the card closes (8.12) and the tap does nothing else (no link
  followed, no panel press, no bar chip). With Rename's keyboard up, a tap outside both cancels the rename and the card
  stays.
- **Outside**, pen: the card closes at the down and `Owner::Outside` swallows the stroke until PenUp (rule 13).
- A second finger clears `Owner::Card` and `Owner::Outside` at `PinchBegin` (as phase 1 does for the bar's owners),
  un-inverting a pressed row with `Fast`.
- `editor_busy` is unchanged (a non-None owner is busy).

### 8.6 Rename (D3, PLAN 9.6)

- `keyboard_open(&mut card.kb, tr("Name in the Index"), <the target's name>)`, letters page, docked away from the
  border: at the top when the anchor's centre is at `y >= 0.55 * ah`. The card stays drawn if it does not meet the
  keyboard's area, else it is left out while the keyboard is up; the border always stays. GL16 of the keyboard area as
  it opens.
- Done: the target page's pid from `card.target` (`p<pid>`); keep the old name; `nav_set_name(nav, pid, text)`
  (nav.rs:171) and `nav_save` (nav.rs:422). Failure: put the old name back, toast 'Could not save the page name.'
  (existing msgid). Success: `s.changed = true`; toast (3000 ms) 'Page %d is in the Index as %s.' for a non-empty
  name, 'Page %d is not in the Index now.' when an existing name was emptied (an empty name removes it, as everywhere),
  nothing when an unnamed page stays unnamed.
- After Done or Cancel the keyboard closes and the card comes back with its title and Rename row updated: GL16 of
  `kb.area ∪ card.rect` (brief decision: the card stays open after Rename, so the owner sees the result). From the bar,
  the bar is rebuilt and redrawn in the same pass (its labels changed).
- Rename is not an undo step (4.3).

### 8.7 Change where it goes (PLAN 9.6)

- The card closes (`card.open = false`, GL16 of its rect, flushed) but the border stays (`card.border`). From the
  bar, the bar closes too (Link to... is never up with the bar; its origin is kept) and comes back after the pick or
  the cancel.
- `lm.retarget = card.seeds`, `lm.from_bar = card.from_bar`, `state = Pick`; Link to... opens with
  `current = card.target`, `no_pages = true` (no Choose from the pages..., R2) and its dock from the anchor (6.6).
- A pick: for each seed, `nav_link_get`, change the target, `nav_link_put`; `nav_save_links`; on failure `nav_load`,
  `migration_reset`, 'Could not save the link.'. On success one `ActKind::Link` step with N records (4.3), recent
  target pushed, toast as 6.7 step 5 (never the bit 1 tip), the border goes (`compose(band ∪ lt.rect)` + `Ui`), and
  from the bar the bar is rebuilt and redrawn.
- Picking the current target changes nothing: no step, no toast; Link to... and the border close.
- New page... and Search work here too (they stay on the page). Cancel or a tap outside: silent (6.10).

### 8.8 Redraw the area (PLAN 9.6; N = 1 only)

- The card closes; the border stays, showing the old area; `link_arm(a, card.from_bar, Some(seed))`: the strip reads
  'Draw the link's new area.'. From the bar, the bar folds into the strip as for New link and comes back afterwards.
- At the lift (6.5), not too small: `nav_link_set_area(nav, pid, seed, Some(r))`, `nav_save_links` (failure as 8.7),
  one step (Some(old), Some(old with r)), the border and the lasso go (`compose(lasso_rect ∪ band ∪ strip)` + `Ui`),
  toast 'New area saved. Undo brings the old one back.' (2500 ms); from the bar, the bar is rebuilt (the reading order
  may change).
- Too small: the toast of 6.5, the mode stays. Cancel: silent, the border goes.

### 8.9 Delete link (PLAN 9.6)

- For each seed: `nav_link_remove` (nav.rs:556); `nav_save_links`; failure: `nav_load`, `migration_reset`, 'Could not
  save the link.'.
- Success: one step with a (Some(old), None) record per link; `s.changed = true`; boxes stale; `panel_stale`; toast
  'Link deleted. Undo brings it back.' (N = 1) or 'Links deleted. Undo brings them back.' (2500 ms); the card and border
  close (8.12); `repaint_panel(Ui)`; from the bar, the bar is rebuilt and redrawn (the empty state when it was the last
  entry).
- A visible border the owner drew is ink and stays (PLAN 6.7).

### 8.10 From the bar (PLAN 7.9, D1)

- **The hold:** in `editor_tick`, when `s.owner == Owner::Bar`, `lb.open`, a chip is pressed, unmoved within the slop,
  and `now - lb.down_at >= LINK_HOLD_MS` (`lb.down_at = now_ms()` at the down, as `link_down_at` is): `bar_hold(a, i)`.
  This replaces phase 1's silent cancel. A pen hold on a chip opens it too (the bar takes pen taps).
- `bar_hold(a, i)`:
  1. `lb.held = i` (the lift then does nothing) and the chip is redrawn uninverted (its rect joins the card's GL16).
  2. The entry's links: the open page's links whose box carries entry number i + 1 in the box cache (`LinkBoxes.entry`,
     PHASE1 12), in reading order. These are the N the row counts (D1); dormant links are not among them.
  3. **Peeking:** clear the peek to the origin without closing the bar (the Back chip's work, PHASE1 13.7), draw the
     entry's borders into the composed page area, and present the page area by one GL16 (`app_invalidate(area, Ui)`,
     `app_flush`): the links live on the origin (PLAN 7.9). **Not peeking:** the borders by DU strips as 8.2 step 5
     when the entry has at most 3 boxes; with more, one GL16 of the strips' bounding rect, flushed on its own (brief
     decision: 4N DU presents for a long ×N row would queue in the driver).
  4. `linkcard_set(.., seeds, readonly, from_bar = true)`; N = 1 gives the page card's rows exactly; N > 1 the ×N rows
     (8.3). A missing entry ('Page not here') gives Change where it goes... and Delete (PLAN 7.2).
  5. `linkcard_place_above`; draw; `app_invalidate(card.rect ∪ chip, Ui)`.
- The bar stays open under the card; closing the card returns to the bar. Every action rebuilds the bar
  (`linkbar_build` with the same origin, `linkbar_layout`) and recomposes `old bar rect ∪ new bar rect` with `Ui`.

### 8.11 Menus, keys and gestures while the card or Link to... is up

`editor_shortcut_in` (editor.rs:3068), before its `front_menu` logic:
- **Card up:** with Rename's keyboard open, refuse everything. `close-menu` or `go-back`: close the card (from the bar,
  back to the bar) and return true. Another menu action: close the card, then continue (so with the bar under it, the
  bar's own rules apply: `links` cancels the bar, another menu commits nothing (no peek is active) and swaps). Any
  other action: refused (`sheet_open`).
- **Making in Pick or Naming state:** with a keyboard open, refuse everything. `add-link`, `close-menu` or `go-back`:
  `link_cancel(a, true)`. Another menu action: `link_cancel(a, false)` then continue, which opens that menu. Any other
  action: refused.
- **Armed** (the strip): today's rules: `add-link` cancels (editor.rs:3174), `close-menu` cancels (editor.rs:3034,
  through `open_menu`), and `open_menu` cancels silently before opening anything else.
- `editor_gesture` (phase 1) needs no change: with the card up over the bar, `front_menu` still returns
  `SHORTCUT_LINKS`, so two fingers down are the bar's fixed cancel; `editor_shortcut_in` closes the card first, then
  cancels the bar (D2). With the card up on the page, two fingers down close it and open Pages. Gestures are ignored
  while any keyboard is open (phase 1's rule, now covering `card.kb` and `lm.kb`).
- `front_menu`, `menu_action`, `close_front` and `open_menu` (editor.rs:2956-3020) keep their phase 1 shape; the card
  and Link to... are handled before them as above (brief decision: they have no action of their own).
- Leaving the editor, sleep (`editor_suspend`), `close_notebook` and a rotation close the card silently.

### 8.12 Closing the card

`card.open = false`, `card.border = false`; `compose(card.rect ∪ card.band)`; `app_invalidate(.., Ui)`: one GL16 of
the card and the strips together (gray ink under the strips comes back exactly). From the bar the bar stays.

---------------------------------------------------------------------------------------------------------------

## 9. The Pages view, and Copy page

### 9.1 The hold path retired (`pagesview.rs`, editor.rs; PLAN 9.6)

- Remove `PG_TO_INDEX` and `PG_UNLINK` (pagesview.rs:29-30), their buttons in the picking header
  (pagesview.rs:368-381: nothing is drawn in their place), `PvResult::PickIndex` and `PvResult::Unlink` (keep every
  other discriminant), `PagesView.pick_had` and the `had` parameter of `pagesview_pick` (pagesview.rs:185).
- `editor.picking` reads `new` while the Pages view picks (Choose from the pages...), never `had`.
- `PG_JUMP` while picking: verify what phase 1's commit 19 did (1.1). Phase 2's rule (brief decision): while picking,
  Go to page... is not drawn: Link to...'s Search does that job, and the retired number pad never opens.
- In editor.rs: `link_hold`'s picker path, the `PickIndex` and `Unlink` arms of `pages_event`, and the toasts only they
  used (14.3) go.

### 9.2 Copy page keeps links (PLAN 9.9)

In `pages_copy` (editor.rs:2460):
1. Before `store_duplicate_page`, `s.page_at(index)`: the source page is in memory and migrated (5.2). A failure to
   read it leaves its links unmigrated; copy them anyway (a legacy link finds the copy's ink: same strokes, same
   seeds).
2. After the copy succeeded, the copy's pid is `nb.page_ids[index + 1]`: `nav_link_copy(nav, src_pid, copy_pid)`; when
   it copied any, `nav_save_links`.
3. A limit or save failure: `nav_load`, `migration_reset`, toast 'The copy has no links: they could not be saved.'
   (3000 ms); the page copy stays.
4. Not an undo step. Links to the copied page are not changed.

---------------------------------------------------------------------------------------------------------------

## 10. E-ink refresh choices (no new display call, no new wait)

`Fast` = DU (188 ms, 294 cold), `Ui` = GL16 (271 ms), `Full` = GC16 (341 ms). Screens only `app_invalidate`;
`app_flush` presents once per loop pass with the strongest mode over one union, so a separate flush is stated where
two must not merge.

| Surface | Waveform | Rect | Notes |
|---|---|---|---|
| Strip opens / closes | Ui | the strip (and the bar's rect when the bar folded into it) | no flash |
| Lasso while drawing | Fast per input batch | each segment's box | never pen mode, never live ink; pen down: DU only |
| Lasso too small | Ui | the lasso's rect | at the lift (pen up), then the toast |
| Lasso lift (new link) | Ui, own flush | lasso rect ∪ strip ∪ dotted rectangle | then Link to... |
| Link to... opens / closes | Ui | its card (plus the rectangle band at a close) | |
| Link to... pager | Ui | the chip grid | |
| Chip or row press | DU immediate (`app_press_feedback`) | the chip or row | outside the accumulator |
| Find PICK typing | phase 1's split host: Fast field, flush; Fast rows, flush; Fast key; one Ui 900 ms after typing pauses | | |
| Find PICK or New page... opens / back to Link to... | Ui | keyboard ∪ rows ∪ Link to... | |
| Choose from the pages... | Full (the Pages view), Full back | whole screen | existing |
| Pick done | Ui | rectangle band ∪ Link to... (∪ the bar coming back) | |
| Card opens from the page | 4 x Fast, each flushed on its own, then Ui | the four strips, then the card | the strips never merge into a union over gray ink |
| Card opens from the bar, peeking | Ui, own flush, then Ui | the page area with its borders, then the card ∪ chip | no DU over a peek's ghosting |
| Card opens from the bar, not peeking | 4 x Fast per box (at most 3 boxes) or one Ui of the strips' bounds, then Ui | | |
| Card closes | Ui | card ∪ strips | |
| Rename keyboard | the keyboard host (field then key, Fast; Ui after 900 ms) | | docked away from the border |
| Link undo / redo, Areas on | Ui | union of the records' boxes, before and after | |
| Link undo / redo, Areas off | toast only | | nothing visible changes |
| Migration | nothing | | invisible |

- Never a GL16 or a toast while the pen is down: the lasso's GL16 comes at PenUp, as the old flow's Pages view did.
- The every-30th-Ui promotion to Full (app.rs `app_flush`) stays.

---------------------------------------------------------------------------------------------------------------

## 11. The runner and docs/SIM.md (no new commands, PLAN 15.3)

### 11.1 New probes (`editor_probe`, editor.rs:4194; the card's through `linkcard_probe`, Link to...'s through `linkbar_probe`)

| Key | Value |
|---|---|
| `editor.link.N.area` | the open page's link N's area, `x y w h` in page px, empty for none. Test the `.area` suffix BEFORE the existing `link.` arm (editor.rs:4243): its `atoi` would read `1.area` as 1 |
| `editor.strip` | `0`, `new` or `redraw` while the strip is up (armed or drawing) |
| `editor.lasso` | how many points the lasso being drawn holds, `0` otherwise |
| `editor.link_area` | the area a new link waits to be given a target for, `x y w h` page px, empty otherwise |
| `editor.linkto` | `0` or `pick` (Link to... up, or hidden under Find PICK, New page... or the Pages view) |
| `editor.linkto.rect` | its card `x y w h`, empty when closed |
| `editor.linkto.dock` | `top` or `bottom`, empty when closed |
| `editor.linkto.chips` | how many chips, all screens |
| `editor.linkto.N` | chip N from 1: `new`, `index`, `recent p<page number>`, `p<page number>` (an Index entry), then ` current` on the marked target; empty past the last |
| `editor.linkto.screen` | `S/T`, the pager's screen and screens |
| `editor.link_card` | `0` or `1` |
| `editor.link_card.rows` | the row names in order, space separated (`rename retarget redraw delete`), a disabled one with a leading `-`; `readonly` for the read-only card |
| `editor.link_card.title` | the title as drawn (`Árbol`, `Page 2`, `The Index`, `Page not here`) |
| `editor.link_card.rect` | `x y w h`, empty when closed |
| `editor.link_card.links` | how many links it acts on (N) |
| `editor.link_card.border` | how many boxes carry the heavy border now, `0` for none |
| `editor.ui_at.X.Y` | the name of the hit at screen point (X, Y) on the card, Link to... or the strip, empty for none: `card.rename`, `card.retarget`, `card.redraw`, `card.delete`, `linkto.chip.N` (N from 1 across screens), `linkto.search`, `linkto.pages`, `linkto.cancel`, `linkto.prev`, `linkto.next`, `strip.cancel`. The way to check every raw tap on them (as `editor.panel_at` does for the panel) |
| `editor.recent` | the Recent targets as page numbers, newest first (`3 2`), empty for none |
| `editor.undo_kind`, `editor.redo_kind` | the kind of the step an undo (redo) would apply: `add`, `erase`, `paper`, `clear`, `link`; empty for none |
| `editor.migrate_left` | pages of the open notebook with a link without an area, not tried this session |
| `editor.links_dirty` | `1` while migrated areas wait for a save, else `0` |
| `editor.kb_dock` | `top` or `bottom` for the keyboard in front (Find, Rename, New page...), empty for none |

### 11.2 Existing probes that gain values

| Key | Change |
|---|---|
| `editor.modal` | gains `card` and `linkto`; the order is `find`, `card`, `linkto`, `links`, then phase 1's; every existing value keeps its meaning |
| `editor.find` | gains `pick` |
| `editor.keyboard`, `editor.text` | also answer for Rename's (`Name in the Index`) and New page...'s (`Name the new page`) keyboards |

### 11.3 Existing probes whose meaning changes on purpose

| Key | Was | Now |
|---|---|---|
| `editor.links` | `N M`: links, and links with their ink on the page | `N M`: links, and links with a box (an area, or ink for one without) |
| `editor.link_mode` | `1` after Add a link until the next ink stroke or a page change | `1` while the strip is up (armed or drawing the lasso) |
| `editor.picking` | `new` or `had` | `new` only (Choose from the pages...) |

### 11.4 SIM.md edits

- The Checks table gets every key above, and the changed rows.
- A paragraph "Making and holding links": the strip, the lasso drawn with `pen line`, Link to...'s dock rule (the area's
  centre in the lower 45% docks it at the top), the formula for its chip and tool-row coordinates (6.6), that a scenario
  checks each raw tap with `editor.ui_at.X.Y`, and that `touch down X Y`, `wait 800`, `touch up X Y` opens the card.
- The keyboard coordinates paragraph gets the top dock's shift (1330 px upright, 7.1).
- `tests/selftest/pass-rust.qs`: `expect editor.link_card is 0`, `expect editor.linkto is 0`, `expect editor.lasso is 0`,
  `expect editor.strip is 0`, `expect editor.links_dirty is 0`, and once a notebook is open
  `expect editor.ui_at.900.900 is` (empty). `tests/selftest/fail-rust.qs`: `expect editor.link_card is 1`,
  `expect editor.linkto is pick`.

No new `GRIMOIRE_*` hook is needed: the failed-save cases use the existing `storage full` / `storage ok` commands
(docs/SIM.md:86).

---------------------------------------------------------------------------------------------------------------

## 12. Unit tests

Run with `cargo test -- --test-threads=1`, host and `--features sim`. New files are picked up (`autotests = true`).
Follow `rust/tests/test_pensync.rs`: `mod testlib;`, `use grimoire::prelude::*;`, plain fns, one `#[test]` with
`run!(..)` lines.

### 12.1 `rust/tests/test_nav.rs` (new cases)

- **Round trip:** links with and without areas save and load back equal (target, area, `has_area`); a file with no
  areas is byte-identical to today's (the existing expectation at test_nav.rs:260 stays); the area line follows its
  link line.
- **An f9d13f5 file** (link lines only, today's bytes) reads unchanged, every link legacy.
- **Area lines:** the last line for a pid and seed wins; an area line before its link line attaches; malformed lines
  (`area.7=beef,-1,0,5,5`, `area.7=beef,0,0,0,5`, `area.7=beef,5000,0,1001,5`, `area.7=beef,1,2,3`, `area.7=zz,1,2,3,4`,
  five digits, a space) are carried verbatim and their link falls back to its ink box; an orphan area is dropped at the
  next save and `areas_dropped` counts it; more than 4000 area lines make the file not read.
- **Caps:** a 4000-link file with every area (about 330 KB) reads and saves under 512 KiB; nav.ini keeps 256 KiB; a
  links.ini over 512 KiB does not read and is never overwritten.
- **Hit test:** an area beats the ink box of the same link; nested areas: the smaller wins whichever was made first; a
  tie goes to the lower seed; a tiny area is padded to `min_px`; an area emptied by a smaller page falls back to ink.
- **Migration:** the area is the UNPADDED ink box, rounded outward (a 1-point stroke gives a 1 x 1 area, never the 7 mm
  box); erase strokes with the link's seed are ignored; a ring covered by precise-eraser strokes still migrates; a
  dormant link (no ink) stays legacy; a read-only `Nav` (`links_writable` false) migrates nothing and returns -1; the
  padded box of a migrated area is within 1 px of the ink box's (`link_box`); 64 links on a dense page migrate in one
  pass (count the passes with a test-only counter, or assert the result equals the per-link result).
- **API:** `nav_link_set` keeps the area of an existing link; `nav_link_remove` takes the area with it; `nav_link_put`
  restores a removed link whole and obeys the per-page limit; `nav_link_copy` copies seeds, targets and areas and copies
  nothing past a limit.
- **PDF:** extend `pdf_has_the_outline_and_links` (test_nav.rs:385): a link with an area gets a /Link /Rect equal to its
  padded area (page px scaled as today).
- Change the comment of the "drawn last wins" block (test_nav.rs:207) to "the smaller box wins" and add a case where
  the later box is the bigger one.

### 12.2 `rust/tests/test_history.rs` (new)

- Add, Erase, Clear and Paper behave exactly as today (port `ownership_follows_the_page`'s checks, history.rs:283).
- A Link step: `history_undo_kind` is `Link`; `history_undo` moves it to redo with an empty dirty rect and leaves the page
  untouched; `history_moved_link(h, true)` returns its records; redo moves it back.
- Interleaved with ink: Add, Link, Add; three undos come off in reverse order, three redos in order.
- A group of N records stays one step.
- Eviction: at `limit` 2, a third push drops the oldest Link step and its records.
- `history_drop_last_redo` / `_undo` remove only the top step; `history_push_link` with no records pushes nothing;
  a push clears redo.

### 12.3 `rust/tests/test_linkcard.rs` (new)

- `linkcard_place`: right when it fits; left when the box is at the right edge; below and above near the side edges;
  never covering the anchor when any placement avoids it; sliding off the panel; never meeting the corner rect
  (including a grown corner); sideways 2480 x 1860; an anchor covering most of the screen (the fallback).
- `linkcard_place_above`: in the chip's column, clamped to the margins, upright and sideways, with the bar's three
  heights.
- `linkcard_set`: the rows of every case in 8.3 (named, unnamed with its page number, the Index, missing, nav.ini
  unwritable, N > 1, N > 1 to the Index, read-only).
- `linkcard_strips`: four rects that do not overlap and together cover exactly what `linkcard_border` paints (render
  into a canvas and compare).
- The card's height for 2, 3 and 4 rows, the ×N line, and read-only (8.3).

### 12.4 Other tests

- `rust/tests/test_linkbar.rs`: `linkbar_build_pick` order (New page..., The Index, recents, entries); recents leave out
  the open page, `index`, missing pages and repeats, at most 4; `current` marks its chip and the pager starts on its
  screen; `no_pages` drops Choose from the pages...; `linkbar_layout_pick` heights for R = 1..3 upright and 1..2
  sideways, the dock rule at 0.55, the top dock below a given corner.
- canvas: a `canvas_dash_poly` test: dash and gap lengths along a polyline with corners; a polyline drawn in two pieces
  with the returned phase equals the whole one pixel for pixel; clipping; only gray 0 and 255.
- keyboard: `modal_rect` with `dock_y = 0` equals today's rect for 1860 x 2480, 2480 x 1860 and 1986 x 2648; with a
  dock its top is `dock_y`.
- editor.rs `#[cfg(test)] mod tests`: the lasso decimation keeps at most 4096 points and the box of the kept points
  equals the box of all points for a ring; the too-small rule at 5 mm both ways.
- `test_i18n.rs`: the new strings fit their budgets in EN and ES (section 14).
- `test_probe.rs`: no settings key is added (bit 1 of `hints` only).

---------------------------------------------------------------------------------------------------------------

## 13. Scenarios

Every new file starts with `# rust-only`. Names sort as listed. A `# root: NAME` must sort before its user.
Notes on the steps:
- Raw taps on the card, Link to... and the strip are checked first with `expect editor.ui_at.X.Y is NAME` (the
  coordinates below follow 6.6 and 8.4; confirm each on the first run and keep it).
- Link to... docked at the bottom upright: the tool row is at y 2300-2440 (centre 2370), Search at x 330, Choose from
  the pages... at 930, Cancel at 1530; chips sit at x 479 and 1381, their rows starting at `2460 - (316 + 140 R) + 152`.
- Contact-level touch after pen input waits out the pen window (PHASE1 6.1).

### 13.1 New scenarios

**`z_link_0_migrate`** (`# copy: tests/fixtures/beta5-links .`, `# settings: mirror_pdf=0`)
- Open the notebook (`tap 200 400`): `expect editor.page is 1`, `expect editor.strokes` is the fixture's count (unchanged),
  `expect editor.links is 4 3` (D is dormant), `expect editor.link.N.area` is link A's unpadded box
  (`300 1000 600 200`, confirm) for the N whose `editor.link.N` is page 3's target (links.ini orders a page's links by
  seed: record N on the first run), `expect editor.links_dirty is 1`, `expect editor.migrate_left is 1` (page 4; page 1
  was tried at its load, D included).
- `expect main.longest_unpumped_ms not -1`; `wait 1500`: `expect editor.migrate_left is 0`,
  `expect main.longest_unpumped_ms is ok`.
- `wait 2500`: `expect editor.links_dirty is 0` (the idle flush).
- Erase ring A whole with the stroke eraser (`pentap panel eraser`, a `pen line` across it): `expect editor.strokes`
  one less; `pen leave`, `wait 1100`; `tap 600 1100`: `expect editor.page is 3` (it still follows); back.
- `tap 1250 1100` (ring C, hidden by the precise eraser): `expect editor.modal is pages`, `expect editor.pages_tab is index`
  (a link to the Index still works); close it.
- Copy page: the Pages view, tick page 1, Copy page: the copy opens: `expect editor.links is 4 3`,
  `expect editor.link.N.area` (the same N: the copy keeps the seeds) equals the original's; back.
- Page 4 (never opened before the idle ticks): tap inside the small ring: `expect editor.page is 3` (smallest wins;
  beta.5 went to page 2).
- `pentap panel library`, `waitbg`.

**`z_link_0b_reopen`** (`# root: z_link_0_migrate`)
- `tap 200 400`: `expect editor.links_dirty is 0`, `expect editor.link.N.area` as before, `expect editor.migrate_left is 0`
  (the areas were saved, so nothing is left to migrate).

**`z_link_1_make`** (`# root: z_bar_0_make`, `# env: GRIMOIRE_CHECK_FIT=1`), the rewritten heart of `z_index_2_links`
- `tap 200 400`. `expect editor.strokes` (record S). `key 38 0` (add-link): `expect editor.strip is new`,
  `expect editor.link_mode is 1`, `shot 01-strip`.
- `pentap 600 1700`: `expect toast is Too small for a link: draw around what links.`, `expect editor.strip is new`,
  `expect editor.strokes is S`, `expect editor.live_matches_render is 1`.
- `pen line 1000 600 1500 800 30 0.6 0.6`: `expect editor.strokes is S` (no ink), `expect display.pen_mode is 0`,
  `expect display.last_fast not ink`, `expect editor.linkto is pick`, `expect editor.linkto.dock is bottom`,
  `expect editor.link_area is 1000 600 500 200` (confirm), `shot 02-linkto`.
- Pick the Árbol chip (check with `editor.ui_at`): `expect toast is Linked to Árbol.` (the root's settings.ini already
  has `hints` bit 1: z_bar_0_make's first link showed the tip, 13.2), `expect editor.links is 4 4`,
  `expect editor.undo_kind is link`, `expect editor.strip is 0`.
- `pen leave`, `wait 1100`, `tap 1250 700`: `expect editor.page is 3`; back with `key 48 0`.
- Undo: `pentap panel undo`: `expect editor.links is 3 3`, `expect toast is Link change undone.`; redo: `4 4`,
  `Link change redone.`.
- A lasso low on the page (`pen line 300 2000 900 2150 ...` after `key 38 0`): `expect editor.linkto.dock is top`,
  `expect editor.linkto.rect` with a top of 150 (below the corner zone). Cancel: `expect toast is No link made.`,
  `expect editor.links is 4 4`.
- Zoomed: pinch in (legacy `pinch`), `key 38 0`, a `pen line` on screen: `expect editor.link_area` is the page rect of
  that screen line (through the view), then Cancel.
- Choose from the pages...: `key 38 0`, a lasso, `tap 930 2370`: `expect editor.modal is pages`,
  `expect editor.picking is new`; `tap 703 340` (page 2's tile): `expect toast is Linked to page 2.`,
  `expect editor.modal is none`.
- Read-only and unreadable refusals are unit-level (no portable fixture), except the 64-link toast if a fixture with
  64 links on a page is cheap to make (optional).
- Sideways (`rotate 1`): the strip, a lasso, Link to... (`shot 03-linkto-sideways`).

**`z_link_1b_es`** (`# root: z_bar_0_make`, `# env: GRIMOIRE_LANG=es GRIMOIRE_CHECK_FIT=1`)
- The strip, Link to..., Find PICK, the card with 4 rows and the ×N card, upright and sideways, shots.

**`z_link_2_newpage`** (`# root: z_bar_0_make`)
- `key 38 0`, a lasso, the New page... chip: `expect editor.keyboard is Name the new page`; type `Ideas` (SIM.md's
  keys), Done: `expect editor.pages is 6`, `expect editor.page is 1` (stayed), `expect toast is Linked to the new page 6.`,
  `expect editor.links is 4 4`, `expect editor.index_rows is 3`; `pen leave`, `wait 1100`, a finger tap inside the new
  area: `expect editor.page is 6`, `expect editor.page_name is Ideas`.
- New page... then Cancel: back to Link to... (`expect editor.linkto is pick`), `expect editor.pages is 6`; Cancel.

**`z_link_3_find_pick`** (`# root: z_bar_0_make`)
- `key 38 0`, a lasso, Search (`tap 330 2370`): `expect editor.find is pick`, `expect editor.kb_dock is bottom`,
  `expect keyboard.draws` within its bound.
- Type `arb`: `expect editor.find.1 is Árbol`; tap the row: the link exists (`expect editor.links is 4 4`),
  `expect editor.page is 1`, `expect editor.back is 0 0` (no jump).
- Again with `2`: `expect editor.find.1 is Link to page 2`.
- Search then Cancel: `expect editor.find is 0`, `expect editor.linkto is pick`.
- A low lasso: Search docks at the top (`expect editor.kb_dock is top`), typed with the shifted coordinates (7.1).

**`z_link_4_undo`** (`# root: z_bar_0_make`)
- Hold the link to page 2 (`touch down 600 1450`, `wait 800`, `touch up 600 1450`), Delete: `expect editor.links is 2 2`,
  `expect toast is Link deleted. Undo brings it back.`.
- `pentap panel undo`: `expect editor.links is 3 3`; `pen leave`, `wait 1100`, `tap 600 1450`: `expect editor.page is 2`
  (it goes where it went); back; redo: `2 2`.
- Interleaved: `pen line` (ink), then delete the link at 600 1100 through the card; undo: the link is back; undo: the
  ink is gone
  (`expect editor.strokes`), links unchanged.
- `storage full`; delete a link: `expect toast is Could not save the link.`, links unchanged, `expect editor.undo_kind`
  unchanged; `storage ok`; delete it; `storage full`; undo: `expect toast is Could not save the links: that change was
  not undone.`, `expect editor.links` unchanged, `expect editor.redo_kind is` (empty: the step was dropped);
  `storage ok`.
- The eye on (`key 30 0`), undo a link step: no toast, and `display.presents` shows one `ui` inside the boxes' union.

**`z_link_5_card`** (`# root: z_bar_0_make`, `# env: GRIMOIRE_CHECK_FIT=1`)
- `expect display.presents not -1`; `touch down 600 1100`, `wait 800`: `expect editor.link_card is 1`,
  `expect editor.modal is card`, `expect editor.link_card.title is Árbol`,
  `expect editor.link_card.rows is rename retarget redraw delete`, `expect editor.link_card.border is 1`; `display.presents`
  holds four `fast` strips, then one `ui` of the card rect, and no `full`; `touch up 600 1100`: `expect editor.page is 1`,
  card still open. `shot 01-card` (the border, the card and the page around them).
- Rename: `editor.ui_at` check, tap `card.rename`: `expect editor.keyboard is Name in the Index`,
  `expect editor.text is Árbol`, `expect editor.kb_dock is bottom`; delete five letters, type `Cap`, Done:
  `expect toast is Page 3 is in the Index as Cap.`, `expect editor.link_card.title is Cap`, `expect editor.link_card is 1`.
- A tap outside (`tap 1200 1900`): `expect editor.link_card is 0`, `expect editor.page is 1` (no follow).
- The corner: hold again, `tap 60 60`: `settings.status_on` toggled and `expect editor.link_card is 1`.
- Change where it goes: `expect editor.linkto is pick`, `expect editor.ui_at.930.2370` is not `linkto.pages` (two
  buttons only: Search and Cancel), `expect editor.modal` never reads `pages`, the chip of Cap reads
  `... current`; pick The Index: `expect toast is Linked to the Index.`, and a finger tap at 600 1100 then opens the
  Pages view's Index tab; close it; undo the retarget.
- An unnamed target (hold the link at 600 1450, to page 2): `expect editor.link_card.rows is rename retarget redraw delete` and its Rename
  row label is Add page 2 to the Index... (`shot 02-card-unnamed`); name it `Two`:
  `expect toast is Page 2 is in the Index as Two.`, `expect editor.index_rows is 3`.
- Redraw (hold the link at 600 1450): `expect editor.strip is redraw`, `expect editor.link_card.border is 1` (the old
  area stays shown); `pen line 1000 1000 1400 1100 ...`: that link's `editor.link.N.area` (its N recorded as above) is
  the new box, `expect toast is New area saved. Undo brings the old one back.`; undo: the old area.
- Delete then undo: covered by z_link_4; here once with the pen on the card rows (`pentap`).
- Placement shots: links near the right edge, the bottom and the top (lassos made in this scenario), upright and
  sideways (`shot 03..08`), each with `expect editor.link_card.rect` recorded.
- A pen stroke outside closes the card and draws nothing: `expect editor.strokes` unchanged, `expect display.ink is 0`.

**`z_link_6_smallest`** (`# root: z_bar_0_make`)
- Make a big link around (200, 900)-(1600, 1300) to page 5 AFTER the existing link at (300, 1000)-(900, 1200) (inside it): a tap at
  600 1100 goes to page 3 (the smaller); a tap at 1500 1250 goes to page 5.

**`z_bar_10_card`** (`# root: z_bar_0_make`)
- Open the bar; hold entry 1 (the ×2 entry, `touch down` on its chip, `wait 800`, `touch up`):
  `expect editor.link_card is 1`, `expect editor.link_card.links is 2`, `expect editor.link_card.rows is rename retarget
  delete`, `expect editor.link_card.border is 2`, `expect editor.bar is open`.
- Delete 2 links: `expect editor.links is 1 1`, `expect editor.bar.entries is 1`, the toast; close the bar; one undo:
  `expect editor.links is 3 3`.
- Peek entry 2, then hold entry 1: `expect editor.bar.peek is 0` (back at the origin), `expect editor.page is 1`, and
  `display.presents` shows the page area's `ui` then the card's, no `fast` strips.
- Change where they go: The Index: both links go to the Index in one step (`editor.link.1`, `editor.link.2`), one undo
  restores both.
- Two fingers down while the card is up over the bar: `expect editor.link_card is 0`, `expect editor.bar is 0`,
  `expect editor.modal is none`.

**`z_link_7_not_ink`** (`# root: z_bar_0_make`)
- During a lasso (`pen down`, two `pen move`s, before `pen up`): `expect display.pen_mode is 0`, `expect display.ink is 0`,
  `expect editor.lasso` greater than 0 (as `not 0`); after `pen up`: `expect display.last_fast not ink`,
  `expect editor.strokes` unchanged. With Live view on (as y_live_ink.qs starts it), the same lasso adds no stroke.

**`y_touch_selftest`** gains one line: `expect editor.ui_at.900.900 is` with no card.

### 13.2 Existing scenarios changed on purpose (each in the commit that changes the behaviour, suite green before and after)

1. **Inked making to ink-free making** (commit 7, then commit 8 inserts Link to...). Find them with
   `grep -lE "key 38 0|tap 1410 2395|picking is|link_mode is" tests/scenarios/*.qs`; at least: z_index_2_links
   (rewritten: making, follow, undo, the card, Choose from the pages...), v_shortcuts_links, g_gest_9_pen_over_link,
   z_back_1_list, y_all_screens (its two link sections), and phase 1's z_bar_0_make, z_bar_8_pager, z_bar_9_corner and
   z_link_areas_1. In each:
   - the lasso adds no stroke: every `expect editor.strokes` after a making drops by one per link;
   - `expect toast is Draw around what links. Then tap where it goes.` becomes `expect editor.strip is new`;
   - after commit 8, `tap 930 2370` (Choose from the pages..., checked with `editor.ui_at`) goes before the tile tap
     that picks, or the tile tap becomes a chip tap;
   - `The link goes to page %d.` / `The link goes to the Index.` become `Linked to page %d.` / `Linked to the Index.` /
     `Linked to %s.`, and the first link made with `hints` bit 1 clear (a fresh root: z_bar_0_make's first link) shows the bit 1 tip
     instead;
   - a hold (`touch down` / `wait 800` / `touch up`) expects the card (`editor.link_card is 1`), never
     `editor.picking is had`;
   - `Link removed. Its ink stays on the page.` becomes the card's Delete and its toast.
2. **`y_all_screens`**: `28-link-pick-new` and `49-link-pick-new-landscape` become the strip and Link to...;
   `28-link-pick-had` and `49-link-pick-had-landscape` become the card; new shots: the ×N card, the read-only card,
   Find PICK docked at the top, New page...'s keyboard; EN and ES, both styles, CHECK_FIT (through l10n-check).
3. **Pages-view hold scenarios**: every `editor.picking is had` and every tap on The Index or Remove link in the Pages
   view's picking header (`tap 1400 95`, `tap 1250 95`, `tap 1630 95` in z_index_2_links and y_all_screens).

### 13.3 Must stay green unchanged

- z_index_1; every phase 1 gesture, bar, Find and eye scenario apart from the four above; every pinch scenario;
  y_live_ink, h_live_vs_settled, live_1_card; the Display and Buttons sheets' scenarios.
- z_link_areas_1's `display.presents` checks (the eye's boxes are now padded areas, within 1 px of the old ink boxes:
  if an exact rect moves by 1 px, record why in the commit message).

### 13.4 Deliberate behaviour changes (list them in the commit messages and the owner notes)

- making a link leaves no ink (the dashed lasso is screen-only);
- Link to... replaces the Pages view as the target chooser (Choose from the pages... keeps it for New link);
- holding a link opens the card over the page; the Pages-view picker, Remove link and The Index button are gone;
- the smallest link under a finger wins (was the one drawn last);
- links.ini gains `area.` lines; beta.5 links are migrated (their ink stays);
- link edits are undoable; Undo can remove a link;
- Copy page copies the page's links;
- `editor.links`, `editor.link_mode` and `editor.picking` (11.3).

---------------------------------------------------------------------------------------------------------------

## 14. UI strings (English / Spanish)

Every row is a new msgid unless marked existing. Add a `// TRANSLATORS:` line with the budget above each:
- toasts: "about 1700 px at 44 px in bold, or two lines";
- card rows: "about 700 px at 44 px";
- Link to... chips: "about 700 px at 44 px in bold" upright, 600 sideways (as the bar's chips);
- Link to... tool words: "about 540 px at 34 px caps, or two lines" (three buttons; 860 with two);
- the strip: "about 1400 px at 44 px in bold";
- keyboard titles: "about 1500 px at 44 px".

### 14.1 New strings

| Where | English | Spanish |
|---|---|---|
| strip | Draw around what links. | Rodea lo que enlaza. |
| strip, redraw | Draw the link's new area. | Dibuja la nueva zona del enlace. |
| Link to... title | Where does the link go? | ¿Adónde va el enlace? |
| Link to... chip | New page... | Página nueva... |
| Link to... tool | Search | Buscar |
| Link to... tool | Choose from the pages... | Elegir entre las páginas... |
| keyboard title | Name the new page | Nombre de la página nueva |
| Find PICK row | Link to page %d | Enlazar con la página %d |
| toast | Linked to %s. | Enlazado con %s. |
| toast | Linked to page %d. | Enlazado con la página %d. |
| toast | Linked to the Index. | Enlazado con el índice. |
| toast | Linked to the new page %d. | Enlazado con la nueva página %d. |
| toast (bit 1) | Linked. Tap it with a finger to go there; hold it for more. | Enlazado. Tócalo con un dedo para ir; mantenlo pulsado para más. |
| refusal | Links could not be read in this notebook. | No se pudieron leer los enlaces de este cuaderno. |
| refusal | This page has %d links: delete one to add another. | Esta página tiene %d enlaces: borra uno para añadir otro. |
| card row | Rename in the Index | Renombrar en el índice |
| card row | Add page %d to the Index... | Añadir la página %d al índice... |
| card row | Change where it goes... | Cambiar a dónde lleva... |
| card row | Redraw the area | Volver a dibujar la zona |
| card row | Delete link | Borrar enlace |
| card row (×N) | Change where they go... | Cambiar a dónde llevan... |
| card row (×N, `ng_`) | Delete %d link / Delete %d links | Borrar %d enlace / Borrar %d enlaces |
| card line (×N) | To change one, hold it on the page. | Para cambiar uno, mantenlo pulsado en la página. |
| keyboard title | Name in the Index | Nombre en el índice |
| toast | Page %d is in the Index as %s. | La página %d está en el índice como %s. |
| toast | Page %d is not in the Index now. | La página %d ya no está en el índice. |
| toast | Link deleted. Undo brings it back. | Enlace borrado. Deshacer lo recupera. |
| toast | Links deleted. Undo brings them back. | Enlaces borrados. Deshacer los recupera. |
| toast | New area saved. Undo brings the old one back. | Zona nueva guardada. Deshacer recupera la anterior. |
| toast | Could not save the links: that change was not undone. | No se pudieron guardar los enlaces: el cambio no se deshizo. |
| toast | Could not save the links: that change was not redone. | No se pudieron guardar los enlaces: el cambio no se rehízo. |
| toast | Link change undone. | Cambio de enlace deshecho. |
| toast | Link change redone. | Cambio de enlace rehecho. |
| toast | The copy has no links: they could not be saved. | La copia no tiene enlaces: no se pudieron guardar. |

Brief additions beyond PLAN-V2's strings (section 18): 'Linked to page %d.', 'Linked to the Index.' (so an unnamed page
reads "page 2", not "Page 2", mid-sentence), the one-msgid bit 1 tip, 'Page %d is not in the Index now.', 'Links deleted.
Undo brings them back.', 'New area saved. Undo brings the old one back.', the redo failure, the two undo/redo toasts,
the Copy page failure and 'Link to page %d'.

### 14.2 Existing strings used (no new msgid)

'Too small for a link: draw around what links.', 'No link made.', 'Could not save the link.', 'Could not save the page
name.', 'Could not add a page', 'Where does the link go? Tap its page.' (Choose from the pages... only), 'The Index',
'Page %d', 'Cancel', the phase 1 strings 'p. %d', '×%d', 'Page not here', 'This page is read-only.', and the read-only
toast `K_READONLY_MSG` (editor.rs:1427).

### 14.3 Retired (no longer used; leave the po entries to `make l10n`)

'Draw around what links. Then tap where it goes.', 'The link goes to page %d.', 'The link goes to the Index.', 'The link
goes to page %d. Tap another to change it.', 'The link goes to the Index. Tap a page to change it.', "The link's page is
not in this notebook. Tap a page to change it.", 'Link removed. Its ink stays on the page.', 'Remove link'.

---------------------------------------------------------------------------------------------------------------

## 15. Docs to update in phase 2

- **docs/FORMAT.md:** the Links section rewritten for v2 (a link is (page, seed, target, area); the `area.` grammar,
  ranges and writer order; the header kept; areas first, ink boxes for links without one; smallest wins; migration
  takes the unpadded ink box and keeps the ink; malformed lines carried, orphans dropped; the 512 KiB cap; a file that
  does not read is never overwritten); the 3.3 rows in "Moving between versions"; `hints` bit 1 in the settings.ini
  section.
- **docs/SIM.md:** section 11.4.
- **docs/DESIGN.md:** the strip, the lasso, Link to..., Find PICK, the card and its placement, the e-ink table of
  section 10.
- **docs/SHORTCUTS.md:** `add-link` no longer waits for ink: it shows the strip, and the next pen stroke draws the area.
- **docs/TESTING.md** and **docs/KINDLE.md:** the owner checks of section 17.
- **po/es-glossary.md:** "zona" for a link's area (check phase 1's entry), "tarjeta" only if a string uses it (none
  does).

---------------------------------------------------------------------------------------------------------------

## 16. Commit plan and the gate

### 16.1 Small logical commits, in this order

Each commit message ends with CLOUD.md's attribution lines (the Co-Authored-By and Claude-Session lines your session's
system reminder gives). Each leaves `make rust-check` green.

0. `fixtures: beta5-links, made by the phase 1 inked flow` (1.3; the generator and its README; no code).
1. `nav: link areas in links.ini (area lines, NAV_LINKS_FILE_MAX, carried and orphan lines)` (3.1-3.4, test_nav
   round trips, FORMAT.md's grammar).
2. `nav: area-first boxes, smallest wins, nav_link_migrate, link records` (3.5, 3.6, test_nav hit test, migration and
   PDF cases).
3. `history: ActKind::Link, link steps in undo, drop-last helpers` (4.1, test_history.rs).
4. `editor: beta.5 links migrate at page load and on idle ticks; links_dirty flushes` (5, z_link_0_migrate without
   its Copy page part, z_link_0b_reopen).
5. `keyboard: a top dock` (7.1, the modal_rect test).
6. `canvas: canvas_dash_poly` (6.4, its test).
7. `editor: making a link leaves no ink (the strip, the dashed lasso, the area, link steps in undo)` (6.1-6.5, 4.2, the
   pick still through the Pages view as today; 13.2 item 1's first round; z_link_7_not_ink; l10n).
8. `linkbar: Link to... (pick mode, Recent targets, New page..., Choose from the pages...)` (6.6-6.11; 13.2 item 1's
   second round; z_link_1_make, z_link_2_newpage, z_link_6_smallest; l10n).
9. `findsheet: PICK mode over the page with the docked keyboard` (7.2, z_link_3_find_pick, l10n).
10. `linkcard.rs: the long-press card (rows, placement, the heavy border)` (8.1, 8.3, 8.4, test_linkcard.rs).
11. `editor: hold a link: the border and the card; Rename, Change where it goes, Redraw, Delete` (8.2, 8.5-8.9, 8.11,
    8.12, z_link_4_undo, z_link_5_card, l10n).
12. `editor, linkbar: hold a bar row: the same card, a ×N row in one step` (8.10, z_bar_10_card).
13. `pagesview, editor: the Pages-view hold path retired` (9.1, 13.2 item 3, z_index_2_links' last hold parts).
14. `editor: Copy page keeps links` (9.2, z_link_0_migrate's Copy page part).
15. `hints: the first-link tip` (bit 1, its strings).
16. `y_all_screens and l10n-check: the strip, Link to..., Find PICK, the card` (13.2 item 2, z_link_1b_es);
    `docs: phase 2` (15).

Push the branch at checkpoints (after commits 4, 9 and 16).

### 16.2 The gate: `make rust-check`

As in phase 1 (PHASE1-BRIEF 22.2): unit tests twice (host, `--features sim`), headless, tools, the armv7 build,
scenarios plus selftest, l10n-check, mirror-cleanup-check, share_check, live_check, bt_check and the exporter. Use
`GRIMOIRE_TEST_PORT=8850`. On this Linux x86_64 machine, Apple-libm float fixtures and outline-font CRCs may differ:
never edit an expectation to pass on Linux; list the differences and why in your report; the Mac gate is
authoritative. `make c-check` skips every `# rust-only` file; count the existing scenarios this phase converts and give
the number in the report (PLAN 20, question 4). Never run a GUI binary, never touch a device.

---------------------------------------------------------------------------------------------------------------

## 17. Owner acceptance on the Kindle (by hand, numbered)

Install the phase 2 build. Test first in the notebooks that have your beta.5 links.

1. **Your beta.5 links.** Open each notebook: every link still goes where it went. Turn on Areas in the Links bar:
   each link's dashed box sits where its ring was, and the ring is still on the page as ink.
2. **Ink is no longer the link.** Erase an old ring completely: a finger tap where it was still follows the link.
3. **Making a link.** In the bar, New link (or your add-link button): a strip at the bottom says "Draw around what
   links.". Circle some words: a dashed line follows the pen with no flash, and when you lift it a dotted rectangle
   stays and "Where does the link go?" opens. No ink is left on the page, ever.
   - Circle low on the page: the chooser opens at the top, below the clock corner.
   - It lists New page..., The Index, the pages you linked or visited recently, then your Index; Search finds any page
     by number or name; Choose from the pages... opens the Pages view.
   - Pick one: "Linked to ..." (the very first time, a tip about tap and hold); from the bar, the bar comes back with
     the new row. One Undo removes the link; Redo brings it back.
   - New page...: name it; a new last page is added and linked, and you stay where you were.
   - A dot is "Too small"; Cancel says "No link made.".
4. **Holding a link.** Hold a finger on a link for about half a second: a heavy dotted border shows exactly what is
   tappable, then a small card opens beside it. The page stays in view the whole time (no Pages view, no full flash).
   - Rename in the Index: a keyboard opens over the page with the border still visible; Done renames that page in the
     Index everywhere. On an unnamed page the row reads "Add page N to the Index...".
   - Change where it goes...: the chooser marks the current target and has no Choose from the pages....
   - Redraw the area: circle the new area; Undo brings the old one back.
   - Delete link: "Link deleted. Undo brings it back."; a border you drew yourself stays.
   - A tap outside the card closes it and does nothing else; a tap in the clock corner toggles the clock and leaves the
     card.
5. **Holding a row in the bar.** The same card, above the bar. A ×2 row deletes or retargets both links in one Undo
   step, and says "To change one, hold it on the page."; while peeking, holding a row first returns to your page.
6. **Nested links.** A small link inside a bigger one: a tap inside the small one follows the small one.
7. **Copy page** keeps the page's links, with the same areas.
8. **Clear page** clears ink only: the links stay (delete them from the card).
9. **Feel.** The border and card appear quickly; circling never lags behind the pen; nothing flashes except a page
   change; writing right after a link is made goes on in fast pen mode.
10. **Going back (optional).** Install the previous build (f9d13f5) again for a moment: your old links still work
    there (through their ink; links made in this build have no ink, so they do nothing there); come back to this
    build: nothing was lost.
11. **Report** any link that goes somewhere else than before, and any place where the card covers what you wanted to
    see.

Known in this build:
- a visible border is freehand ink until phase 4;
- Rename is not undone by Undo; a page made by New page... stays after Undo removes its link.

---------------------------------------------------------------------------------------------------------------

## 18. Brief decisions (fold into PLAN-V2 section 19)

1. links.ini keeps its header line `# Grimoire: links drawn on pages` (PLAN 13.1's example differs): a file with no
   areas stays byte-identical to beta.5's.
2. links.ini gets no version line; new kinds of lines are the version mechanism, carried by every version since beta.5.
3. Area values are 1 to 4 decimal digits; more than 4000 area lines make the file not read; an orphan area is counted
   in `Nav.areas_dropped` and logged once at the next save.
4. Read-only pages are not migrated; their links keep the ink-box fallback.
5. `nav_link_migrate` makes one pass over a page's strokes; the idle migration drops pages it read (not cached) and
   tries each page once per session (`migrate_done`).
6. Link steps move through `history_undo` / `history_redo` like any step (`apply` leaves the page alone) and the editor
   applies their records; the step is dropped when the save fails, undo and redo alike.
7. A link undo or redo with Areas off toasts 'Link change undone.' / 'Link change redone.' (nothing visible changes).
8. The strip is not a sheet: while armed the page pans, zooms, turns and follows links (a page change cancels silently).
9. Link to... is a second `LinkBar` (`EditorState.lt`), so the bar's state and phase 1's `lb.open` meaning stay intact.
10. Link to... docks at the top when the area's centre is at y >= 0.55 of the screen; Find PICK and New page... dock
    with it.
11. Recent targets: this session's picks (up to 8 kept), then the back list, at most 4 chips.
12. A pick of the current target changes nothing; a retarget or redraw cancel is silent.
13. The card stays open after Rename; every other row closes it.
14. The card and Link to... have no action of their own: `editor_shortcut_in` closes or cancels them before its menu
    logic, and `front_menu` keeps phase 1's shape.
15. A bar row hold with more than 3 boxes shows the borders by one GL16 instead of 4N DU strips; while peeking, the
    borders ride on the page area's GL16.
16. While picking in the Pages view, Go to page... is not drawn.
17. New owners: `Lasso = 13`, `Card = 14`, `LinkTo = 15`, `Strip = 16`.
18. No new runner command: raw taps on the new surfaces are checked with `editor.ui_at.X.Y`; the probes of section 11
    extend PLAN 15.3's list.
19. Toasts: 'Linked to page %d.' and 'Linked to the Index.' beside 'Linked to %s.'; the first-link tip is one msgid
    shown instead of the 'Linked to' toast; 'Page %d is not in the Index now.' for an emptied name; the strings of 14.1
    marked as brief additions.
20. The fixture for migration is made once by the phase 1 build's inked flow (a real device copy only if the owner's
    session provides one).
