# The Index hook: PLAN V2 (Rust)

The complete revised plan for the Index hook in the Rust app. It replaces
`index-hook-final-plan-2026-10-27.json` (the synthesized plan, written against the C app at 89893aa) wherever they
differ, and keeps everything from it that no ruling changed, translated to the Rust modules in `rust/src/`.

- Binding sources: `docs/index-hook/README.md` (the owner's brief, rulings R1 and R2, decisions D1-D8, the six phases),
  `docs/index-hook/CLOUD.md` (rules for cloud sessions), `rust/PORTING.md` (the contract).
- Folded in: `index-hook-shape-gestures-2026-10-27.json` (the recorded one-finger shape gesture subsystem and the
  sideways "fit the whole page" setting). Part G of this plan is the integrated gesture design.
- Companion: `docs/index-hook/PHASE1-BRIEF.md`, the self-contained implementation brief for phase 1.
- Line numbers are Rust lines at `claude/rust-index-base` (17cbfbc) unless a C file is named. They drift; the
  function names are the stable reference.
- Units: 300 dpi, 11.8 px per mm, logical (rotated) screen px. Text 44 px, titles 64 px, small text 34 px. Upright
  1860 x 2480, sideways 2480 x 1860.
- Thresholds marked (prov.) ship provisional in phase 1 and are frozen in phase 3 from the phase 1 touch traces.
- Strings: every new UI string is given as 'English' / 'Spanish' where it is defined (its home section here; for
  phase 1 also the table in PHASE1-BRIEF.md 16); a later mention, and the owner's acceptance steps, name only the
  English. "Existing msgid" marks a string the app already has, with its translation unchanged.

---------------------------------------------------------------------------------------------------------------

## 0. In one paragraph

After this work, two fingers moved up on the page bring up the Links bar along the bottom. It lists the Index
entries this page links to, numbered in reading order. Tap an entry and the page above the bar becomes that page at
once, with no flash; tap the page to stay there, or close the bar to go back to where you started. Hold a link on
the page (or an entry in the bar) and a dotted border shows exactly what counts as tappable, while a small card
beside it offers Rename, Change where it goes, Redraw the area and Delete link; the page stays in view the whole
time. Making a link no longer leaves ink: you circle the words, a dashed line follows the pen but is never ink, and
you pick where it goes. Your beta.5 links keep working: each one's circle becomes its tappable area, and the circle
stays on the page as ordinary ink. Two fingers down open the Pages view; two fingers left or right turn the page
(never adding one). Sideways, the whole page now fits by default, so the two-finger swipes work in both
orientations; where the page can scroll, two fingers pan. Settings gets a Gestures sheet between Controller and
Updates: record a two-finger swipe or tap, a pen-button click, or a shape you draw with one finger, for any action;
every toggle and important setting is an action. Smart shapes snap a box, circle, triangle or line only when you
hold the pen still and the shape really is one, and one Undo removes it. The Index button opens a map of the whole
notebook as dots: well-linked pages solid, lightly linked ones dithered, unlinked ones hollow. It ships in six
Kindle builds, and the owner tests each one.

---------------------------------------------------------------------------------------------------------------

## 1. The rulings, and where each is applied

Each ruling is quoted from README.md, then where this plan applies it. Nothing here re-opens a ruling.

### R1. Making a link no longer leaves ink

> A link's area is an invisible region. The bar's eye toggle shows it (off by default), and so does making the link.
> A visible border is optional ordinary ink, drawn with smart shapes. Existing beta.5 inked-lasso links migrate:
> their stroke box becomes the area, and their ink stays on the page as ink. links.ini is new in the unreleased
> beta.5, so it may change.

- Model: a link is (page id, seed, target, area); the area is stored on an `area.` line in links.ini and is never
  drawn into `page_canvas`, thumbnails, the share page, the PDF page image or the sleep image (section 9.1, 13.1).
- The eye (`link_areas`, default 0) shows areas (section 9.4, phase 1 with legacy ink boxes, stored areas from
  phase 2). Making a link shows its area: the dashed lasso, then the dotted rectangle until the pick (section 9.3).
- A visible border is ordinary ink: freehand until phase 4, a smart shape from phase 4 (sections 6.7, 10). The
  border is never given the link's seed.
- Migration: on page load and on idle ticks, a legacy link's area = the UNPADDED point box of its ink strokes, erase
  strokes ignored; the lasso ink stays as ordinary ink; flushed by an explicit `nav_save_links` (section 9.8,
  phase 2).
- links.ini v2 grammar (section 13.1, phase 2).
- Phase note, said plainly: in phase 1 the Links bar's New link calls today's `link_start` (editor.rs:2258, the
  inked lasso) as a stopgap; phase 2 replaces it with ink-free making and removes `link_box_drawn` and
  `link_cancel`'s undo-the-box path. Inked making never ships past phase 2.

### R2. Long-press a link on the page

> A dotted border shows its bounds. A small card beside it offers Delete link and Rename. The plan also adds Change
> where it goes and Redraw area. The notebook page stays in view: no switch to the Pages view. The bar's long press
> uses the same card.

- `link_hold` (editor.rs:2439, today the Pages-view picker) is replaced in phase 2 by the heavy dotted border (DU
  strips) plus the link card, with the page in view (section 9.5). The Pages-view hold path (`link_hold` ->
  `pagesview_pick`, PG_UNLINK, PG_TO_INDEX) is retired in phase 2.
- Card rows, in this order: Rename in the Index (or 'Add page %d to the Index...'), Change where it goes..., Redraw
  the area, Delete link (destructive last).
- The bar's row hold opens the same card (section 7.9, phase 2). In phase 1, before the card exists, a row hold is
  not wired (it cancels the press silently), so phase 1 never introduces a new jump to the Pages view. The page's
  own hold keeps today's behaviour in phase 1 only.
- Card placement never covers the clock corner zone (section 3.2).
- From the card, Change where it goes stays on the page: Link to... opened from the card hides 'Choose from the
  pages...' (Search and the Index chips reach every target), so no card flow ever shows the Pages view (section 9.6).

### D1. Links bar rows

> One row per Index entry the page links to (two links to one entry show once, marked xN). Holding a row renames the
> entry, or deletes/retargets all its links on this page in one undoable step. Holding a link on the page acts on
> that one link.

- Grouped rows only (the synthesized plan's per-link alternative is removed): section 7.2.
- Row hold, entry with N > 1 links: card rows Rename (the entry, per D3), 'Change where they go...', 'Delete %d
  links', plus the line 'To change one, hold it on the page.'; Redraw the area is absent (it is per link). Delete
  and retarget are one `Link` history step holding N records (section 9.7). Entry with one link: the page card's
  rows exactly.
- A hold on the page acts on that one link (section 9.5).

### D2. While hopping

> While hopping, the bar keeps listing the page you started on. Tap the page to stay. X, two fingers down or Back
> return to the start.

- Peek is a display state; the chips stay the origin's links (section 7.4).
- Tap the page to stay = a finger lift within the 30 px slop outside the bar (section 7.5). A tap in the clock
  corner zone toggles the clock and does NOT commit (section 3.2).
- Return to the start: X, two fingers down (always, whatever the bindings: G6.2 makes `touch.2.down` a fixed
  cancel while the bar is up), the Back key, plus the synthesized plan's superset: the links action or the Links
  button again, a grip drag down, Close menu (section 7.6).
- A finger tap on the panel while the bar is up stays (commits) and presses nothing, except a lift on the Links
  button, which is "the Links button again" and returns to the start (section 7.5).
- From phase 3 a finger that moves past the finger capture's start gate over the page while the bar is up (about 6
  mm in the recorded design; the value is fingershape's `FS_GATES`) is a finger shape, not a commit (G8.7).

### D3. Rename

> Rename renames the target page's Index name everywhere. On an unnamed page it reads 'Add page N to the Index...'.

- Card row 'Rename in the Index' / 'Renombrar en el índice' runs the keyboard host and `nav_set_name(nav, target_pid,
  name)` (nav.rs:171 already takes a pid) plus `nav_save`; on an unnamed target 'Add page %d to the Index...' /
  'Añadir la página %d al índice...'; toast 'Page %d is in the Index as %s.' (section 9.5).
- The Index map's Rename does the same (section 11.6).

### D4. Gestures are recorded

> i wanted more of a gesture system where a user can record a basic shape or pattern and use them anywhere. A shape
> drawn with ONE FINGER anywhere on the page, at any zoom, is a gesture. The user records a shape (2-3 samples) for
> any action. Multi-finger swipes and taps record as those patterns too.

- Reverses the synthesized plan's question 6 and its departure (b) ("a fixed set ... not a free-form shape"), which
  are deleted. Two kinds of recorded gesture: fixed patterns (`touch.2.*`, `touch.3.*`, `touch.1.<dir>`, `pen.*`)
  and recorded one-finger shapes (`shape.<action>`).
- Capture in screen mm, independent of page zoom; anywhere on the page, upright or sideways; matched by
  `fingershape.rs` (called, not re-planned): G8.
- Recording 2-3 samples in Settings > Gestures (G9), stored as `shape.<action>` lines in gestures.ini (G10).
- "For any action": every action in the registry takes a finger shape, a touch pattern and (phase 6) a pen gesture,
  with one physical exception. `hold-eraser` is momentary (the eraser while held, back at the release); a finger
  shape or a two-finger swipe has no release, so `hold-eraser` takes keys, controller buttons and, from phase 6, the
  pen button's hold and drag slots (a pen-button hold has a release). This is the plan's one departure from D4's
  words (section 5 (g)); open question 2 asks whether a latched finger form is wanted. `gestures` is assignable to
  every source: a gesture can switch gestures off (the toast says how to switch them back on), and the Settings >
  Gestures row, a key or a controller button switches them on (section 12.4).
- Ships in phase 3. Angle rule 2 becomes "two fingers navigate; one finger taps, holds and draws shapes; the pen
  writes" (section 5).

### D5. Two fingers pan when the page can scroll

> Two fingers pan when the page can scroll, zoomed in or sideways. There, the user's shapes and the panel's
> Pages/Links buttons open those menus. NEW SETTING: sideways, fit the whole page (no scrolling), so the two-finger
> swipes work in every orientation. On by default. Assignable like every toggle.

- Per axis, a two-finger translate on an axis the view can scroll is a PAN (G5). The synthesized plan's edge bands
  are dropped: D5 decides it (where the page scrolls, two fingers pan, and the panel buttons and shapes open the
  menus), so bands are neither planned nor asked about.
- `pinch_zoom` off disables zoom only; a classified pan still pans (G5).
- Where the page scrolls, Pages and Links open from the panel's Pages chip and Links button (always visible on the
  expanded panel), finger shapes (phase 3), keys and controller buttons.
- New setting `landscape_fit` (settings.ini, default 1), Display sheet row 'Sideways, show the whole page', action
  `landscape-fit` (TOGGLE): G12. Ships in phase 1 with the two-finger built-ins; keys and controller buttons bind it
  in phase 1, gestures and shapes from phase 3. The phase 1 owner notes say it is on by default.

### D6. Built-in gestures

> two fingers down = Pages; two fingers up = Links bar; two fingers left/right = next/previous page (never adds a
> page; at the end, a toast). The mirror rule closes menus.

- `touch.2.down=pages`, `touch.2.up=links`, `touch.2.left=next-page`, `touch.2.right=previous-page`; the pen
  button's existing triple-click and hold `clear-page` stay (G6.1). Nothing else is a default: `pen.Button.double=
  links` is not one.
- `editor_shortcut_at(..., src)` with `src` in {Touch, Shape, Pen}: next-page on the last page toasts 'This is the
  last page.' and adds nothing (G6.3). Key and Pad keep today's panel behaviour.
- The mirror rule (G6.2). Phase 1.

### D7. Panel buttons

> The panel gets both new buttons: a Shapes toggle and a Links bar button.

- `P_LINKS` (chain glyph, `IconKind::Link`, with a 7 px link dot) after Next in the page group: phase 1,
  `PANEL_BUTTONS` 11 -> 12 (panel.rs:9).
- `P_SHAPES` directly under the eraser, never inverted: phase 4, `PANEL_BUTTONS` 12 -> 13 (section 10.7).
- Symbolic `tap panel NAME` lands in phase 1 before `P_LINKS` (section 15.1).

### D8. One Gestures sheet

> One Gestures sheet for touch and pen gestures (the pen plan's pen.ini folds into gestures.ini).

- One Settings > Gestures sheet (phase 3) whose rows show touch, shape and (phase 6) pen gestures.
- One file, gestures.ini, with `touch.*`, `shape.*` and `pen.*` lines (G10); there is no pen.ini. The Rust crate has
  no pengesture module and no pen.ini, so phase 6 builds `penbutton.rs` itself and migrates `pen_button`'s
  hard-coded triple-click and hold (editor.rs:2038) into gesturemap built-ins.

---------------------------------------------------------------------------------------------------------------

## 2. What changed from the synthesized plan

1. D4 reverses question 6: one-finger recorded shapes join the fixed patterns (Part G, sections G8-G10).
2. D5 answers question 4 by panning on any scrolling axis and adds `landscape_fit`; the edge bands, their 24/26 mm
   tests and their hint are gone; the pan hint names the panel buttons instead (G5).
3. The owner's six phases replace P1-P7. The old P1 ("Measure and harden", nothing new on screen) is merged into
   phase 1 with the gestures and the Links bar. The old P7 (pen-button gestures, which depended on another lane) is
   phase 6 and ships alone. The docs and fit sweep (old step 26) is spread over every phase (section 16).
4. Questions 1, 2, 3, 5, 7 and 8 are answered with the recommended option; their "(question N)" alternatives are
   removed.
5. Two features land on `claude/rust-port` before phase 1 and are planned against their described interfaces:
   Live view fast ink and the clock/battery corner (section 3).
6. Everything is in Rust terms: C files map to the Rust module of the same name; `CORE_SRC` means a pure module with
   no App dependency, host-tested; `tests/test_X.c` means `rust/tests/test_X.rs` or a `#[cfg(test)]` module; the gate
   is `make rust-check`.
7. Three pure modules come from `claude/rust-index-pure` (fingershape.rs, shape.rs, graph.rs). This plan calls them
   and never re-plans their internals (section 4.3).
8. Feasibility findings in the Rust code that change the plan's wiring (not its design):
   - The keyboard host already exists: `app_keyboard_pointer` and `app_keyboard_tick` (app.rs:1434, 1465) draw only
     on change, flush the field then the key, and settle by GL16 after 900 ms. The synthesized plan's kbhost step is
     therefore small: the toast floor, a hook between the field flush and the key so a sheet can flush its own rows
     there (phase 1, for Find), and a dock (top or bottom, phase 2) (section 8.1).
   - `zoomed()` (editor.rs:911) tests `s.ox != 0.0`; with the whole page centred sideways `ox` is negative at rest,
     so `landscape_fit` needs a rest offset (G12).
   - The editor reads every page through `store_load_page` (store.rs:1729, via `load_page` and `page_at`,
     editor.rs:445-461); neither it nor the exporter's `store_read_page` (store.rs:1776) pumps. Peeks (phase 1) need
     a pumped variant of the editor's reader, with the pump inside `page_deserialize` (section 14.4).
   - `history_undo`/`history_redo` (history.rs:247, 255) take only `&mut Page`; link steps need the editor to apply
     a link record itself (section 9.7).
   - `make c-check` runs the same `tests/scenarios`, the selftest and `y_all_screens` (through l10n-check) against
     the C binaries and has no skip list; new scenarios need a `# rust-only` header that the scenarios rule skips
     when c-check passes an explicit flag (section 15.1).
   - There is no SDL window simulator in the crate: `sim_sdl.rs` is the headless runner only (sim_sdl.rs:8-11), so
     the synthesized plan's window emulation (Shift, Alt or Ctrl with the right button) is not planned.
   - The pure branch holds empty module slots today (commit 887f38a), so the call surfaces in sections G14, 10.2
     and 11.3 are assumptions to verify when it lands.
9. Phase 1 details that `PHASE1-BRIEF.md` settled against the code (its "brief decision" notes) are folded back into
   this plan, so the two agree; section 19 lists them.

---------------------------------------------------------------------------------------------------------------

## 3. The two features that land first (contracts this plan relies on)

Neither is in `claude/rust-index-base` yet. Phase 1 starts from `claude/rust-port` after both land, verifies these
interfaces against the merged code, and adjusts only the call sites named here.

### 3.1 Live view fast ink

Described as: a pen sample hook in input_evdev, a capture-blocked flag in app dispatch, the editor's live ink style,
and a separate `/live/pen` stream (README: "a pen sample stream to the browser, with provisional ink").

What this plan does with it:

- The touch extraction (phase 1, `touchgest.rs`) touches only the touch path. `emit_pen`, the pen sample hook (at
  the end of `emit_pen`, its call deferred outside the `State` borrow as `drain` does for the hover hook) and
  `input_pump`'s pen-first order stay exactly as feature A leaves them. Pumping touch in `input_pump` goes after the
  pens and must not starve or reorder the pen sample hook.
- `/live/pen` carries pen samples only. Gestures, the finger-shape trail, the bar, the card, the eye, the lasso
  dashes and the map never reach it; they are `App.screen` pixels and reach the screen tile stream, as the panel
  does (accepted; open question 3).
- The capture-blocked flag: every pen contact that will NOT become ink raises it at PenDown, so `/live/pen` shows no
  provisional ink for it:
  - phase 1: the pen commit during a peek (the stroke is swallowed), pen taps on the bar, the Find sheet and its
    keyboard (covered if feature A's predicate already derives from `sheet_open` and the owners; this plan extends
    that predicate with `lb.open`, the peek state and `find.open`);
  - phase 2: the link lasso (`Owner::Lasso`), pen taps on the link card and Link to...;
  - phase 3: pen taps on the record card;
  - phase 5: pen drag and taps in the Index map;
  - phase 6: a pen-button drag slot bound to an action, raised at PenDown.
- Smart shapes (phase 4): provisional ink of the raw stroke is correct while drawing (it is ink); after the snap
  the page stream carries the snapped stroke. Phase 4 checks against feature A's contract that the browser drops
  provisional ink when the page frame for that rect arrives; if it does not, the snap sends the pen stream a clear
  for that stroke.
- The editor's live ink style means ink. The lasso dashes, the trail, the long-press strips and the map previews use
  `RefreshMode::Fast` without the live ink style (the synthesized plan's `display_live_ink(true)` for the lasso is
  replaced by plain Fast with its own flush; phase 2 re-checks against feature A's meaning of the style).
- If the flag turns out to gate screen dispatch ("input owned elsewhere"), the Gesture branch in `app_event` sits
  after it (G4.3).

### 3.2 The clock/battery corner

Described as: toggled by a finger TAP in a ~12 mm top-left zone. No gesture may claim that tap. A shape that starts
there is still a shape.

Assumed interface (G4.2 has the full contract): `clock_corner_zone(a) -> Rect` = `{0, 0, z, z}` with `z =
mm_to_px(12.0, a.dpi)` (about 142 px) in the logical frame; `clock_corner_tap(a, x0, y0, moved, cancelled) -> bool`
true for a one-finger, unmoved (30 px slop), real lift whose LANDING is in the zone. The corner decides at the lift,
never at the down, so the down still reaches the editor and a stroke that passes the capture's start gate becomes a
shape. If feature B
swallows the lift in `app_event`, it hands the screen an off-screen cancel up instead, so owners end cleanly.

Every finger-tap consumer this plan adds or touches calls the predicate first and does nothing when it is true:

| Consumer | Phase |
|---|---|
| `Owner::Link` lift, before `link_follow` | 1 |
| Links bar "tap the page to stay" commit, and its swallowed tap outside when not peeking | 1 |
| Find sheet dismissal (tap outside) | 1 |
| Link card's swallowed tap outside; Link to... dismissal; link-mode strip | 2 |
| Record card: a corner tap toggles the clock and is not reported as "One finger is kept for taps and links." | 3 |
| Index map empty-tap deselect | 5 |

Layout rules so nothing covers the zone:

- The link card's placement rule rejects any placement that meets the zone (it slides or tries the next side).
- Link to... docked at the top starts below the zone (y = z + 8, about 150 px, instead of 20). The Find sheet's
  rows card never starts above y = z + 8.
- The keyboard docked at the top (phase 2's PICK mode, section 8.1) starts its modal at y = z + 8, not 20, so its
  field and keys never meet the zone. A corner tap still wins while any keyboard is open (G4 rule 3).
- The eye's area tags move right of the zone when an area's top-left falls inside it.
- The Index map's header puts Back at x = z + 16 instead of 0 (section 11.5), so the corner stays live there too.
- The Links bar is at the bottom and never meets the zone.

The corner is one finger only: a two-finger tap whose midpoint is in the zone is `touch.2.tap`; the first finger of a
two-finger swipe that lands in the zone ends with the off-screen cancel and never toggles the clock. The pen is never
a corner tap. A gesture fire suppresses all contacts until they lift, so a late finger never toggles it either.

---------------------------------------------------------------------------------------------------------------

## 4. Rust module map

### 4.1 Existing modules this plan changes

| Module | What changes | Phases |
|---|---|---|
| `input.rs` | `InType::Gesture = 24`; flag bits on Touch events; `input_touch_context(TgView)`; `InputCaps.touch_slots` | 1 |
| `input_evdev.rs` | `touch_frame` body moves to touchgest; kernel time per SYN_REPORT; SYN_DROPPED resync; `input_pump` drains touch; real-lift bugs become cancels; touch log and trace file | 1, 3 |
| `platform_none.rs` | the new input entry points as no-ops | 1 |
| `sim_sdl.rs` | contact-level commands through touchgest (the headless runner only: no SDL window is ported, sim_sdl.rs:8-11); symbolic panel taps; probes; `shape`/`fpath`/`trace` | 1, 3 |
| `mainloop.rs` | `event_name` gains "gesture" and its assert; `App.input_drained` per batch; `main.longest_unpumped_ms` | 1 |
| `power.rs` | `power_asleep_drops` drops `InType::Gesture` | 1 |
| `app.rs` | Gesture dispatch gates; `App.gestures: GestureMap`; `App.input_drained`; settings probes; capture-blocked predicate extension; `app_keyboard_pointer`'s hook between the field flush and the key (section 8.1) | 1-6 |
| `editor.rs` | `editor_gesture`, `editor_shortcut_at`, view context push, pinch classes, `landscape_fit` view, Links bar hosting, peek and commit, back list, eye overlay, P_LINKS; later lasso, card, link history, migration, finger shapes, smart shapes, P_SHAPES, map hosting | 1-6 |
| `pagesview.rs` | 'Go to page...' opens the Find sheet; Index tab Graph button (5); '%d links here' counts (5); the hold path retired (2) | 1, 2, 5 |
| `nav.rs` | area lines, area-first boxes, smallest wins, migration, file cap, copy | 2 |
| `history.rs` | `ActKind::Link` and `ActKind::Replace`; undo/redo report the kind; drop-last helpers | 2, 4 |
| `store.rs` | new settings keys; a pumped variant of the editor's page reader `store_load_page`; `i18n_fold_next`/`i18n_name_match` built on `fold_next` (store.rs:1444) | 1-5 |
| `i18n.rs` | the name search helpers' public home | 1 |
| `shortcuts.rs` | `ActionSrc`; new action ids and names; `ShortcutBinding.gamepad: bool` becomes a source; `SHORTCUT_MAX` 64 -> 192 | 1, 3 |
| `library.rs` | Buttons tables gain the phase 1 names; Settings bottom row; `Modal::Gestures = 17`; BindKind; record card | 1, 3, 6 |
| `dispmenu.rs` | 'Sideways, show the whole page' row after Invert (dispmenu.rs:163) | 1 |
| `panel.rs` | `PANEL_BUTTONS` 11 -> 12 (1) -> 13 (4); `panel_fit` | 1, 4 |
| `keyboard.rs` | a `dock_top` field that `modal_rect` honours (top or bottom) | 2 |
| `canvas.rs` | `canvas_dash_rect` (1); `canvas_dash_poly` with a dot mode (2, 3); `canvas_slash` (4); `canvas_line_aa` row-span and `canvas_disc_pattern` (5) | 1-5 |
| `ui.rs` | new `IconKind` glyphs (eye, link-plus, magnifier, bubbles, shapes on/off, gestures) | 1, 3, 4 |
| `export.rs` | unchanged: `nav_link_areas` becomes area-first underneath | 2 |
| `bgjob.rs` | a layout work kind for big Index maps | 5 |
| `live.rs`, `httpd.rs` | nothing beyond feature A | none |

### 4.2 New modules (ours)

| Module | Kind | Owns | Phase |
|---|---|---|---|
| `touchgest.rs` | pure, host-tested (like `pensync.rs`) | contact frames in, today's touch events plus `InType::Gesture` out (G3) | 1 |
| `gesturemap.rs` | pure | built-ins, gestures.ini overlay (read 1, write 3), shape lines (3), pen slots (6), mirror | 1, 3, 6 |
| `linkbar.rs` | editor-hosted sheet, like `dispmenu.rs` | the Links bar layout, chips, tool row, peek header, Link to... pick mode (2) | 1, 2 |
| `findsheet.rs` | editor-hosted sheet | Go to page or name (GO mode 1, PICK mode 2) | 1 |
| `linkcard.rs` | editor-hosted card | the long-press card layout, placement, rows | 2 |
| `actions.rs` | pure registry | labels, groups, kinds, sources for every action | 3 |
| `mapview.rs` | full-screen view, like `pagesview.rs` | the Index map's screen, drawing, interaction | 5 |
| `penbutton.rs` | pure | pen-button click, multi-click, hold and drag recognition | 6 |

Each joins `rust/src/lib.rs` (`pub mod x;`) and its `prelude` (`pub use crate::x::*;`); the pure ones sit in the
foundations list beside `pensync`. Tests: `rust/tests/test_touchgest.rs`, `test_gesturemap.rs`, `test_actions.rs`,
`test_history.rs`, plus cases in `test_nav.rs`, `test_shortcuts.rs`, `test_panel.rs`, `test_probe.rs`, `test_i18n.rs`.
`autotests = true` picks up new test files.

The synthesized plan's `kbhost.c` is not a new module: the keyboard host is app.rs's `app_keyboard_pointer` /
`app_keyboard_tick`, a hook between the field flush and the key (phase 1) and a dock (phase 2) (section 8.1).

### 4.3 Pure modules from `claude/rust-index-pure` (called only)

| Module | Called from | Phase | What this plan calls |
|---|---|---|---|
| `fingershape.rs` | editor.rs (page capture), library.rs (record card), gesturemap.rs (codec) | 3 | capture, gates, `fs_match`, record checks, codec, trail and thumbnail helpers, synth (G14) |
| `shape.rs` | editor.rs (hold check, preview, shape-last) | 4 | recognize a stroke at a zoom and paper, return the kind and the snapped points (section 10.2) |
| `graph.rs` | mapview.rs, bgjob.rs | 5 | build the model from Nav and NotebookInfo, seed, relax with a pump, satellites, tray, Around, incremental placement (section 11.3) |

Their thresholds, algorithms, corpora and unit tests are the pure branch's; this plan quotes no threshold of
theirs as its own and reads them from the module (for example `FS_GATES`). If a module arrives without a piece this
plan calls (for example the finger capture state machine, which the recorded design assigns to fingershape), the
phase asks the pure branch to add it, with exactly the listed surface; nothing else in the plan changes.

---------------------------------------------------------------------------------------------------------------

## 5. Six rules you can feel (amended)

1. The page is home. Making, following, renaming, retargeting and deleting a link all happen over the page. Only the
   Pages view and the Index map fill the screen, and both open only when you ask.
2. Two fingers navigate; one finger taps, holds and draws shapes; the pen writes. A gesture fires when the fingers
   lift, never under a resting palm or a hovering pen. (D4 adds "draws shapes".)
3. Every gesture has a visible twin: two fingers down = the page chip; two fingers up = the Links button; left and
   right = the arrows; Go to, New link, Areas and Index are labelled buttons in the bar; Smart shapes = a panel
   button plus a labelled row in the tool picker; a finger shape is a row in Settings > Gestures with its picture.
4. Look, then commit. A jump from the bar is a peek: tap the page outside the bar to stay (one GC16), or use the
   bar's own close to go back. A shape snap is a preview until the pen lifts.
5. Undo covers everything on the page: making, deleting, retargeting or redrawing a link is one step, and so is each
   snapped shape.
6. The owner's words are the defaults. Where this plan goes beyond them it says so (section 19).

Departures from the brief's literal words that remain:

- (c) The card also offers Change where it goes and Redraw the area (R2 adopts them). It has no 'Go there': a tap
  already goes there.
- (d) A pen touch outside the bar stays on the shown page, but that stroke is not drawn: a closing refresh still on
  the panel would take the stroke off fast pen mode.
- (g) Two limits on "anything the controller category can" and "for any action": `hold-eraser` is momentary and a
  finger gesture has no release, so it is assignable to keys, controller buttons and (phase 6) the pen button, not
  to touch patterns or finger shapes (open question 2); calibration is a flow, not a setting, so it is not an action.
  A switch that a binding could turn off for good takes only the sources that survive it: Enable shortcuts and the
  Bluetooth controller toggle take TOUCH, SHAPE and PEN (never KEY or PAD, which they switch off). Everything else,
  language included, is an action (section 12.4).
- The synthesized plan's departures (a) (edge bands), (b) (fixed vocabulary), (e) and (f) are now rulings D5, D4,
  D3 and D6.

---------------------------------------------------------------------------------------------------------------

## 6. The experience (by phase)

Upright unless said otherwise. [phase N] names the build that brings each part.

### 6.0 After the update [phase 1]

- For an owner upgrading from an earlier build (13.4), the first time a notebook opens, one two-line toast for 6 s: 'New: two fingers up show this page's links. Two
  fingers down show all pages.' / 'Novedad: dos dedos hacia arriba muestran los enlaces de esta página; hacia
  abajo, todas las páginas.' (`hints` bit 0).
- The panel has a new Links button after Next: a chain glyph, with a 7 px solid dot in its corner when this page has
  links. Phase 4 adds a Shapes button under the eraser.
- The first sideways turn shows the sideways hint once (`hints` bit 8, G12).

### 6.1 Opening the Links bar [phase 1]

- Two fingers up anywhere on a page that does not scroll vertically, the Links button, or the `links` action (a key,
  a controller button; from phase 3 a gesture or finger shape; from phase 6 a pen gesture).
- When the fingers lift, the card rises from the bottom in one GL16: about 0.35 s, no flash.
- Header: 'Links on this page (3)' / 'Enlaces de esta página (3)', X at the right.
- Numbered chips in reading order, one per Index entry this page links to (D1): '1 Meeting notes  p. 12', '2 Page 20
  p. 20', '3 The Index'. Two links to one entry show once, marked '×2'. The eye draws the same numbers on the page.
- Tool row: Go to / Ir a (magnifier), New link / Nuevo enlace (chain with a plus), Areas / Áreas (an eye, dimmed
  while off), Index / Índice (three bubbles).
- Empty page: 'No links on this page yet.' / 'Esta página aún no tiene enlaces.', with New link as the main,
  inverted button.
- The first 3 times, the header adds 'Tap a link to look. Tap the page to stay.' / 'Toca un enlace para mirarlo.
  Toca la página para quedarte.' (`hints` bits 3-4).

### 6.2 Hopping (peek) [phase 1]

- Tap chip 1: it inverts at once (DU) and the page above the bar becomes page 12 (GL16 of that area only), about
  0.4 s from the prefetch, up to 1.2 s cold; the panel's page chip shows 12.
- Header: '‹ Page 3   Showing Meeting notes. Tap the page to stay.' / '‹ Página 3   Mostrando Meeting notes. Toca
  la página para quedarte.' with the X. The chips stay page 3's (D2).
- Stay: tap the page above the bar with a finger (counts at the lift). The bar closes, the shown page becomes the
  open page, one GC16 marks the arrival. That tap does nothing else (no link follow, no panel press). A pen touch
  there also stays and its stroke is not drawn. A tap on the panel also stays and the button is not pressed, except
  the Links button: a tap on it is "the Links button again" and goes back (below). A tap in the clock corner zone
  toggles the clock only and does not stay.
- Back: '‹ Page 3' shows page 3 again and keeps the bar. X, two fingers down (whatever gestures are bound), the
  Links button again, Close menu or the Back key close the bar and return to page 3 exactly as left, zoom included
  (GL16, or GC16 after 3 or more peeks).
- New link, Go to, Index, or another menu's button or key act on the shown page, so they commit first.
- Peeks never write notebook.ini: after a sleep, power-off or crash during a peek the notebook reopens on page 3.

### 6.3 Holding a link [phase 2; phase 1 keeps today's hold]

- Hold a finger still on a link for 0.6 s: a heavy dotted border appears around its area in about 0.2 s (only the
  four strips, by DU); then a card opens beside it by GL16 (right if it fits, else left, below, above; never over
  the clock corner). The page stays in view.
- Card (780 x 666): title 'Meeting notes · p. 12' / 'Meeting notes · pág. 12'; Rename in the Index / Renombrar en
  el índice (an unnamed page: 'Add page 20 to the Index...' / 'Añadir la página 20 al índice...'; a link to The Index
  has no rename row); Change where it goes... / Cambiar a dónde lleva...; Redraw the area / Volver a dibujar la zona;
  Delete link / Borrar enlace.
- Rename: the keyboard opens over the page with the card and border visible; Done toasts 'Page 12 is in the Index as
  Meeting notes.' / 'La página 12 está en el índice como Meeting notes.'
- Change where it goes: Link to... with the current target marked, on the page (no 'Choose from the pages...' from
  the card, R2). Redraw the area: draw a new area for this link.
  Delete link: 'Link deleted. Undo brings it back.' / 'Enlace borrado. Deshacer lo recupera.' A border you drew is
  ink and stays.
- A tap outside closes the card and does nothing else. On a read-only page the card shows only the target and 'This
  page is read-only.' / 'Esta página es de solo lectura.'
- In the bar, holding a chip 0.6 s opens the same card above the bar; if peeking, the bar first returns to the origin;
  the entry's areas get the border; a '×2' entry offers 'Delete 2 links' / 'Borrar 2 enlaces' and 'Change where they
  go...' / 'Cambiar a dónde llevan...' with 'To change one, hold it on the page.' / 'Para cambiar uno, mantenlo pulsado
  en la página.' and no Redraw (D1).
- Phase 1: the page hold keeps today's `link_hold` (the Pages-view picker); a chip hold does nothing.

### 6.4 Making a link, without ink [phase 2; phase 1 uses today's inked flow]

1. New link in the bar, the `add-link` action, or 'Add a link' on the Index tab.
2. The bar folds into a 130 px strip: 'Draw around what links.' / 'Rodea lo que enlaza.', with Cancel.
3. Circle the words with the pen: a 4 px dashed line follows the nib by DU. It is screen UI, never ink.
4. At the lift, the dashes become a dotted rectangle around the circle's box.
5. Link to... ('Where does the link go?' / '¿Adónde va el enlace?') opens at the bottom, or at the top (below the
   clock corner) when the rectangle is in the lower part of the screen. Chips: New page... / Página nueva..., The
   Index / El índice, up to 4 Recent targets, then the Index entries, paged. Tool row: Search / Buscar, Choose from
   the pages... / Elegir entre las páginas..., Cancel / Cancelar.
6. Tap 'Meeting notes': toast 'Linked to Meeting notes.' / 'Enlazado con Meeting notes.'; the dotted rectangle goes
   (unless Areas is on); the page looks exactly as before; from the bar, the bar comes back with the new entry; one
   Undo removes the link.

Other outcomes: New page... asks 'Name the new page' / 'Nombre de la página nueva', appends a page at the end, names
it and links to it ('Linked to the new page 53.' / 'Enlazado con la nueva página 53.'); too small (under 5 mm both
ways): 'Too small for a link: draw around what links.' (existing msgid), mode stays; Cancel: 'No link made.'
(existing msgid); a refusal always says why (read-only page, links.ini unreadable, 64 links on the page; strings in
section 9.3).

Phase 1: New link runs today's `link_start` (the inked lasso and the Pages-view pick), said in the phase 1 notes.

### 6.5 Following [phase 1; smallest-wins from phase 2]

- A finger tap inside a link's area goes there with GC16, as today; the page left goes on the back list.
- Nested areas: the smallest under the finger wins (phase 2; phase 1 keeps today's rule on legacy boxes).
- The pen always writes, even on a link. A tap in the clock corner zone toggles the clock, never follows.

### 6.6 Seeing areas [phase 1]

- Areas in the bar draws a dashed outline and a numbered tag on every link area of the page; numbers match the chips.
  Remembered (`link_areas`). Never drawn into the page, the PDF, thumbnails or the sleep image.
- First time: 'Dotted boxes show where a finger tap follows a link. They never print.' / 'Los recuadros punteados
  muestran dónde un toque con el dedo sigue un enlace. Nunca se imprimen.' (`hints` bit 2).
- Phase 1 boxes come from the legacy ink fallback; from phase 2 they are stored areas.

### 6.7 Visible borders

A visible border is ink you draw yourself; from phase 4 Smart shapes make a clean box. It is never part of the link:
erasing it keeps the link, and deleting the link keeps the border.

### 6.8 Go to [phase 1]

- From the bar's Go to, 'Go to page...' in the Pages view, or the `go-to-page` action. A keyboard over the page,
  field 'Page number or name' / 'Número o nombre de página'. '12' puts 'Go to page 12' / 'Ir a la página 12' first;
  'reu' lists 'Reunión' (accents and case ignored).
  A result goes there (GC16), and the page left goes on the back list.

### 6.9 Pages view and page turns [phase 1]

- Two fingers down open the Pages view (GC16); two fingers up close it (mirror rule).
- Two fingers left or right turn the page. At the end: 'This is the last page.' / 'Es la última página.'; at the
  start: 'This is the first page.' / 'Es la primera página.' A gesture never adds a page; the panel's + still does.

### 6.10 Sideways and zoomed in [phase 1]

- Sideways, the whole page fits by default (`landscape_fit` = 1): nothing scrolls at rest, and every two-finger
  swipe works as upright. Pinch zooms in from the whole page.
- Where the page can scroll (zoomed in, or sideways with the setting off) a two-finger drag pans, now without zoom
  drift, even with pinch zoom off. There, Pages and Links open from the panel's Pages chip and Links button (and
  from phase 3 your finger shapes). The first such pan shows 'Here two fingers move the page. Open Pages and Links
  from the panel.' / 'Aquí dos dedos mueven la página. Abre Páginas y Enlaces desde el panel.' once (`hints` bit 6).
- An axis that does not scroll still swipes: zoomed so only the vertical axis scrolls, left and right still turn
  pages.

### 6.11 Finger shapes [phase 3]

- Record a shape for any action in Settings > Gestures (2-3 samples). Draw it with one finger anywhere on the page,
  at any zoom: a dotted trail follows the finger; at the lift it fires (the trail clears with one GL16 and a toast
  names the action) or fades if it is not one of yours. Details: G8-G11.

### 6.12 Smart shapes [phase 4]

- Draw a box and hold the nib still for 0.55 s: one DU frame shows a clean rectangle in the same pen and size. Lift
  to keep it; move on and your own line comes back, and the stroke continues. Handwriting never snaps. One Undo
  removes the shape. On by default.
- The panel's Shapes button toggles it: a filled glyph when on, an outline with a slash when off, never inverted.
  The tool picker gets the row 'Smart shapes: On' / 'Formas inteligentes: activadas' and 'Hold: Normal' / 'Espera:
  Normal'.

### 6.13 The Index map [phase 5]

- From the bar's Index button, the Graph button on the Index tab, or the `index-map` action. Full screen (GC16),
  titled 'Index' / 'Índice' with a Graph | List switch ('Grafo | Lista').
- Every page is a dot; the open page has a double ring; named pages are labelled; well-linked pages are solid,
  lightly linked ones half or quarter dithered, unlinked ones hollow; unnamed unlinked pages are small hollow dots
  around their section's named page; unlinked pages before any name sit in a 'Not linked yet' / 'Aún sin enlaces'
  tray.
- Tap a dot for its card ('Page 12. Links to 3, linked from 2.' / 'Página 12. Enlaza a 3, enlazada desde 2.', Open /
  Abrir, Around it / Alrededor, Rename / Renombrar); tap it again to go there.
  Drag with one finger to pan, pinch to zoom: rough DU previews while moving, one GL16 at the end.

### 6.14 Settings > Gestures [phase 3; pen rows phase 6]

- The Settings bottom row reads KOReader | Controller | Gestures | Updates | Done (Spanish: KOReader | Mando | Gestos
  | Actualizaciones | Listo, the existing words plus 'Gestos').
- The sheet: 'Gestures: on (tap to change)' / 'Gestos: activados (toca para cambiar)'; Editing..., Tools..., Pages...,
  Index..., Menus..., Screen..., System... (the group names of section 12.5, EN and ES there); Reset gestures /
  Restablecer los gestos; Done. Rows read 'Action: gesture' ('Pages view: 2 fingers down' / 'Vista de páginas: 2
  dedos hacia abajo'), with a thumbnail for a finger shape.
- Tap a row and the record card asks 'Do the gesture for Undo' / 'Haz el gesto para Deshacer'; a two-finger pattern
  shows its name large; a finger shape fills up to three numbered slots; Use, Try again, Cancel / Usar, Otra vez,
  Cancelar. A refused try says why. Hold a row 0.6 s to clear it; Reset gestures brings back the built-ins (and
  confirms first when shapes exist). It works with controller shortcuts off.

### 6.15 When things go wrong

- links.ini unreadable: the bar says 'Links could not be read.' / 'No se pudieron leer los enlaces.' and New link
  says why instead of doing nothing.
- A gesture refused only because the pen was near: 'Move the pen away, then swipe.' / 'Aparta el lápiz y desliza.'
  at most once every 10 minutes.
- 2000 pages and 4000 links: the bar, the card and following a link cost the same as anywhere else; the map opens on
  a quick placement and tidies itself once its background layout finishes.

---------------------------------------------------------------------------------------------------------------

# Part G. Gestures

Part G is the integrated gesture design: the touch pipeline, the two-finger built-ins, one-finger shapes,
recording, gestures.ini and the sideways "fit the whole page" setting, in Rust terms. Section numbers G1-G15 are
referenced from the rest of the plan.

## G1. Layers and modules

| Module | New? | Owns | Phase |
|---|---|---|---|
| `touchgest.rs` | new, pure (host-tested like `pensync.rs`) | contact frames in, today's TouchDown/Move/Up and Pinch* stream out, plus `InType::Gesture`; landing windows, commit classes, swipe/tap gates, resting contacts, pen-near rule, size guards, record mode | 1 (record mode, 3 fingers, hover rule: 3) |
| `input_evdev.rs` | changed | slot bookkeeping (`TouchSt`), kernel time per SYN_REPORT, the frame build, the touch log and trace file; calls touchgest; pen path and the Live pen sample hook unchanged | 1 |
| `sim_sdl.rs` | changed | contact-level runner commands feeding the SAME touchgest (headless only: no SDL window is ported, sim_sdl.rs:8-11); legacy commands unchanged | 1 (shape/fpath/trace: 3) |
| `platform_none.rs`, `input.rs` | changed | `InType::Gesture`, flag bits, `input_touch_context()` forwarding to all three platforms, `InputCaps.touch_slots` | 1 |
| `mainloop.rs` | changed | `event_name` entry for Gesture (assert moved to the new last variant); `App.input_drained` set per batch | 1 |
| `app.rs` | changed | Gesture dispatch gates (asleep, book running, presence, capture-blocked flag position); the corner-tap predicate call site | 1 |
| `gesturemap.rs` | new, pure | built-ins, the gestures.ini overlay (read in phase 1, write in phase 3), shape lines (phase 3), pen slots (parsed from phase 1, live in phase 6), mirror lookup | 1, 3, 6 |
| `editor.rs` | changed | `editor_gesture`, `editor_shortcut_at`, view context push, PAN/ZOOM classes in `pinch()`, the one-finger session (`Owner::Link` plus a finger capture in phase 3), trail, landscape_fit in `view_reset`/`view_clamp`/`zoomed` | 1, 3 |
| `shortcuts.rs` | changed | `ActionSrc` enum; new action ids and names; in phase 3 `ShortcutBinding.gamepad: bool` becomes a source | 1, 3 |
| `actions.rs` | new (phase 3, plan step 16) | the UI-free action registry (labels, groups, kinds, sources mask KEY, PAD, TOUCH, SHAPE, PEN) | 3 |
| `library.rs` | changed | Settings bottom row, Modal::Gestures = 17, BindKind sheet, record card; Gesture events in record mode | 3 |
| `fingershape.rs` | from claude/rust-index-pure: CALLED ONLY | capture state machine, gates, integer $P, record checks, slip bank, codec, trail/thumbnail helpers | called from 3 |
| `penbutton.rs` | new, pure (phase 6) | pen-button click/hold/drag recognition replacing `pen_button`'s hard-coded logic (editor.rs:2038) | 6 |

`lib.rs`: `pub mod touchgest; pub mod gesturemap;` (phase 1), `actions` (phase 3), `penbutton` (phase 6), each
re-exported in `prelude`, in the pure foundations list next to `pensync`.

---------------------------------------------------------------------------------------------------------------

## G2. The event contract (input.rs)

- `InType::Gesture = 24`, appended after `KeyUp = 23`. `mainloop.rs` `event_name` gains `"gesture"` and its
  `const _: () = assert!(K.len() == InType::Gesture as usize + 1)`.
- `InEvent` for a Gesture: `value` = gesture id (`>0`), or in record mode a negative reject reason; `x, y` = start
  midpoint; `x2, y2` = end midpoint; `t_ms` and `k_ms` = the firing frame's time.
- Gesture ids (touchgest.rs `pub const`): `G_2_UP, G_2_DOWN, G_2_LEFT, G_2_RIGHT, G_2_TAP` and the same five for 3
  fingers. One-finger swipes and shapes are NOT touchgest events (the editor classifies them via fingershape.rs).
- `TouchDown/Move/Up` gain: `k_ms` (the SYN_REPORT kernel time, today pen only) and `value` flag bits
  `TOUCH_F_BROAD = 1` (this contact's TOUCH_MAJOR exceeded T_palm at some frame; log-only until phase 3 freezes
  T_palm) and `TOUCH_F_CANCEL = 2` (the off-screen cancel; x and y stay -10000.0 exactly as `cancel_touch` sends
  them today, input_evdev.rs:1330, so existing code keeps working).
- `PinchBegin.value`: 0 = legacy (runner `pinch`; the editor behaves exactly as today), 1 = a
  touchgest-classified session follows.
- `PinchMove.value` = class: 0 legacy, 1 ZOOM (touchgest committed; the editor skips its own 10% / 70 px dead
  zone and re-anchors at the first classified move), 2 PAN (offsets only, zoom held at its commit value), 3 SWIPE
  (the editor ignores the moves).
- Order on a fire: `PinchEnd` first, then `Gesture`, in the same batch, so `s.pinching` is already clear when
  `editor_shortcut_at` checks it. A session rejected only because the pen was near ends with `PinchEnd.value = 1`
  (0 otherwise; no existing reader looks at it), which is how the editor learns to show the pen toast (phase 1).
- Record-mode reject reasons (negative values, phase 3): `-1` pen near, `-2` fingers not together, `-3` not
  straight, `-4` swipe farther, `-5` fingers not the same way, `-6` hand moved, `-7` spread kept for zoom, `-8` one
  finger kept for taps and links.
- Every screen ignores `InType::Gesture` except the editor and (record mode, phase 3) the Library: `library_event`
  returns on `app_pointer_phase(ev) < 0` (library.rs:2405), so the record branch goes before that line.

---------------------------------------------------------------------------------------------------------------

## G3. The pipeline

```
 evdev slots ──► input_evdev: TouchSt + k_ms per SYN_REPORT ──► TgFrame ─┐
 runner finger/swipe/tapn/shape/trace ──► TgFrame ───────────────────────┤
                                                                         ▼
                                        touchgest_frame(g, frame, pen, &mut out)   (pure, O(1), no alloc)
                                                                         │
       ┌────────────────────────┬──────────────────────┬─────────────────┴───────────┐
   1 contact               2-3 contacts, ZOOM/PAN   2-3 contacts, SWIPE/TAP      suppressed (palm, pen,
   TouchDown/Move/Up       PinchBegin(1)/Move(1|2)  PinchBegin(1)/Move(3)/        late contact, after a
   (k_ms, flags)           /End                     End + Gesture(id)             fire): nothing until all lift
       │                        │                      │
       ▼                        ▼                      ▼
 app_event: asleep drop · book-running drop · note_input · [capture-blocked flag position] · screen
       │                        │                      │
 editor_event                pinch(): legacy (0) as    editor_gesture: mirror rule ─► gesturemap ─►
 Owner::Link session:        today; 1 zoom, 2 pan,     editor_shortcut_at(a, action, x, y, Touch)
  corner tap / link tap /    3 ignore                  (every existing gate: idle, sheets, keyboards)
  link hold / [phase 3]
  shape or 1-finger swipe ─► fingershape.rs ─► editor_shortcut_at(..., Shape)
```

### G3.1 touchgest.rs (phase 1)

Assumed shape of the pure API (ours to write in phase 1):

```rust
pub struct TgContact { pub id: i32, pub slot: i32, pub x: f32, pub y: f32,
                       pub nx: f32, pub ny: f32,          // device-normalized: calibrate.rs:310 reads ev.nx/ny
                       pub major: i32, pub tool: u8, pub pressure: i32 }
pub struct TgFrame { pub k_ms: u32, pub n: usize, pub c: [TgContact; 16] }      // logical px after map()
pub struct TgPen { pub prox: bool, pub touching: bool, pub left_ms: u32, pub last: (f32, f32),
                   pub hover: Option<(f32, f32, u32)> }                          // hover used from phase 3
pub struct TgView { pub scrolls_v: bool, pub scrolls_h: bool, pub zoom_ok: bool, pub gestures_on: bool,
                    pub record: bool, pub keyboard_open: bool }
pub struct TgOut { pub n: usize, pub ev: [InEvent; 8] }
pub fn touchgest_frame(g: &mut TouchGest, f: &TgFrame, pen: &TgPen, out: &mut TgOut);
pub fn touchgest_cancel(g: &mut TouchGest, k_ms: u32, out: &mut TgOut);   // pen arrival, SYN_DROPPED
pub fn touchgest_tick(g: &mut TouchGest, k_ms: u32, out: &mut TgOut);     // landing windows with no new frame
pub fn touchgest_set_view(g: &mut TouchGest, v: TgView);
pub fn touchgest_probe(g: &TouchGest, key: &str, out: &mut String) -> bool; // touch.* probes
```

Times are `u32` with `wrapping_sub` (PORTING timing rule). Geometry is classified in integer px rounded from
`map()`'s f32; emitted events carry `map()`'s f32 unchanged, so a single finger's TouchDown/Move/Up are bit-identical
to today's.

1. ONE FINGER: exactly today's stream: TouchDown at once, TouchMove on change, TouchUp at lift, no added latency.
   The global pen window (`P.prox || now - pen_left < 1000 ms`) still suppresses single fingers.
2. LANDING: a second contact within 150 ms (prov.): the first finger ends with the off-screen cancel
   (`TOUCH_F_CANCEL`, never a tap), `PinchBegin(value 1)` goes out, and the session can become a gesture. 150-400
   ms: zoom or pan only, never a swipe or tap. Over 400 ms: a palm, suppressed (today's rule, input_evdev.rs
   1385-1392). A third contact within 250 ms makes three fingers (phase 1: a three-finger session is rejected and
   never zooms or pans; `touch.3.*` fire from phase 3 when `InputCaps.touch_slots >= 3`). The finger count is fixed
   at the first of: any contact moving 3 mm, the end of the landing window, the first lift. A later contact rejects
   the gesture, except during a committed zoom.
3. COMMIT (today's dead zone): the session commits when the spread changes by 10% or the midpoint travels 70 px.
   Classified once, there:
   - ZOOM if `zoom_ok` and spread change px >= 0.6 x travel. `zoom_ok` = `settings.pinch_zoom` and no sheet that
     refuses pinches (the existing PinchBegin gates).
   - Otherwise TRANSLATE along the dominant axis. On an axis the view can scroll, a TRANSLATE is a PAN (D5; no edge
     bands; per axis, never per direction, so a drag that starts at a scroll limit pans nowhere and never navigates
     by surprise). More than 30 degrees off axis: PAN if either axis scrolls, else a reject ("Swipe in a straight
     line." in record mode).
   - Otherwise it is a SWIPE candidate, and nothing moves on screen.
   - Three fingers are never ZOOM or PAN.
4. FIRING, at the first lift:
   - SWIPE: axis travel >= 20 mm (236 px); off-axis <= tan 30 degrees of that; each finger moved >= 12 mm with
     cosine >= 0.8 to the midpoint direction; largest finger travel <= 2x the smallest; spread change <= 25%; <= 1000
     ms from the first landing. Sliding back under 20 mm cancels it.
   - TAP: every contact lifts within 150 ms of the others, <= 300 ms from first landing to last lift, no contact
     moves more than 5 mm.
   - No multi-finger double taps in v1, so nothing ever waits.
   - After a fire or a reject, every contact stays suppressed until all have lifted (today's `T.suppressed`), so a
     late finger never leaks a tap, a link follow or a corner toggle.
5. RESTING CONTACTS (grip thumb, writing palm): a contact still within 3 mm for 300 ms before the gesture's first
   finger, or flagged `MT_TOOL_PALM`, is left out of the count and the landing rule if every gesture finger is >= 40
   mm from it; if it moves > 3 mm during the gesture the gesture is rejected ("Your hand moved: try again.").
   Multi-finger only.
6. PEN NEAR (multi-finger, staged): phase 1 ships the conservative rule: pen touching, in proximity, or gone < 300
   ms: reject; gone 300-1000 ms: accept when every finger is >= 40 mm from the pen's last contact point. Phase 3 (from
   the log): also accept while the pen hovers when the hover mapping is learned, the hover point is <= 50 ms old, and
   every finger is >= 60 mm from it. Today `hover_learn` (input_evdev.rs:913) runs only while Live view's hover hook
   is installed (`if self.hover_fn.is_some() && contact`, input_evdev.rs:1002) and needs the raw wacom digitizer
   (`wacom.fd`, `pen_mono`, `wacom_mono`, `W.placed`), and the hover point comes only from `raw_hover`
   (input_evdev.rs:882-893). So phase 3 changes input_evdev: `hover_learn` runs on contact frames whether or not the
   hook is set (while grabbed, never at the PIN), and `TgPen.hover` is fed from `raw_hover`. A device without the raw
   digitizer never learns the mapping and keeps the phase 1 rule. Size guards: TOUCH_MAJOR > T_palm rejects (log-only until phase 3), fingers < 8 mm or > 70 mm apart
   reject, a contact ever reported as tool 1 is never a finger. A valid swipe refused only for the pen toasts
   "Move the pen away, then swipe." / "Aparta el lápiz y desliza." at most once per 10 minutes.
7. Pending timeouts (the 3-finger landing window) resolve in `touchgest_tick`, which `input_poll` calls after a
   touch drain that read nothing, so a still pair resolves without a new frame and a queued lift is never missed.

### G3.2 Input fixes folded into the extraction (phase 1)

- Pen arrival (`pen_frame`, input_evdev.rs:1062-1067) and the palm window (1436-1442) end a finger with the
  off-screen cancel, never a real lift (today both emit a TouchUp at the finger's place, which can follow a link or
  toggle the corner).
- `SYN_DROPPED` on touch resyncs every slot with `EVIOCGMTSLOTS` (the pattern in `input_set_grab`, 2833-2842) and
  suppresses until all lift.
- Kernel timestamps: `ev_us(ev)` at SYN_REPORT (input_evdev.rs:685; every node is already CLOCK_MONOTONIC).
- `input_pump` (2579) drains touch into touchgest too, after the pens as today, so a 1.2 s render no longer
  overflows the kernel buffer; queue order is preserved. The pen sample hook (feature A, at the end of `emit_pen`)
  is not touched and still fires from the pen path only.
- The busy flag (`input_poll`, 2631) counts every down contact, not just `T.active`.
- The pinch pair is chosen by landing order, not slot order; a pinch ends cleanly when one of its fingers lifts.
- When PinchBegin cancels a finger's panel press (editor.rs:3239-3250), the button is restored with
  `RefreshMode::Fast`, not `Ui`.
- Log: the touch node's `ABS_MT_SLOT` and `ABS_MT_TOUCH_MAJOR` (0x30, new constant) absinfo go on a NEW `qlog!`
  line right after the "input: touch %s ..." line (input_evdev.rs:2169-2177): `input: touch slots %d..%d,
  touch_major %d..%d r%d`. The existing "input: found ..." line and its NAMES table (input_evdev.rs:2006-2031) are
  untouched, since adding axes there would change an existing log line (rule 9) and could be cut by its 256 - 40
  byte budget. `InputCaps.touch_slots` from `EVIOCGABS(ABS_MT_SLOT)`.

### G3.3 One finger on the page (editor.rs, `Owner::Link`)

Today every finger landing on the page outside the panel with no sheet becomes `Owner::Link` (editor.rs:3437-3441):
an unmoved lift follows a link, a 600 ms unmoved hold (`LINK_HOLD_MS`, tick at 3652) runs `link_hold`. V2 keeps the
owner and its name and adds, in order of decision at the lift:

1. Off-screen cancel (`TOUCH_F_CANCEL`): the session ends silently (today's `press_move` to -10000 already marks
   it moved). No follow, no corner, no shape (an armed shape capture aborts).
2. Corner tap (feature B, section G4.2): unmoved within the 30 px slop, landed in the zone: the corner only.
3. Link tap: unmoved, not held: `link_follow` as today.
4. Link hold: fired by `editor_tick` (phase 1 keeps `link_hold`; phase 2 replaces it with the card).
5. Phase 3, when armed: a finger whose capture reached DRAWING (fingershape's start and lead gates) becomes a shape
   or a one-finger swipe (section G8).
6. Anything else: nothing, as today.

Belt and braces for "the pen always wins": a `PenDown` that arrives while `s.owner == Owner::Link` (possible only in
the runner, or if a prox frame was lost) ends the finger session silently (shape abort, no follow) and starts the
stroke, instead of today's "one gesture at a time" drop at editor.rs:3378 (`return; // one gesture at a time`,
in the check at 3376-3379). This changes only that one case; `make
scenarios` must confirm no scenario relied on the drop.

---------------------------------------------------------------------------------------------------------------

## G4. Priority and arbitration

Strongest first. A lower rule never overrides a higher one.

1. Not Grimoire's input: not grabbed (the PIN), asleep (`power_asleep_drops` gains `InType::Gesture`), a book being
   added (`app_event` 772-775 also drops Gesture). Gestures count as presence (`note_input`).
2. THE PEN ALWAYS WINS. Pen proximity or touch cancels every finger session at once (`TOUCH_F_CANCEL`, PinchEnd,
   suppression until all lift); single fingers are suppressed in the 1 s pen window; multi-finger gestures follow
   the staged rule; the pen writes even on a link area, over a shape trail, or while the bar is up (the bar's own
   rules then commit and swallow that stroke). A finger shape never delays ink: pen arrival aborts it (section G8.6).
3. The clock corner's one-finger TAP (section G4.2). Nothing below may consume it, a keyboard included: a keyboard
   never covers the zone (section 3.2), and a tap there toggles the clock even while one is open.
4. A keyboard is open (Pages view Go to page, Find, Rename, naming): every gesture is ignored
   (`TgView.keyboard_open`, and `editor_gesture` checks again); every other finger tap goes to the keyboard.
5. Two or three contacts: touchgest's classes (ZOOM, PAN, SWIPE, TAP) decide; a second contact always aborts a
   one-finger session.
6. One finger: corner tap > link tap > link hold (600 ms, the card from phase 2) > shape / one-finger swipe (phase 3)
   > nothing. A shape needs early movement (fingershape's lead gate) and a hold needs none for 600 ms, so they are
   exclusive by construction; a tap never leaves the capture's start slop.
7. Action dispatch: the bar's fixed cancel and the mirror rule first (section G6.2), then the binding, then
   `editor_shortcut_at`'s existing
   gates (idle only, menu actions act through open menus, nothing through a keyboard). A refused dispatch does
   nothing (a refused shape fades as a reject).

### G4.1 Arbitration table

| Situation | Result |
|---|---|
| Two fingers land 120 ms apart, move 25 mm down, upright, nothing scrolls | `touch.2.down` = Pages view (GC16) |
| Same, zoomed in (vertical axis scrolls) | PAN; Pages opens only from the panel chip, a shape (phase 3) or a key |
| Same, zoomed in so only the vertical axis scrolls, drag left 25 mm | horizontal does not scroll: SWIPE, next page |
| Drag starts at the scroll limit on a scrolling axis | PAN that moves nothing; never a swipe |
| Second finger lands 250 ms after the first | zoom or pan only |
| Second finger 450 ms after | palm, suppressed until all lift |
| Pen hovering, two-finger swipe (phase 1) | reject, rate-limited toast |
| Finger resting on a link when the pen comes near | off-screen cancel: no follow |
| One finger taps in the top-left 12 mm zone over a link area | clock toggles; the link is not followed |
| First finger of a two-finger swipe lands in the zone | the swipe fires; the clock does not toggle (cancel up) |
| Two-finger TAP whose midpoint is in the zone | `touch.2.tap` (the corner is one finger only) |
| One finger starts in the zone and draws a Z (phase 3) | a shape |
| Links bar up, one finger taps the page | commits (stay); in the zone: clock only, no commit |
| Links bar up, finger moves past the capture's start gate (phase 3) | a shape, not a commit (section G8.7) |
| Pages view open, `touch.2.up` | mirror of `touch.2.down`: closes Pages |
| Links bar up, `touch.2.down` | the bar's fixed cancel: closes the bar back to the origin page (D2), whatever is bound |
| `touch.2.up=nothing` in gestures.ini, bar opened by the Links button, `touch.2.down` | still the fixed cancel: back to the origin, Pages does not open |
| Links bar up, finger taps the panel's Links button | cancel: back to the origin (the Links button again); any other panel tap commits and presses nothing |
| Zoomed in (or sideways with the setting off), Links bar up, two fingers down | the bar does not pan (G5): SWIPE, the fixed cancel, back to the origin |
| Zoomed in, Pages view open, two fingers up | the Pages view does not pan (G5): SWIPE, the mirror rule closes Pages |
| Display sheet open, `touch.2.down` | menu swap: Display closes, Pages opens (existing menu rule) |
| Paper sheet open, `touch.2.left` | refused (`sheet_open`), nothing happens |
| Last page, `touch.2.left` | toast "This is the last page.", no page added |

### G4.2 The clock/battery corner (feature B; contract this plan relies on)

Feature B lands on claude/rust-port before phase 1. This plan assumes, and phase 1 verifies, this interface:

- `clock_corner_zone(a: &App) -> Rect`: the square `{0, 0, z, z}` with `z = mm_to_px(12.0, a.dpi)` (about 142 px)
  in the logical frame of the current orientation (top-left as the user sees it).
- `clock_corner_tap(a: &App, x0: f32, y0: f32, moved: bool, cancelled: bool) -> bool`: true for a one-finger,
  unmoved (30 px slop, `press_move`), real lift whose LANDING `(x0, y0)` is in the zone. Cancel ups
  (`TOUCH_F_CANCEL`, x = y = -10000) are never corner taps, so the first finger of a two-finger gesture never
  toggles the clock.
- The corner decides at the LIFT, never at the down, so the down still reaches the editor and starts an
  `Owner::Link` session (and, in phase 3, a shape capture). If feature B swallows the lift in `app_event`, it must
  hand the screen an off-screen cancel up instead, so owners end cleanly.

Every finger-tap consumer this plan adds or touches calls the predicate first and does nothing when it is true:
`Owner::Link`'s lift (before `link_follow`), the Links bar's "tap the page to stay" commit (phase 1), the bar's and
card's swallowed tap outside (phases 1 and 2), Find and Link to... dismissal, the Index map's empty-tap deselect
(phase 5), and the record card (phase 3: a corner tap toggles the clock and is not reported as "One finger is kept
for taps and links."). One-finger tap and hold are reserved and never recordable, so no recorded gesture can claim
the tap. The pen is never a corner tap.

### G4.3 Live view fast ink (feature A; contract)

- The touch extraction touches only the touch path: `emit_pen`, the pen sample hook and its deferred call outside
  the `State` borrow stay exactly as feature A leaves them; `input_pump` reads the pens first, as today.
- Gestures and the finger trail are not pen samples: they never reach /live/pen. The trail is drawn into
  `a.screen`, so viewers of the screen stream see it (accepted, as the plan accepts the bar).
- The capture-blocked flag: a pen contact that will not become ink must raise it, so /live/pen shows no provisional
  ink. For this section: a gesture never involves the pen; surfaces a gesture opens (Pages view, Links bar, sheets,
  record card) are covered by whatever predicate feature A computes from `sheet_open` and the owners, extended with
  the bar, its peek and the record card; pen-button drag slots bound to an action (phase 6) raise it at PenDown. A
  finger shape in progress never raises it: the pen arriving aborts the shape and writes. If the flag turns out to
  gate screen dispatch ("input owned elsewhere"), the Gesture branch in `app_event` sits after it.
- The trail never uses the editor's live ink style (that style means ink).

---------------------------------------------------------------------------------------------------------------

## G5. D5: two fingers pan where the page can scroll

- "Can scroll" per axis, computed in `view_clamp` (editor.rs:848) after its 0.01 snap: `scrolls_v = ah / zoom <
  pgh - 0.5`, `scrolls_h = aw / zoom < pgw - 0.5`, both forced false whenever a surface that does not pan the page is
  in front: the Pages view, the Links bar (peeking or not), Find, the Display, Paper and tool sheets (everything
  `sheet_open` covers, editor.rs:2032, plus the bar and Find), and later the card, Link to... and the Index map
  (which pans itself). Pinches are not routed to the Pages view (editor.rs:3357-3360) and the bar does not pan, so a
  PAN there would move nothing; with the flags false a translate is always a SWIPE candidate, and the mirror rule,
  the bar's fixed cancel and menu swaps work zoomed in too. Pushed with `input_touch_context(TgView)` at the end of
  `view_clamp`, on every sheet open/close, keyboard open/close, `editor_enter`, `editor_leave` (all false), and on
  settings changes.
- `pinch_zoom` gates ZOOM only. A PAN pans even with pinch zoom off (D5 says pan where the page can scroll; today,
  with pinch zoom off and a width-fit sideways page, there is no way to pan at all). In `pinch()`, a classified
  session (`PinchBegin.value == 1`) starts with `pinching = true` regardless of `pinch_zoom`, keeping the other
  gates (`owner != None`, `paper_open`, `mb.open`, `ts.open`); a legacy session (value 0) keeps today's gate line for
  line, so `k_pinch_gate` and every runner `pinch` scenario are unchanged.
- Where the page scrolls, menus open from: the panel's Pages chip (`P_PAGES`) and Links button (`P_LINKS`, phase 1),
  finger shapes bound to `pages` or `links` (phase 3), keys and controller buttons. Both panel buttons stay visible
  whenever the panel is expanded; a collapsed panel is the user's choice and expands with one tap.
- One-time hint (settings `hints` bit 6, reused from the plan's edge hint with new text), the first time a
  two-finger drag pans while `touch.2.up` or `touch.2.down` is bound: "Here two fingers move the page. Open Pages
  and Links from the panel." / "Aquí dos dedos mueven la página. Abre Páginas y Enlaces desde el panel." (4 s,
  shown at the PinchEnd of that pan).
- Sideways with `landscape_fit` on (the default) nothing scrolls at rest, so the built-ins work anywhere in both
  orientations; zoomed in, D5 applies.

---------------------------------------------------------------------------------------------------------------

## G6. Built-ins (D6), the mirror rule, page turns

### G6.1 Built-ins (gesturemap.rs; never written to a file)

```
touch.2.down  = pages
touch.2.up    = links          (the Links bar)
touch.2.left  = next-page
touch.2.right = previous-page
pen.Button.triple = clear-page (today's 3 quick clicks, editor.rs:2038; live in gesturemap from phase 6)
pen.Button.hold   = clear-page (today's 1.2 s hover hold; same)
```

Two- and three-finger taps and all one-finger swipes stay unbound; the Gestures sheet suggests them on a line:
"Ideas: two-finger tap for Undo, three-finger tap for Redo." / "Ideas: toque con dos dedos para Deshacer, con tres
para Rehacer." `shape.*` has no built-ins (a built-in shape would fire for people who never asked).

### G6.2 The mirror rule

While a menu is in front (`front_menu`, editor.rs:2718, extended with `SHORTCUT_LINKS` while the bar is up), let
`own` be the touch gesture bound to that menu's action (`gesturemap_key_for(m, action, ActionSrc::Touch)`; for the
Pages view, the binding of `pages` or `index`). A gesture equal to `own` or to its mirror (same finger count, up and
down or left and right swapped) is dispatched as the menu's own action, which `editor_shortcut` already treats as
"close its own menu" (2832-2852). So 2-up closes the Pages view and each menu's own gesture closes it too.
Any other menu gesture swaps menus by the existing rule; `close-menu` bound to a gesture closes any. If the user
unbinds `pages` from 2-down, 2-up over the Pages view is simply `links` (a swap). The Pages view honours only these
closes (pinches are not routed there today, editor.rs:3359).

The Links bar adds one fixed rule before the mirror rule, because D2 names "two fingers down" unconditionally: while
the bar is up (peeking or not), `touch.2.down` is always the bar's own action (cancel back to the origin page),
whatever gestures.ini binds. Without it, a user who unbound `touch.2.up` (by hand in phase 1, by the sheet in phase
3) would get `pages` from 2-down over the bar, and 7.8's "another menu: commit, then swap" would commit the peeked
page, the opposite of D2. A scenario covers it with `touch.2.up=nothing` (G15.5).

### G6.3 Page turns never add a page

`editor_shortcut_at` with `src` in `{Touch, Shape, Pen}`: `next-page` on the last page toasts "This is the last
page." / "Es la última página." (1500 ms) and returns true; `previous-page` on the first page toasts "This is the
first page." / "Es la primera página." (a PLAN-V2 decision for symmetry with "at the end"). `Key` and `Pad` keep
today's `panel_act(P_NEXT)` behaviour, which adds a page on the last one as the panel's + does (editor.rs:2897-2899).

---------------------------------------------------------------------------------------------------------------

## G7. Dispatch

- `app_event` (app.rs:629): Gesture is dropped asleep and while a book runs, counts as presence, then goes to the
  screen. Gestures do not need Enable shortcuts; the master switch is `settings.gestures` (default 1; off: touchgest
  still pinches, emits no Gesture, and in phase 3 finger shapes are not armed).
- `editor_gesture(a, ev)` (new, editor.rs): ignored when `!s.open`, a keyboard is open, or a record card is up;
  applies the bar's fixed cancel (`touch.2.down` while the bar is up) and the mirror rule (G6.2); looks the id up in `a.gestures` (`gesturemap_action`); calls
  `editor_shortcut_at(a, action, mx, my, ActionSrc::Touch)` with the end midpoint as anchor.
- `editor_shortcut_at(a, action, x, y, src) -> bool` (new; `editor_shortcut(a, action)` stays as the wrapper with
  `src = Key`, anchored at the last pen point or the screen centre). Every existing gate holds (`owner != None`,
  `pinching`, `sheet_open`, the Pages-view keyboard). The anchor is used by zoom actions from phase 3.
- `ActionSrc { Key, Pad, Touch, Shape, Pen }` in shortcuts.rs (phase 1: Key, Pad, Touch used; Shape from 3, Pen
  from 6).
- Feedback: the action's own refresh is the feedback; nothing is drawn while two fingers move (a PAN or ZOOM paints
  `Fast` at most every 180 ms as today, then `Ui` at PinchEnd).
- Phase 1 new action names (shortcuts.rs ids after `SHORTCUT_NAME_PAGE = 46`, `K_ACTIONS` grows): `links` (MENU,
  the bar), `go-to-page`, `go-back`, `link-areas` (from the Links bar section), and `landscape-fit` (TOGGLE). They
  join library.rs's existing Buttons tables so keys and controller buttons bind them at once; phase 3's registry
  regroups them.

---------------------------------------------------------------------------------------------------------------

## G8. One-finger shapes (D4; phase 3)

UI name: "finger shape" / "figura con el dedo" (never just "shape": Smart shapes own that word).

### G8.1 Arming (capture runs only when all hold)

`settings.gestures` on; at least one one-finger binding exists (a `shape.*` line or a `touch.1.*` swipe); no keyboard
open; the editor is open with no sheet in front except the Links bar; the record card is not up. Not armed: nothing
changes from today and no trail is ever drawn.

### G8.2 Capture (EditorState gains `fs: FsCapture` plus trail fields; all calls are into fingershape.rs)

- Finger lands as `Owner::Link`: `fs_begin(&mut s.fs, x, y, t)` with `t = ev.k_ms` on the Kindle and `ev.t_ms` in
  the runner (the runner stamps offsets; k_ms is 0 headless).
- Each TouchMove: `fs_add(&mut s.fs, x, y, t)`; it returns the state (`Pending`, `Drawing`, `Dead`). Every
  threshold of the capture (the start slop, the lead, the late start, the overtime and the pause) is fingershape's
  and lives in `FS_GATES`; this plan restates none of them. The app's own slops (the tap slop and the 30 px press
  slop) are unchanged, and a move inside the capture's start slop does nothing, as today.
- The app ends a capture with `fs_abort` when: the link hold fires; a `TOUCH_F_BROAD` frame arrives; the cancel up
  arrives (second contact at any delay, `MT_TOOL_PALM`, pen arrival); a PinchBegin arrives (`pinch()` already drops
  the `Owner::Link` finger, editor.rs:3236, and also calls `fs_abort`).
- While down, `editor_tick` calls `fs_tick(&mut s.fs, now)` ONLY when `a.input_drained`, so a queued lift is never
  read as a pause. When it returns `Dead` the trail thins and the rest of the touch is ignored until the lift.
- Lift (real): `fs_finish(&mut s.fs, &mut cand)` runs fingershape's gates (in screen mm, independent of page zoom)
  and returns an `FsGate`: `Ok` goes to `fs_match`; `Swipe(dir)` is a one-finger swipe, dispatched only when
  `touch.1.<dir>` is bound; any other value is a reject (G11) or, before DRAWING, nothing.
- v1: exactly one contact for the whole session.
- `editor_suspend`, `editor_enter`, `editor_leave` call `fs_abort` (no refresh).

### G8.3 Matching and firing

- `fs_match(&cand, &a.gestures.bank, pump)` returns `{ outcome: Fired(action) | Reject | Unclear, d1, thr, d2 }`.
  The caller passes `work_pump` as the pump, which fingershape calls between templates (its cadence is
  fingershape's) so a 32-shape match keeps the pen's ~100 ms budget. The decision rules are fingershape's.
- Before dispatch, the session owner is cleared (`s.owner = Owner::None`) because `editor_shortcut` refuses while an
  owner is set.
- Match: `compose(a, trail_rect)`, `app_invalidate(a, trail_rect, RefreshMode::Ui)`, then
  `editor_shortcut_at(a, action, cx, cy, ActionSrc::Shape)` anchored at the candidate's box centre. The cleanup GL16
  and the action's own refresh go out in the same flush; full-screen actions carry the trail away. A toast names
  the action (its label, 1200 ms) unless the action toasted itself (`a.toast_until` changed during dispatch) or
  opened a full-screen view. `editor_shortcut_at` returning false is treated as a reject.
- One-finger swipe (`touch.1.<dir>` bound): dispatched with `ActionSrc::Touch`, same cleanup.
- Reject: section G11.

### G8.4 Where shapes act

Anywhere on the page, at any zoom, upright or sideways (coordinates are logical, so "up" is the user's up). Shapes
act through open menus exactly as a key bound to the same action. They never start on the panel, a sheet, the card,
the tool picker or the Pages view (those take another owner at the down). No shapes in the Pages view, the Index
map, the Library or Settings in v1, except the record card.

### G8.5 The clock corner

A shape may start in the corner zone: the down reaches `Owner::Link`, the capture runs, and the lift is moved, so
`clock_corner_tap` is false. A tap there never leaves PENDING, so capture never consumes it and no trail is drawn.

### G8.6 Aborts

- Second finger or pinch: the trail rect joins the pinch's first refresh, or gets a GL16 if the pinch never
  commits.
- Pen arrival: restore the trail rect with `RefreshMode::Fast` and add it to the writing settle rect (never a GL16
  while the pen is down); the settle's GL16 cleans it.
- Sleep, rotation, leaving the editor: no refresh (the screen is redrawn anyway).

### G8.7 With the Links bar up

The bar is the only menu that leaves the page live under a finger. A finger whose capture reaches DRAWING over the
page is a shape, not "tap the page to stay" (the bar commits only within the 30 px slop). Its lift dispatches as a key would: the
bar's own shape (`shape.links`) closes it back to the origin page (D2's cancel), another menu's shape swaps, anything
else is refused and fades.

---------------------------------------------------------------------------------------------------------------

## G9. Recording: Settings > Gestures (phase 3; pen rows phase 6)

### G9.1 The sheet

- Settings bottom row (library.rs:1430-1445): KOReader 420 -> 300 px; Controller, Gestures, Updates share the
  middle (about 280 px upright Gothic, 282 Classic, 306 sideways); Gestures uses glyph F07CB (new `IconKind`),
  "Gestures" / "Gestos", not gated by Enable shortcuts. Raw taps in scenarios and SIM.md move (Controller 746 1930,
  Updates 1152 1930, sideways 1062 1695).
- `Modal::Gestures = 17`, probe name `"gestures"` in `SHEETS`. The Buttons sheet code parametrized by a `BindKind`
  descriptor `{ title, prompt, sources, text, learn, forget, reset }`, not a copy.
- List: "Gestures: on (tap to change)" / "Gestos: activados (toca para cambiar)", Editing..., Tools..., Pages...,
  Index..., Menus..., Screen..., System... (section 12.5), Reset gestures / Restablecer los gestos, Done / Listo (10
  rows, under the sideways limit of 11).
- Rows: "Undo: 2-finger tap" / "Deshacer: toque con 2 dedos"; with a shape, a 96 x 96 thumbnail (`fs_draw_thumb`)
  in a 120 px right column; a shape-only row reads "Undo: finger shape" / "Deshacer: figura con el dedo"; a carried
  newer line reads "finger shape (newer version)" / "figura con el dedo (versión más nueva)", no thumbnail.
  `hold-eraser` is listed from phase 6 with the pen as its only gesture kind (before that no gesture can hold it, so
  its row is absent); the gestures switch, Enable shortcuts and the Bluetooth controller are listed like any
  toggle.

### G9.2 Record mode

- Tapping a row opens the record card and sets `TgView.record = true`: touchgest makes no zoom or pan side effects
  and reports every class and rejection (negative Gesture values) with the runtime constants (one table; a unit test
  asserts it). The Library's event fn handles Gesture before its `ph < 0` return.
- The card (1400 px wide, about 1000 px tall so it fits sideways): title 64 px "Do the gesture for %s" / "Haz el
  gesto para %s"; body 44 px, phase 3: "Draw a shape with one finger, or swipe or tap with two fingers." / "Dibuja
  una figura con un dedo, o desliza o toca con dos dedos."; phase 6 replaces it with "Draw a shape with one finger,
  swipe or tap with two fingers, or use the pen's button." / "Dibuja una figura con un dedo, desliza o toca con dos
  dedos, o usa el botón del lápiz." (two msgids); three 260 x 260 dashed numbered slots; one progress/reason line
  (44 px, two lines at 34 px if needed; only that line refreshed, GL16); footer 34 px "One stroke, without lifting.
  Two turns or a loop." / "De un trazo, sin levantar el dedo. Dos giros o un bucle." (the plan's edge footer is
  gone with the edge bands); buttons Use / Try again / Cancel ("Usar" / "Otra vez" / "Cancelar").
- A shape is drawn anywhere on the card except the button row, with the same trail; a touch that starts on a button
  and reaches DRAWING becomes a shape and the press is dropped.

### G9.3 The flow (the first try decides the kind)

- Two- or three-finger pattern: its name at 64 px, Use / Try again / Cancel. If another action holds it: "Now for
  %s. Use moves it here." / "Ahora es para %s. Usar lo trae aquí."; Use toasts "That gesture is no longer for %s." /
  "Ese gesto ya no es para %s."
- Multi-finger rejection lines (GL16 of the line): "The pen is near: move it away." / "El lápiz está cerca:
  apártalo."; "Put the fingers down together." / "Apoya los dedos a la vez."; "Swipe in a straight line." /
  "Desliza en línea recta."; "Swipe farther." / "Desliza más lejos."; "Move both fingers the same way." / "Mueve los
  dos dedos igual."; "Your hand moved: try again." / "La mano se movió: inténtalo otra vez."; "Two fingers spreading
  is kept for zoom." / "Separar dos dedos es para el zoom."; and, for a one-finger TAP or HOLD only, "One finger is
  kept for taps and links." / "Un dedo es para tocar y enlaces." (not for a corner tap).
- A fast straight one-finger line records as "1 finger right" (TOUCH source, `touch.1.<dir>`).
- A non-straight one-finger stroke starts a shape:
  - Sample 1: `fs_finish` gates, then the quick first stage of `fs_record_step` (fingershape's slip check).
    Accepted: "Good. Draw it again (2 of 3)." / "Bien. Dibújala otra vez (2 de 3)."
  - Samples 2 and 3: when `fs_record_step` returns `Inconsistent` (fingershape decides how close is close enough),
    "Not like the first one: draw it again." / "No se parece a la primera: otra vez." (slot stays empty). After 2: "Once more makes it surer, or Use." / "Una vez
    más la afina, o Usar."
  - Before Use is enabled: separation against every other action's shape ("That is the shape for %s. Use moves it
    here." / "Es la figura de %s. Usar la trae aquí.", Use then toasts "That shape is no longer for %s." / "Esa figura
    ya no es para %s."; or "Too close to the shape for %s." / "Muy parecida a la figura de %s.", samples discarded),
    then the full slip bank, resumable (`fs_record_step` with a path budget per tick that the app chooses so no
    tick exceeds ~100 ms), showing "Checking that a slip cannot draw it..." /
    "Comprobando que un roce no la trace..." (about 1-2 s on the Kindle); a hit: "Too easy to make by accident." /
    "Demasiado fácil de hacer sin querer." plus "Give it two turns or a loop." / "Dale dos giros o un bucle."
  - Refusals (the design's 6.4, copied verbatim with their ES): too small, too slow, late start, paused, straight,
    tangled, broad contact, two strokes (a refused straight stroke followed within 700 ms by another), pen near, late
    second finger, full ("Grimoire keeps up to 32 shapes." / "Grimoire guarda hasta 32 figuras."), newer-line
    replace warning ("This replaces a shape from a newer Grimoire." / "Esto sustituye una figura de un Grimoire más
    nuevo.").
- Use: gestures.ini written atomically; the in-memory map changes only after the write succeeds; the card closes
  and the row shows its thumbnail. A new shape replaces the action's old shape; its touch and pen bindings stay.
- Timeout 10 s only before the first attempt; none once a sample exists. A second tap on the row stops recording.
  Rotation relayouts the card and keeps the samples.

### G9.4 Clear, Reset, binding rule

- Hold a row 0.6 s (`ROW_HOLD_MS`, library.rs:727): clears that row's touch, shape and pen gestures; a cleared
  built-in is written `=nothing`; toast "No gestures for %s now." / "Ya no hay gestos para %s."
- Reset gestures deletes gestures.ini; when shapes exist it confirms first: "Reset gestures? Your %d finger shapes
  will be deleted." / "¿Restablecer los gestos? Se borrarán tus %d figuras." (`ng_` plural forms).
- Binding rule, both sheets: per source (TOUCH, SHAPE, PEN, PAD) a binding does one action and an action has at most
  one binding per source; keys keep shortcuts.ini's many-to-one; built-in duplicates stay legal (the pen's triple
  and hold). So Pages can be 2 fingers down, a finger shape and a pen double-click at once (what D5 needs where the
  page scrolls).
- Every toggle assignable (phase 3, plan steps 16-17): the registry's sources mask includes TOUCH, SHAPE and (phase 6)
  PEN for every action, with three exceptions (section 12.4): `hold-eraser` is KEY, PAD and (phase 6) PEN, since a
  touch pattern or shape has no release; `shortcuts` (Enable shortcuts) and `bluetooth-controller` are TOUCH, SHAPE
  and PEN only, since a key or controller binding would switch off its own source. `gestures` takes every source
  (turning gestures off with a gesture toasts "Gestures off. Settings > Gestures turns them back on." / "Gestos
  desactivados. Ajustes > Gestos los vuelve a activar."). `landscape-fit` is KEY, PAD, TOUCH, SHAPE, PEN.

---------------------------------------------------------------------------------------------------------------

## G10. gestures.ini (D8: one file for touch, shapes and pen)

In `store_root()` beside shortcuts.ini and controller.ini. Grammar: shortcuts.ini's ('#' and ';' comments, trimmed
key=value, a later duplicate wins, keys case-insensitive), read through shortcuts.rs's bounded reader at 64 KiB.

```
# Grimoire: gestures (Settings > Gestures). Built in unless a line says otherwise:
# touch.2.down=pages, touch.2.up=links, touch.2.left=next-page, touch.2.right=previous-page, pen.Button.triple=clear-page, pen.Button.hold=clear-page
# shape.ACTION=1,N,S1,S2[,S3]: a finger shape you drew; format 1, N samples; each sample is 32 points in drawing order, x then y as two hex digits, the larger side spanning 00-ff.
touch.2.tap=undo
touch.2.left=nothing
pen.Button.double=links
shape.undo=1,3,<128 hex>,<128 hex>,<128 hex>
```

- Keys: `touch.<1|2|3>.<up|down|left|right|tap>` (`touch.1.tap` refused: reserved), `shape.<action>`,
  `pen.<Button|Button2|Eraser>.<tap|double|triple|hold|drag>`.
- Values: an action file name, `nothing`, or `pan|zoom` in a pen drag slot. Shape values: format 1, N in {2, 3},
  exactly N samples of 128 hex digits, parsed and formatted by fingershape's codec (`fs_line_parse`,
  `fs_line_format`; the sample encoding is the codec's). Derived data is computed at load by `fs_prepare`, never
  stored.
- Overlay model: built-ins are never written; a line overrides one; `=nothing` switches one off; only set slots are
  written; with none left the file is removed; Reset deletes it. `=nothing` never touches shape lines.
- Validation: malformed lines are refused and counted (toasted when the sheet opens, gone at the next write): bad
  key, `touch.1.tap`, a shape line the codec reports `Malformed` (wrong count, bad hex, a degenerate sample), or an
  action without SHAPE in its sources. Carried verbatim and written back: well-formed lines this
  version cannot use (unknown version, unknown action, unknown pen slot), each up to 1 KiB, 16 KiB in all (the plan's
  2 KiB carry is too small for shape lines of about 420 B). Limits: 32 shape lines (`FS_SHAPES_MAX`).
- An unreadable file: built-ins only, no shapes, never overwritten; the sheet offers Reset.
- Read in `app_init` (next to `shortcuts_load_all`), by the Controller sheet's Reload, and each time the Gestures
  sheet opens; held in `App.gestures: GestureMap`.
- Phases: phase 1 reads touch lines (a hand-edited file works; a decision recorded in section 19) and carries pen
  and shape lines without acting on them; phase 3 writes and adds shapes; phase 6 makes pen lines live. It cannot live in settings.ini: dotted `pen.` and `touch.`
  keys are calibration there (store.rs `settings_cb`, test_probe.rs).
- FORMAT.md documents it, plus the moving-between-versions row (older versions ignore it).

gesturemap.rs API (ours):

```rust
pub enum GestureKey { Touch { fingers: u8, kind: TouchKind }, Pen { button: PenBtn, kind: PenKind } }
pub fn gesturemap_defaults(m: &mut GestureMap);
pub fn gesturemap_load(m: &mut GestureMap, path: &str) -> i32;            // refused count, or -1 unreadable
pub fn gesturemap_action(m: &GestureMap, k: GestureKey) -> i32;           // 0 = nothing
pub fn gesturemap_key_for(m: &GestureMap, action: i32, src: ActionSrc) -> Option<GestureKey>;
pub fn gesturemap_mirror(k: GestureKey) -> GestureKey;
pub fn gesturemap_set(m: &mut GestureMap, k: GestureKey, action: i32) -> i32;      // phase 3, returns the loser
pub fn gesturemap_shape_set(m: &mut GestureMap, action: i32, s: &FsShape) -> i32;  // phase 3
pub fn gesturemap_save(m: &GestureMap, path: &str) -> bool;                         // phase 3, atomic
```

---------------------------------------------------------------------------------------------------------------

## G11. Feedback on e-ink (and the flash budget)

| Event | Refresh |
|---|---|
| Two-finger swipe or tap | nothing while moving; the action's own refresh (Pages view GC16, bar GL16, page turn GC16 via `go_to_page`) |
| PAN / ZOOM | `Fast` repaint at most every 180 ms of t_ms (today's), `Ui` at PinchEnd if it moved |
| Panel press cancelled by a second finger | `Fast` restore (was `Ui`) |
| Page-turn end toast | toast only (no page refresh) |
| Trail frames (phase 3) | round 7 px black dots every 20 px with a 2 px white halo (canvas dash-poly dot mode, plan step 6), into `a.screen` only (never `page_canvas`, never a stroke, never .qpg, PDF, thumbnail or sleep image); new dots' rect as `app_invalidate(rect, Fast)` then its own `app_flush` at once (as the keyboard field does), so a trail frame never merges into another GL16; at most one per 40 ms (25 Hz); plain DU, not live ink, not pen mode; the first frame draws the path since landing |
| Match | `compose(trail_rect)` + `Ui`, flushed with the action's refresh |
| Reject | at the lift, compose the trail rect and redraw every other dot (one `Fast`: it thins), then 250 ms later from `editor_tick` a `Ui` of the rect (the dots fade over GL16's 271 ms, no flash); no toast except once ever (`hints` bit 7): "That is not one of your finger shapes. Settings > Gestures lists them." / "No es una de tus figuras. Ajustes > Gestos las muestra." |
| Abort by pen | `Fast` restore + writing settle (never GL16 while the pen is down) |
| Record card reason line | `Ui` of that line only |

- One GL16 per shape; it counts toward the every-30th promotion to Full (app.rs `app_flush`); DU frames do not;
  never GC16.
- Hard rules hold: screens never call display directly (only `app_invalidate` + `app_flush`); no wait is added; no
  MXCFB wait; no second waiter; nothing on the main thread runs > ~100 ms with the pen near (the match and the slip
  bank are pumped through the hooks fingershape takes).
- Colour panels: a Fast refresh that is not live ink waits for the driver's read; if `display.last_fast` shows `wait`
  on a colour device, the trail drops to 12 Hz there (device check in phase 3).

---------------------------------------------------------------------------------------------------------------

## G12. Sideways, fit the whole page (D5's new setting; phase 1)

- Key: settings.ini `landscape_fit` 0|1, default 1 (missing means 1), undotted. Store: `Settings.landscape_fit:
  bool`, default in `settings_default` (store.rs ~2464), a `keep_bool` arm in `settings_cb`, and a `put_int` line in
  `settings_save` AFTER `language` (the newest known key, store.rs comment "Last of the known keys"), with phase 1's
  other new keys in this order: `gestures`, `landscape_fit`, `hints`, `link_areas`. Every existing line keeps its
  bytes and order; older builds carry the new lines as unknown. Probe `settings.landscape_fit` (app.rs numeric
  table). FORMAT.md settings table row.
- `view_reset` (editor.rs:875): `s.zoom_default = if a.settings.landscape_fit && aw > ah { s.zoom_min } else { aw as
  f32 / s.pgw as f32 };` (zoom_min is already `min(aw/pgw, ah/pgh)`). New `s.zoom_max = (aw as f32 / s.pgw as f32) *
  4.0`, the same expression and order as today's `zoom_default * 4.0`, so the ceiling is bit-identical when the
  setting is off or upright; `view_clamp` (851) and `pinch` (3291) clamp to `zoom_max`.
- `zoomed()` (911) compares `ox` against the rest offset, not 0: with the whole page centred sideways, `ox` is
  negative at rest, and today's `s.ox != 0.0` would call the resting view zoomed (Fit page would redraw for nothing).
  New field `s.rest_ox` set at the end of `view_reset`; `zoomed()` uses `s.ox != s.rest_ox`, which is today's test
  exactly whenever `rest_ox == 0.0`. The `editor.zoom` probe (zoom / zoom_default) reads 1.00 at the whole page.
- Rotation: `editor_enter` calls `view_reset` on a size change (3175-3177): turning sideways lands on the whole page
  with the rotation's usual GC16; upright is unchanged.
- Toggling while sideways: `view_reset` + `compose_all(a, Full)` even if pinched in. Upright: only the toast. Toggled
  from the Library-hosted Display sheet: set `s.view_w = 0` on the closed editor state so the next `editor_enter`
  resets the view.
- Pinch while on: works as today when `pinch_zoom` is on, from the whole page up to `zoom_max`; zooming out stops at
  the whole page. Zoomed in, the view scrolls and D5 applies. A page turn, Fit page or rotation returns to the whole
  page. The setting never changes by pinching.
- UI: Display sheet (dispmenu.rs) checkbox after "Invert colors" (163): "Sideways, show the whole page" / "De lado,
  ver la página entera". Settings has no room (library.rs:1395). `row_h` (74) and `card_rect` (81) count the extra
  row (still within the 100-140 px clamp).
- Action `landscape-fit`: TOGGLE; phase 1 in the Buttons sheet's Pages group (K_PAGES 8 -> 9 rows); phase 3 in the
  registry's Pages group with sources KEY, PAD, TOUCH, SHAPE, PEN. Label "Whole page sideways" / "Página entera de
  lado". Toasts: on "Sideways shows the whole page." / "De lado se ve la página entera."; off "Sideways fills the
  width." / "De lado ocupa todo el ancho."; upright "Applies when the Kindle is sideways." / "Se aplica con el
  Kindle de lado." Keys and controller buttons bind it from phase 1; gestures and shapes from phase 3 (this is
  "assignable like every toggle").
- One-time hint (`hints` bit 8), the first sideways turn after the update with the setting on: "Sideways now shows
  the whole page, so two-finger swipes work. Pinch to zoom in; Display changes this." / "De lado ahora se ve la
  página entera y los gestos de dos dedos funcionan. Pellizca para ampliar; se cambia en Pantalla."
- Scenarios that rotate and depend on the width fit get `# settings: landscape_fit=0`: at least f_batch2,
  y_all_screens, y_pagesize_1_ksc, y_pagesize_2_back, z_4_erase_zoom_rotate; the rest are audited by running the
  suite. The phase 1 owner test notes say it is on by default.

---------------------------------------------------------------------------------------------------------------

## G13. Settings keys and hint bits (gesture-related)

| Key | Values | Default | Phase |
|---|---|---|---|
| `gestures` | 0/1 | 1 | 1 |
| `landscape_fit` | 0/1 | 1 | 1 |
| `hints` bitmask | bit 0 first-open gestures tip, 6 pan hint (section G5), 7 shape reject tip, 8 sideways hint (bits 1-5 belong to links and shapes); bits 0 and 8 show only to upgrading owners (section 13.4) | 0 | 1 (bits 0, 6, 8), 3 (bit 7) |
| `rotation_lock` | 0, 1..4 | 0 | 3 (the action) |

First-open tip (bit 0), the first time a notebook opens after the update, two lines for 6 s: "New: two fingers up
show this page's links. Two fingers down show all pages." / "Novedad: dos dedos hacia arriba muestran los enlaces de
esta página; hacia abajo, todas las páginas."

---------------------------------------------------------------------------------------------------------------

## G14. The fingershape.rs API this plan calls (assumed; verify against claude/rust-index-pure)

The pure branch currently holds an empty slot (`rust/src/fingershape.rs`, commit 887f38a). PLAN V2 calls only these
names; their internals (gates, $P, thresholds, slip bank, codec) are the pure branch's and are not re-planned here.

```rust
pub enum FsState { Idle, Pending, Drawing, Dead }
pub struct FsCapture { /* state, 256-point buffer with in-place decimation, landing, lead, timers */ }
pub fn fs_begin(c: &mut FsCapture, x: f32, y: f32, t_ms: u32);
pub fn fs_add(c: &mut FsCapture, x: f32, y: f32, t_ms: u32) -> FsState;
pub fn fs_tick(c: &mut FsCapture, now_ms: u32) -> FsState;          // fingershape's overtime and pause gates
pub fn fs_abort(c: &mut FsCapture);
pub fn fs_state(c: &FsCapture) -> FsState;
pub fn fs_finish(c: &mut FsCapture, out: &mut FsCandidate) -> FsGate;
//   FsGate: Ok | Swipe(FsDir) | TooFewFrames | TooSmall | TooSlow | LateStart | Paused | Straight | Tangled | Broad
pub fn fs_trail_points(c: &FsCapture, from: usize, out: &mut [(i16, i16)]) -> usize;  // points since the last flush
pub struct FsShape { pub n: u8, pub pts: [[(u8, u8); 32]; 3] }
pub struct FsBank { /* prepared clouds, spread, T_g, ratio per shape; up to FS_SHAPES_MAX */ }
pub fn fs_prepare(bank: &mut FsBank, action: i32, s: &FsShape);
pub fn fs_match(cand: &FsCandidate, bank: &FsBank, pump: &mut dyn FnMut()) -> FsMatch;
//   FsMatch { outcome: Fired(i32) | Reject | Unclear, d1: i32, thr: i32, d2: i32 }
pub struct FsRecordCheck { /* resumable */ }
pub fn fs_record_begin(rc: &mut FsRecordCheck, samples: &[FsCandidate], bank: &FsBank, quick: bool);
pub fn fs_record_step(rc: &mut FsRecordCheck, budget_paths: u32) -> FsRecordStep;
//   FsRecordStep: Working(percent) | Ok(FsShape) | Inconsistent | SameAs(i32) | TooClose(i32) | SlipReachable
pub fn fs_line_parse(v: &[u8], out: &mut FsShape) -> FsLine;          // Ok | Malformed | CarryNewer
pub fn fs_line_format(s: &FsShape, out: &mut String);
pub fn fs_draw_trail(c: &mut Canvas, pts: &[(i16, i16)], from: usize) -> Rect;
pub fn fs_draw_thumb(c: &mut Canvas, r: Rect, s: &FsShape);
pub const FS_SHAPES_MAX: usize = 32;
pub const FS_GATES: FsGates;                                          // one table, runtime == record
// cfg(any(test, feature = "sim")): fingershape_synth
pub fn fs_synth(name: &str, x: f32, y: f32, size_mm: f32, ms: u32, rot_deg: i32, seed: u32, rev: bool,
                out: &mut Vec<(f32, f32, u32)>) -> bool;
```

Coordinates are logical screen px with the 11.8 px/mm scale from `ui_px`; times are u32 ms. If the pure branch ships
the matcher without the capture machine (which the recorded design assigns to fingershape), phase 3 asks the pure
branch to add the `fs_begin` ... `fs_trail_points` surface above; this plan does not write it itself.

---------------------------------------------------------------------------------------------------------------

## G15. Runner, simulator, probes, tests

### G15.1 Runner commands (sim_sdl.rs `script_step_in`; docs/SIM.md command table)

Phase 1, all through the same touchgest the Kindle uses, stamped with t_ms offsets as `drag` and `pinch` do (no
virtual app clock; `k_ms = t_ms` headless). The frame clock is monotonic: each batch starts it at max(`now_ms()`, the
last frame's time), and the runner's `touchgest_tick` uses that same clock, so a contact sequence split by an
`expect` or a `wait` never sees time run backwards:
- `finger ID down|move|up X Y [MAJOR] [TOOL]`, grouped into one contact frame until `frame [MS]` (default 13 ms);
- `swipe N X Y DX DY [STEPS] [SEP] [LANDGAP]`;
- `tapn N X Y [SEP] [MS]` (SEP default 150 px, fingers in a horizontal row centred on X, clamped to the screen);
- `pen hover X Y` and `pen leave` (exist, SIM.md:54) also drive touchgest's pen context; the runner's `pentap` ends
  with a leave stamped at its pen up (a nib tap lifted away), so a later gesture sees the pen gone; contact-level
  scenarios wait past the pen window (`wait 1100`) after pen input;
- `tap panel NAME` / `pentap panel NAME` (symbolic panel taps, resolved through the panel's hits after settling a
  stale panel, as a human tap does; names include `links` from phase 1). They are tested at the top of the existing
  `tap`/`pentap` arm (sim_sdl.rs:1152), before its `scan_floats`, which would read `panel` as 0.
- Legacy `tap`, `touch`, `drag touch` and `pinch` keep their exact event sequences (direct injection, PinchMove
  value 0).

Phase 3: `shape NAME X Y SIZE_MM [MS] [ROT] [SEED] [REV]` (via `fs_synth`), `fpath X1 Y1 X2 Y2 ... [MS]`, `trace
FILE` (F lines as contact frames on their k_ms deltas, P lines into the pen context).

No window emulation: the Rust crate has no SDL window simulator (sim_sdl.rs:8-11; sim_sdl.rs:3 is the C header's
comment), and CLOUD.md forbids running a GUI binary. If the SDL simulator is ever ported, it can feed touchgest the
same frames.

### G15.2 Probes (SIM.md Checks table; one of each family in tests/selftest/pass-rust.qs, a failing one in fail-rust.qs)

- Phase 1, runner-owned via `touchgest_probe`: `touch.contacts`, `touch.class`, `touch.suppressed`, `touch.gesture`
  (last fired, e.g. `2.down`), `touch.reject` (last reason name). App: `gestures.ACTION` (its bindings, e.g.
  `touch.2.down`), `settings.gestures`, `settings.landscape_fit`, `settings.hints`, `editor.scrolls` (`v h` as `0 1`),
  `main.longest_unpumped_ms` (prints `ok` under the runner's bound, else the gap, since `expect` has no numeric
  comparison; see section 14.4 for what it counts).
- Phase 3: `fshape.last` (`<fired|reject|refused|cancel> <action|reason> <d1> <thr> <d2>`), `fshape.fired`,
  `fshape.state`, `fshape.trail` (`x y w h` or empty), `gestures.shape.ACTION` (sample count),
  `gestures.shape_count`, `library.gestures_group`, `library.gestures.learning`.

### G15.3 The touch log (phase 1)

- grimoire.log (qlog!, printf formats, new lines only): the new "input: touch slots" absinfo line (G3.2); while
  input is grabbed (never at the PIN), the
  first 300 multi-contact frames per session (kernel time, slot, tracking id, tool, touch_major, pressure, landing
  and lift gaps); per session median and max TOUCH_MAJOR of single fingertips.
- Opt-in trace: when the owner creates an empty `grimoire/touch-trace`, Grimoire appends `grimoire/touch-trace.txt`
  while grabbed: header `# grimoire touch-trace 1, rotation R, WxH, dpi D`; `F <k_ms> <slot>:<id>:<x>:<y>:<major>:<tool>
  ...` (logical px); `P <k_ms> prox|down|up|leave` (pen state only, never pen coordinates, so no handwriting);
  `E <k_ms> <outcome>` (gesture outcomes in phase 1, shape outcomes from phase 3). 4 MB cap, never exported.
- Owner script (docs/KINDLE.md): 20 two-finger swipes each way, 10 two-finger taps, 5 three-finger taps, writing
  with the palm resting, 10 swipes straight after lifting the pen; for phase 3's freeze also one-finger Z, N, S,
  circle, alpha and triangle (10 each), 10 one-finger swipes each way, 10 one-finger V shapes, 5 min writing with the
  palm resting, 2 min holding and reading.

### G15.4 Unit tests

- Phase 1: `rust/tests/test_touchgest.rs` (pure, host): today's parity (begin/move/end, palm flag, second finger at
  120 ms pinches, at 450 ms is a palm, pen window); pen arrival gives a cancel, never a tap; SYN_DROPPED leaves no
  phantom; landing-order pairing; busy flag; each threshold on both sides (travel 19/21 mm, angle 29/31, landing
  140/160 ms, spread 0.55/0.65, pen distance 35/45 mm, finger ratio 1.9/2.1); per-axis pan (a drag at a scroll
  limit pans, never swipes); diagonal; reversal cancels; resting thumb ignored, moving palm rejects; value 0 legacy
  parity; k_ms survives a 1.2 s read stall; BROAD at T_palm + 1 not T_palm; CANCEL only on cancels.
  `rust/tests/test_gesturemap.rs`: overlay, `=nothing`, refused and carried lines, unreadable never written, mirror
  table. `editor.rs` `#[cfg(test)]`: `view_reset`/`zoomed`/`zoom_max` with landscape_fit on and off (bit-identical
  ceiling and `zoomed()` when off). `test_probe.rs`: the new keys are undotted. `test_shortcuts.rs`: counts with the
  new names. `test_i18n.rs`: new strings fit.
- Phase 3: test_touchgest (hover rule both sides of 60 mm; unlearned mapping falls back; T_palm guard; record mode
  shares constants; record-mode one-finger pen rejection); test_gesturemap (shape lines with overlays, `=nothing`
  leaves shapes, carry limits 1 KiB / 16 KiB, Reset deletes); test_actions (every SHAPE-capable action has labels;
  `hold-eraser` is not TOUCH or SHAPE; `shortcuts` and `bluetooth-controller` are not KEY or PAD). fingershape's own tests belong to the pure branch.

### G15.5 Scenarios

Phase 1 (new files, each with the `# rust-only` header so `make c-check` skips them, section 15.1):
- `t_touch_1_contacts`: a contact-level pinch zooms like the legacy one; a TOOL 2 contact suppresses until all lift;
  a finger resting on a link when the pen arrives does not follow it.
- `g_gest_1`: upright, 2-down opens Pages (GC16), 2-up closes; 2-up opens the bar, 2-down closes it to the origin;
  2-left next page, 2-right previous; last page toast with `editor.pages` unchanged; first page toast.
- `g_gest_1b_unbound`: gestures.ini `touch.2.up=nothing`; the Links button opens the bar, a peek, then 2-down returns
  to the origin and the Pages view does not open (the bar's fixed cancel).
- `g_gest_2_zoomed`: pinched in, a mid-screen vertical drag pans with `editor.zoom` unchanged; a drag from the top
  edge also pans (no bands); at a scroll limit a drag opens nothing; the pan hint once; the panel's Pages chip and
  Links button open their menus; still zoomed, with the bar up 2-down returns to the origin, and with Pages open 2-up
  closes it (non-panning surfaces, G5).
- `g_gest_3_sideways`: rotate 1 with the default: swipes anywhere. `g_gest_3b_sideways_width`, with `# settings:
  landscape_fit=0`: an upward vertical drag pans (the view rests at the top, so a downward one pans nowhere) and
  left/right still turn pages.
- `g_gest_4_pen`: a swipe 300 ms after the pen leaves, 60 mm away, works; at 20 mm `touch.reject` is pen and the
  toast shows once.
- `g_gest_5_pinch_off`: swipes work, spreads do nothing, an upward drag still pans where the page scrolls.
- `g_gest_6_keyboard`: a swipe while naming does nothing. `g_gest_7_asleep`: gestures are dropped.
- `g_gest_8_corner`: a one-finger tap in the zone toggles the clock (feature B's probe) and nothing else: idle, over
  a link area, with the bar up, while peeking, after a gesture; a two-finger swipe whose first finger lands in the
  zone fires and does not toggle.
- `s_fit_1`: rotate 1: `editor.zoom` 1.00 at zoom_min; swipe 2 down mid-screen opens Pages; pinch in then next page
  returns to the whole page; Fit page returns; the Display toggle gives the width fit and persists across restart;
  the key toggles; upright toggle only toasts; the hint once.
- `y_all_screens`: the Display sheet's new row upright and sideways, EN/ES, both styles, CHECK_FIT.
- Every pinch scenario stays green: f_batch2, k_pinch_gate, p_power, v_controller_actions, y_all_screens,
  z_4_erase_zoom_rotate, y_pagesize_2_back, z_a_precise_other_size.

Phase 3: `g_gest_10_hover`, `g_gest_11_three` (rows shown with sim slots 10, hidden with 2),
`g_gest_12_onefinger` (a learned `touch.1.left` fires only from the page, never the panel), `v_gestures_1`..`_3`,
`v_gestures_es`, `s_shape_1_record` .. `s_shape_9_traces` (as the shape design's 10.5, with `s_shape_4_guards`
adding a shape that starts in the corner zone and still fires, and `s_shape_6_scroll` asserting a pan plus a shape
bound to `pages`), `v_gestures_shape_clear_reset`; `main.longest_unpumped_ms is ok` in a 32-shape match. Phase 6:
`v_gestures_4_pen`.

---------------------------------------------------------------------------------------------------------------

# Part L. Links

## 7. The Links bar (`linkbar.rs`) [phase 1; the card from phase 2; the map chip from phase 5]

### 7.1 What it is, and how the editor hosts it

- 'Links' / 'Enlaces': the Index entries this page links to (the owner's "hotbar").
- `linkbar.rs` is an editor-hosted sheet, like `dispmenu.rs`: `EditorState` gains `lb: LinkBar`. API (ours, as
  PHASE1-BRIEF.md 13.1 gives it): `linkbar_build(lb, nb, nav, page, origin, dpi)`, `linkbar_layout(lb, aw, ah)`,
  `linkbar_draw(lb, a, nb, nav, peek) -> Rect`, `linkbar_pointer(lb, a, ph, ev, dirty) -> LbResult`,
  `linkbar_probe(lb, key, out, cap) -> bool`. Opening is the editor's `open_menu(SHORTCUT_LINKS)`, which builds and
  lays out the bar. `LinkBar` and the box cache hold 64-entry arrays, so their `Default` is written by hand (Rust
  derives `Default` for arrays only up to 32), and `EditorState` keeps its derive. The bar's own functions
  never reach back into the editor's state (PORTING "Sheets and keyboards": the editor takes `lb` out for the call
  and puts it back).
- `LbResult { None, Peek(i), Commit, Cancel, Back, NewLink, GoTo, Eye, Index, Pager, Changed }` in phase 1; phase 2
  adds `Card(i)`, phase 5 `MapChip`. The editor acts on each.
- It is a MENU action (`links`, new `SHORTCUT_LINKS`), so it joins `sheet_open` (editor.rs:2032), `front_menu`
  (2718), `menu_action` (2735), `close_front` (2748) and `open_menu` (2782). Its own action closes it (back to the
  origin), another menu's action swaps (commit first if peeking, section 7.8), Close menu closes it.
- While it is up every other shortcut is refused (`sheet_open` is true, so `editor_shortcut`'s non-menu branch
  returns false): no undo, no page turn behind a peek.
- Opening it disarms link mode (`link_stop`).
- New owners: `Owner::Bar` (a pointer that started on the bar: chips, tool row, grip, header) and
  `Owner::Outside` (a swallowed touch outside a sheet: the bar's commit or close on lift; reused by the card, Find
  and Link to...). `Owner::Modal` stays the paper sheet's.
- Pointer routing: while the bar is up, its branch comes first in `editor_event`'s down dispatch (before the
  Pages view, sheets and the panel, editor.rs:3345 onwards): a down on the bar is `Owner::Bar`; any other finger down,
  the panel included, is `Owner::Outside` (no panel press feedback, so "a tap on the panel also stays, and the button
  is not pressed" holds), with one exception decided at the lift: a lift on the panel's Links button (`P_LINKS`) is
  "the Links button again" and cancels to the origin (7.6); a pen down outside commits at once and swallows the
  stroke.
- Compose order (`compose`, editor.rs:1109): page (or peek canvas), eye overlay, panel, editor sheets, the bar, then
  (phase 2) the card. The bar draws above the panel and every editor sheet. `compose` resets `a.toast_floor` at
  editor.rs:1115; the bar sets it to its own top after.

### 7.2 Content (D1: grouped)

- One entry per distinct target among the page's live links: stored areas (phase 2), plus beta.5 links whose ink
  still exists (phase 1: these are all of them). Built from `nav_links_of(nav, pid, &mut first)` (nav.rs:568), each
  target resolved with `nav_link_page(l, nb)` (589); dedupe by target string, count N.
- Order: reading order of each entry's first box. Box tops within 10 mm count as one line, read left to right.
  Entries are numbered 1..n; the eye tags every box with its entry's number.
- Labels: a named target: its Index name (`nav_name`), bold 44 px, cut with `font_draw_left_name`; an unnamed page:
  'Page 20' (existing msgid 'Page %d'), regular; a link to the Index: the Notebook icon (`IconKind::Notebook`,
  existing) and 'The Index' (existing msgid, 'El índice'); a deleted page or unknown target: 'Page not here' /
  'Página no disponible', 50% dithered, a tap toasts 'That link's page is not in this notebook.' (existing msgid), a
  long press (phase 2) offers Delete; a legacy link with no area and no ink is not listed (dormant, as today).
- Right column: 'p. 12' / 'pág. 12' at 34 px, and '×2' on a second line for N = 2.
- Backlinks are not a list section. From phase 5 the header carries a chip '%d pages link here' / '%d páginas
  enlazan aquí' that opens the Index map around this page; hidden when 0.

### 7.3 Layout

Upright (1860 x 2480): card at x 20, 1820 wide, bottom 20 px above the edge. From the top: 16 px padding; a 36 px
grip strip with a 160 x 8 bar; a 100 px header (origin: 'Links on this page (%d)' / 'Enlaces de esta página (%d)'
with an X of 118 x 100 at the right, and a pager ‹ 1/3 › of two 118 x 100 buttons left of the X when there are more
than 6 entries; peek: a back chip '‹ Page 3' of at most 380 px, then 'Showing %s. Tap the page to stay.' /
'Mostrando %s. Toca la página para quedarte.' truncated, then the X); R rows of chips (R = 1..3): 2 columns of 878
x 124, 24 px column gap, 16 px row gap; 20 px gap; a 140 px tool row of four 430 x 140 buttons with 20 px gaps, each
an icon over a 34 px caps word; 20 px padding. Height = 316 + 140 R (456, 596 or 736 px); at least 1724 px of page
stays visible. Chip: a 64 px black disc with the white number in 34 px bold, the name from x + 96, a 150 px right
column; nothing wraps.

Sideways (2480 x 1860): 2440 wide, 3 columns of 784 x 124, at most 2 rows (6 per screen), tool buttons 585 x 140,
height 456 or 596 px (at most 32% of 1860).

Tool row: Go to / Ir a (magnifier F0349); New link / Nuevo enlace (link-plus F0C94, md-link_plus, a new
`IconKind::LinkPlus`; F0339 is md-link_variant, the existing `IconKind::Link` of the panel's button); Areas / Áreas
(eye F0208: off,
the glyph in a 50% ordered dither inside a NORMAL frame so it reads dimmed, not `ButtonStyle::Disabled` gray; on, a
solid eye on a Selected button, since inverted means 'on' inside sheets and only the panel reserves it for the
current tool); Index / Índice (three bubbles F05E3). TRANSLATORS width note: 'about 400 px at 34 px caps'. New
glyphs are `IconKind` entries (ui.rs:22-82, glyph table ~281).

### 7.4 Tap an entry: peek (D2)

1. `app_press_feedback` inverts the chip at once (DU).
2. A peek is a display state only. `s.index`, the open page, its history, the box cache and link hit tests stay on
   the origin until a commit, so autosave, undo or a link tap can never act on the peeked page.
3. The peek canvas comes from a 3-entry canvas cache (`s.ccache`: the origin pinned plus two targets). On a miss it
   loads with the pumped page reader (section 14.4) and renders pumped every 64 rows.
4. `compose` blits the peek canvas at the target's rest view instead of `page_canvas` (`page_canvas` and the
   origin's view are never touched during a peek, so a cancel only stops showing it). The rest zoom comes from one
   helper shared with `view_reset` (`view_rest(aw, ah, pgw, pgh, fit) -> (zoom, zoom_min, zoom_max)`), centred as
   `view_clamp` would, so a peek sideways shows the whole page when `landscape_fit` is on. The page area above the
   card and the card (its selected chip and peek header) go out together as one GL16 present. No GC16.
5. The selected chip keeps an 8 px frame; the header switches to the peek state; the panel's page chip shows the
   peeked number.
6. Peeks never set `nb.last_page` or `info_dirty` and never write thumbnails.
7. Tapping the selected entry again commits, the same as tapping the page.
8. An entry for The Index commits, then opens the Index list (Full).

### 7.5 Commit (stay)

- FINGER outside the card, the panel included: the touch becomes `Owner::Outside`; the commit happens on the LIFT
  within the 30 px slop. Order at the lift: off-screen cancel (`TOUCH_F_CANCEL`) does nothing; `clock_corner_tap`
  true toggles the clock and does nothing else (section 3.2); a lift on the panel's Links button cancels to the
  origin (7.6); otherwise commit, and no panel button is pressed. If a second finger lands first, the
  pending commit is cancelled and the touch becomes a gesture (PinchBegin clears `Owner::Outside`); this fixes the
  trap where the first finger of a two-finger close swipe committed and the gesture then opened Pages. From phase 3,
  a finger whose capture reaches DRAWING is a finger shape (G8.7).
- PEN outside: commits on pen down, and the stroke is swallowed until pen up (it raises the capture-blocked flag,
  section 3.1). That keeps the next stroke on MTK pen mode, since the closing refresh is not raced.
- Commit makes the shown page current: `go_to_page`'s work with the cached canvas swapped into `page_canvas`
  (`std::mem::swap` of the two `Canvas` values, never a copy) instead of a render: save the origin, write its
  thumbnail if needed, make the target the open page without reading it again (the session cache's page when it has
  one, else the peek entry's `Box<Page>` moved into the session cache: the canvas was rendered from that very Page,
  so they match), set `last_page` and `info_dirty`, `view_reset`, close the card, one `compose_all(a,
  RefreshMode::Full)` (GC16, clears the peeks' ghosts, resets the every-30th counter).
- With no peek, the tap outside only closes (GL16 of the card rect) and is swallowed.
- The origin goes on the back list. Committing a page reached through Back pops the list instead, as a browser's
  Back does.

### 7.6 Cancel (back to the origin)

- Triggered by: X; the `links` action or the Links button again (a finger lift on it, peeking or not); two fingers
  down (the bar's fixed cancel, G6.2, whatever is bound); a one-finger drag of 15 mm or more down that starts on the
  grip or header; Close menu; the Back key or a key bound to `go-back` (handled before `editor_shortcut_at`'s
  `sheet_open` refusal, editor.rs:2874, which would otherwise refuse it); leaving the editor or a sleep.
- If peeking: the origin canvas is swapped back at the zoom and offset it had; `compose_all` with `Ui`, or `Full`
  after 3 or more peeks (GL16 ghosting builds up).
- Otherwise: a GL16 of the card rect.

### 7.7 Back chip and back list

- While peeking, '‹ Page 3' shows the origin again (canvas swap, GL16 of the page area); the bar stays.
- Not peeking and the back list not empty: 'Back to %s' / 'Volver a %s' ('Back to Chapter 1') peeks the top of the
  back list.
- The back list (`s.back`, up to 32 page ids per open notebook, memory only): pushed by link follows, bar commits,
  Find results, Index map Opens and pages opened from the Pages view; not by next or previous page; cleared at
  notebook close. The `go-back` action pops it with `go_to_page`; empty: 'Nothing to go back to.' / 'No hay a dónde
  volver.'

### 7.8 Acts on the shown page, so it commits first

- New link: commit, then link mode on that page; the bar folds into the link-mode strip and comes back afterwards
  with the new entry (phase 2; phase 1: commit, close the bar, then today's `link_start`).
- Go to: commit, then the Find sheet.
- Index: commit, then the Index map (phase 5), or the Pages view's Index tab before phase 5.
- Another menu's button or key: commit, then swap. This needs explicit code in `editor_shortcut_at`'s menu branch
  (editor.rs:2832), since `close_front` takes no action argument: when the bar is in front and peeking and the action
  is another menu (not `links`, not `close-menu`), call the commit, then `open_menu(action)`. Two fingers down never
  takes this path (the fixed cancel).
- Areas: toggles `settings.link_areas` and saves; a GL16 of the button plus a GL16 of the union of the visible boxes,
  flushed separately when far apart; no toast; the first time the tip (`hints` bit 2).
- Pager: ‹ and › redraw the chip grid only, GL16.

### 7.9 Long press an entry (R2, D1) [phase 2]

- 600 ms within the slop (timed in `editor_tick`, like `LINK_HOLD_MS`).
- If peeking, first return to the origin with a GL16, because the links live there.
- The link card (section 9.5) opens above the bar, aligned with the chip's column; the entry's boxes get the heavy
  dotted border (four DU strips each, their own flush).
- N = 1: the page card's rows exactly. N > 1: 'Rename in the Index' (the entry, D3), 'Change where they go...' /
  'Cambiar a dónde llevan...', 'Delete %d links' / 'Borrar %d enlaces' (`ng_` plural), and the line 'To change one,
  hold it on the page.' / 'Para cambiar uno, mantenlo pulsado en la página.'; no Redraw row. Delete and retarget
  are one `ActKind::Link` step with N records.
- Phase 1: not wired; a hold past 600 ms cancels the press silently (no peek, no Pages view).

### 7.10 Instant jumps

- Canvas cache: `page_canvas` (the origin, never touched by a peek) plus 2 extra canvases, so at most 3 x 4.6 MB
  (5.3 MB at 1986 x 2648). Key: page id + page size + `bgimage_generation()`. No edit generation is needed in phase
  1: no page can be edited while the bar is up, and the cache is freed when the bar closes. A commit swaps structs,
  never copies. An allocation failure (`try_reserve_exact` in `canvas_init`) makes the tap a plain jump (close the
  bar, `go_to_page`, the origin on the back list): the same pixels, slower, no peek.
- Prefetch in `editor_tick` only when `a.input_drained` and `s.owner == Owner::None`: the visible entries' targets,
  then the top of the back list; one 160-row band per tick (a clipped render through `canvas_set_clip`); pages not
  in memory load with the pumped page reader; it stops at the first contact (between bands).

### 7.11 Opening cost and states

- The card is composed into `App.screen` and its rect GL16'd: about 50-90 ms of CPU plus 271 ms.
- Read-only page: the bar opens for browsing, New link drawn disabled; a tap on it toasts 'This page is read-only.'
  / 'Esta página es de solo lectura.'
- links.ini unreadable: 'Links could not be read.' / 'No se pudieron leer los enlaces.'; New link and Areas
  disabled; New link toasts the reason when tapped.

### 7.12 Checklist for every new surface (bar, card, link-mode strip, Link to..., Find, Index map, record card)

- joins `sheet_open`, `front_menu`, `menu_action`, `close_front`, `open_menu` (the map and the record card as their
  hosts require);
- sets `toast_floor` after compose;
- swallows its tap outside on the lift (pen on down, raising the capture-blocked flag);
- never claims the clock corner tap, and never covers the zone;
- never draws while the pen is down (Fast only, no GL16, no toast);
- every keyboard goes through the keyboard host (section 8.1);
- every string EN and ES with TRANSLATORS width notes, visited in `y_all_screens` upright and sideways under
  CHECK_FIT.

### 7.13 Probes (editor_probe)

`editor.bar` (`0`, `open`, `peek`), `editor.bar.entries`, `editor.bar.N` (`p12 x2`, `index`, `missing`),
`editor.bar.peek` (the shown page or 0), `editor.bar.rect`, `editor.back` (`depth top`), `editor.canvas_cache`
(`hits misses entries`), `editor.areas`, `editor.panel_at.X.Y`, `editor.modal`'s new values `links` and `find`,
`notebook.last_page`. All documented in docs/SIM.md's Checks table (PHASE1-BRIEF.md 6.2 has the exact values).

---------------------------------------------------------------------------------------------------------------

## 8. The keyboard host and the Find sheet [phase 1; PICK mode phase 2]

### 8.1 The keyboard host

- Already in the crate: `app_keyboard_pointer` (app.rs:1434) draws only when a key changed something, flushes the
  field then the key as separate Fast refreshes, and `app_keyboard_tick` (1465) settles with one GL16 after 900 ms.
  Every new keyboard (Find, Rename, New page..., the map's Rename) uses exactly these.
- Added in phase 1, a split host: today `app_keyboard_pointer` has already invalidated the key's rect (unflushed)
  when it returns `Changed` (app.rs:1450-1455), so a caller that flushes its own rows then would present rows and key
  as one union from the rows down to the key, the overlapping refresh the host exists to avoid (app.rs:1444-1448).
  New `app_keyboard_pointer_split(a, kb, phase, ev) -> (KbResult, Rect)` does the same draw and field flush but
  returns the key's rect without invalidating it; the caller draws and flushes its rows (Fast, its own
  `app_flush`), then `app_invalidate(key, Fast)`. `app_keyboard_pointer` becomes the split call plus that
  invalidate, byte-identical for its callers. Find uses the split host: field, rows, key.
- Added in phase 1: the toast floor at the keyboard's top while it is open.
- Added in phase 2 (PICK mode is its first caller): a dock. `keyboard_draw` recomputes `kb.area` from
  `modal_rect(screen)` on every draw (keyboard.rs:382-385), so the dock is a `dock_top` field that `modal_rect`
  (keyboard.rs:239) honours, set by `keyboard_set_dock(kb, top)`; bottom stays the default and byte-identical.
  Docked at the top, the modal starts at y = z + 8 (below the clock corner zone, section 3.2), not 20.
- Never hosted inside a full-screen compose (the Pages view redraws everything per call).
- `keyboard.draws` (existing runner probe) is asserted in every scenario that types.

### 8.2 The Find sheet (`findsheet.rs`)

- Over the page, hosted by the editor: `EditorState.find: FindSheet { open, mode: Go | Pick, kb, rows, rect }`.
  Field 'Page number or name' / 'Número o nombre de página' (the field is inside the keyboard modal, with the keys
  right under it). Up to 5 result rows upright and 3 sideways, in a card above the keyboard modal whose top is never
  above y = z + 8; while typing they refresh between the field and the key (the split host, 8.1). No match: 'No page
  names match.' / 'Ningún nombre coincide.'
- Digits: 'Go to page %d' / 'Ir a la página %d', or 'There is no page %d (%d pages).' / 'No hay página %d (%d
  páginas).'
- Letters: Index names matched with `i18n_name_match` (store.rs's `fold_next` and `cmp_name`, store.rs:1444 and 1487,
  exposed through i18n.rs, no behaviour change for their existing callers): prefix first, then word start, then
  substring; accent-folded, case-insensitive.
- GO mode: a result goes there (`go_to_page`, GC16) and pushes the back list.
- PICK mode (phase 2): from Link to...'s Search; the keyboard docks on the side away from the area (the dock, 8.1).
- Opened by the bar's Go to (commit first), the `go-to-page` action, and the Pages view's 'Go to page...' (PG_JUMP,
  pagesview.rs ~540), which from phase 1 closes the Pages view and opens Find over the page in one Full refresh (the
  Pages-view number pad is retired; `j_pages_keyboard` is updated deliberately and keeps its `keyboard.draws` bound).
- A tap outside (not the corner) closes it and is swallowed; Close menu and its own action close it. It is a MENU
  action (`go-to-page`) in the menu family.
- Probes: `editor.find` (`0`, `go`, `pick`), `editor.find.rows`, `editor.find.N`.

---------------------------------------------------------------------------------------------------------------

## 9. Link areas, making, the eye, the long-press card (R1, R2, D1, D3)

### 9.1 Model (R1) [phase 2]

- A link is (page id, seed, target, area). The area is an axis-aligned rectangle in page px at the notebook's page
  size (`store_notebook_page_size`), stored on an `area.` line beside the unchanged `link.` line (section 13.1).
  `NavLink` (nav.rs:28) gains `area: Rect` and `has_area: bool`.
- The area is invisible: never in `page_canvas`, thumbnails, the share page, the PDF page image or the sleep image.
  Only `App.screen` overlays show it: the eye, the dashed lasso while drawing, the dotted rectangle until a target
  is picked, the heavy border of a long press.
- The seed stays the link's id; `nav_new_seed` (nav.rs:708) keeps it unique per page and already makes new ink
  avoid link seeds.
- A visible border is the owner's own ink, with its own seed, never part of the link. Giving a border the link's
  seed is deliberately not done (it would bring back the dependence on ink and the precise-eraser quirk).

### 9.2 Hit test and the box cache [phase 1 legacy boxes; phase 2 areas]

- Each link's box is its area; an unmigrated legacy link uses the point box of its ink strokes (`link_box`,
  nav.rs:603), erase strokes ignored. Padded to at least 7 mm a side around its centre (`NAV_LINK_MIN_MM`).
- Phase 2: the SMALLEST padded box under the finger wins, a tie to the lower seed, replacing "last stroke wins"
  (`nav_link_at`, nav.rs:683, and `nav_link_areas`, 663, become area-first). Phase 1 keeps today's rule.
- The editor caches the open page's boxes (at most 64, with entry numbers) in `EditorState.boxes`. It is marked
  stale on page change, link edits and migration, and while legacy links remain on the page also on `draw_end`,
  erase end, `undo_redo` and clear; it is rebuilt lazily, only when the eye composes or the bar opens, because
  `nav_link_areas` walks every stroke once per link. A tap still uses `nav_link_at` as today.
- `nav_link_areas` prefers areas, so the PDF's /Link annotations follow with no change to `export.rs`. Links to the
  Index stay out of the PDF, as today.

### 9.3 Making one, without ink (R1) [phase 2]

1. ARM from the bar's New link, the `add-link` action or the Index tab's Add a link. Refusals say why: read-only
   (the read-only toast); links.ini unreadable: 'Links could not be read in this notebook.' / 'No se pudieron leer
   los enlaces de este cuaderno.'; 64 links: 'This page has 64 links: delete one to add another.' / 'Esta página
   tiene 64 enlaces: borra uno para añadir otro.'
2. STRIP: 1820 x 130 at the bottom, GL16: 'Draw around what links.' / 'Rodea lo que enlaza.' (new msgid; the old
   'Draw around what links. Then tap where it goes.' retires) with Cancel (260 x 100). add-link again, Close menu,
   Cancel, a page change or leaving the editor disarms it; nothing is written.
3. LASSO: the next pen contact is `Owner::Lasso` whatever the tool, eraser end included; smart shapes never apply;
   it raises the capture-blocked flag. Points in page px, decimated to at most 4096, draw a 4 px black dashed line
   (12 on, 8 off) with `canvas_dash_poly` as an overlay that `compose` redraws; each batch goes out as
   `RefreshMode::Fast` with its own flush (not the live ink style, section 3.1). Nothing touches `page_canvas`, the
   page or history.
4. LIFT: under 5 mm both ways (`LINK_BOX_MIN_MM`, editor.rs:24): the dashes are recomposed away, toast 'Too small for
   a link: draw around what links.' (existing msgid), mode stays. Otherwise area = the points' box clamped to the
   page; one GL16 of the union swaps the dashes for a dotted rectangle (3 px dashes, white halo).
5. LINK TO... (the bar's frame in pick mode, `linkbar.rs`): title 'Where does the link go?' / '¿Adónde va el
   enlace?'; docks at the TOP (below the clock corner, y = z + 8) when the area's centre is in the lower 45% of the
   screen, else at the bottom; chips New page... / Página nueva..., The Index / El índice, up to 4 Recent targets
   (this session's link targets and last visited pages, this page left out), then the Index entries in Index order,
   paged; tool row Search / Buscar (Find PICK mode), Choose from the pages... / Elegir entre las páginas... (today's
   `pagesview_pick`, the only part of picking mode kept; offered only when Link to... comes from New link, never
   from the card, 9.6), Cancel / Cancelar.
6. PICK: `nav_link_set` plus the area, then `nav_save_links`; on failure `nav_load` rolls back and the toast 'Could
   not save the link.' (existing). On success one `ActKind::Link` step and the toast 'Linked to %s.' / 'Enlazado con
   %s.'; the first time it adds 'Tap it with a finger to go there; hold it for more.' / 'Tócalo con un dedo para
   ir; mantenlo pulsado para más.' (`hints` bit 1). The dotted
   rectangle ends at the pick unless the eye is on. From the bar, the bar comes back.
7. NEW PAGE...: the keyboard host asks 'Name the new page' / 'Nombre de la página nueva' over the page. Done:
   `store_add_page(nb, npages, paper)` (store.rs:1586) at the end, then `nav_set_name` (empty leaves it unnamed),
   then the link; toast 'Linked to the new page %d.' / 'Enlazado con la nueva página %d.'; the editor stays on the
   origin. Cancel adds no page and
   returns to Link to....
8. CANCEL at any point: 'No link made.' (existing). Nothing to undo.

Removed in phase 2: `link_box_drawn` (editor.rs:2276) and `link_cancel`'s undo-the-box (2311); the link-mode path
in `draw_end` that took the next ink stroke as the box.

### 9.4 The eye [phase 1]

- `settings.link_areas` (default 0, remembered), action `link-areas` (TOGGLE, no toast).
- In `compose(r)`, after the page blit and before the panel, for every cached box that meets r: the HIT box (what the
  eye promises is what a finger hits); a 4 px black dashed outline (16 on, 12 off) on a 2 px white halo
  (`canvas_dash_rect`); a 52 x 52 white tag with a 3 px frame and the entry number in 34 px bold at the box's
  top-left, kept inside the screen and moved right of the clock corner zone when it would fall in it.
- 1-bit only, so DU and GL16 look alike and inversion keeps it distinct. At most 64 rect tests per compose; the
  live-ink composes reuse the cache.
- Phase 1 boxes are legacy ink boxes; from phase 2 stored areas.

### 9.5 Long press and the card (`linkcard.rs`) [phase 2]

- At 600 ms within the 30 px slop (`editor_tick`, editor.rs:3652, where `link_hold` fires today) the heavy border
  goes around the box inflated 8 px: 5 px black dashes (20 on, 14 off) on a 3 px white halo on both sides, solid 24 px
  corner Ls. Only the four strips go out by DU, each strip set flushed on its own (`app_flush`), so the gray ink
  inside is not dithered; the card follows by GL16, never one union. On close: GL16 of the card and the strips.
- Card 780 x 666: title row 110 px (target name in 44 px bold plus 'p. 12' at 34 px); four 118 px rows with 12 px
  gaps: Rename in the Index / Renombrar en el índice ('Add page %d to the Index...' / 'Añadir la página %d al
  índice...' for an unnamed page; disabled for The Index, an unknown target or an unwritable nav.ini); Change where it
  goes... / Cambiar a dónde lleva...; Redraw the area / Volver a dibujar la zona; Delete link / Borrar enlace.
  TRANSLATORS width 'about 700 px at 44 px'.
- Placement: 24 px right of the box if it fits within a 20 px margin; else left; else below; else above. Clamped to
  the screen, never covers the box when any placement avoids it, slides horizontally to avoid the panel, and never
  meets the clock corner zone.
- `EditorState.card: LinkCard { open, pid, seed or entry, rows, rect, strips }`, `Owner::Card` for presses on it;
  joins the menu family (Close menu closes it); a tap outside closes it and is swallowed (`Owner::Outside`, on the
  lift, corner excepted).
- Read-only page: only the target and 'This page is read-only.'

### 9.6 Card flows

- Rename (D3): the keyboard host over the page titled 'Name in the Index' / 'Nombre en el índice', card and border
  still visible above it. Done: `nav_set_name(nav, target_pid, name)` (nav.rs:171) plus `nav_save` (422); the old
  name is restored on failure ('Could not save the page name.', existing). Naming an unnamed page toasts 'Page %d is
  in the Index as %s.' / 'La página %d está en el índice como %s.', so the new section start is no surprise. Rename
  is a notebook-wide name, not a page edit: it is not in the page's undo (decision, section 19).
- Change where it goes: Link to... with the current target marked, then an `ActKind::Link` retarget (N records from
  the bar). Opened from the card, Link to... has no 'Choose from the pages...' button (R2: the page stays in view);
  Search (Find PICK mode, over the page) and the Index chips reach every target, and the card flow never shows the
  Pages view.
- Redraw the area: link mode for this link ('Draw the link's new area' / 'Dibuja la nueva zona del enlace'); the
  new box replaces the old as one `ActKind::Link` step.
- Delete link: `nav_link_remove` (nav.rs:556), a save and one step; toast 'Link deleted. Undo brings it back.' /
  'Enlace borrado. Deshacer lo recupera.' (new msgid; 'Link removed. Its ink stays on the page.' retires with the
  Pages-view path).
- Retired in phase 2: `link_hold`'s Pages-view picker, PG_UNLINK and PG_TO_INDEX (pagesview.rs), and the Pages-view
  hold toasts that only they used.

### 9.7 Undo for link edits (`history.rs`) [phase 2]

- `ActKind::Link` (history.rs:15) with `Action.link: Option<Box<LinkEdit>>` = `{ pid, n, records: Vec<(before:
  Option<LinkRec>, after: Option<LinkRec>)> }`, `LinkRec { seed, target, area, has_area }`. N > 1 is the ×N group.
  Memory only, in the page's `PageHistory`.
- `history_undo` / `history_redo` keep their signatures for ink; new `history_undo_kind(h) -> Option<ActKind>` lets
  `undo_redo` (editor.rs:2071) see a Link step on top and apply it itself: pop the step, apply the records to
  `s.nav`, `nav_save_links`, move the step to the other stack, rebuild the box cache, compose the affected boxes'
  union (GL16 if the eye is on, else nothing visible changes).
- Save failure: `nav_load` restores what is on disk, the step is dropped (new `history_drop_last_redo` /
  `history_drop_last_undo`), toast 'Could not save the links: that change was not undone.' / 'No se pudieron
  guardar los enlaces: el cambio no se deshizo.' This fixes the history-loses-its-step behaviour.
- Redo after the target page was deleted does nothing. The undo limit (500) frees link records. The panel's Undo is
  enabled by link steps.

### 9.8 Migrating the owner's beta.5 links (R1) [phase 2]

- A `link.` line with no `area.` line is LEGACY and keeps working exactly as today through the ink-box fallback, in
  the editor, the bar, the eye and the PDF.
- On page load (`page_at`): each legacy link of that page whose ink exists gets area = the UNPADDED point box of its
  ink strokes (never the 7 mm grown box), erase strokes ignored, rounded outward to whole px
  (`nav_link_migrate(nav, page, pid) -> bool`). A new `EditorState.links_dirty` flag is set.
- Idle: after a notebook opens with legacy lines, `editor_tick` migrates one such page per idle tick
  (`a.input_drained`, no owner): pages in memory directly, others through the pumped page reader.
  `main.longest_unpumped_ms` reads `ok`.
- Flush: `save_page` (editor.rs:1217) saves only a dirty page and never links.ini, so `nav_save_links` (nav.rs:740)
  runs explicitly whenever `links_dirty`: on leaving a page, in the 2 s idle autosave tick, at notebook close and at
  `editor_leave`.
- The lasso ink stays ordinary ink and its seed stays reserved. A lasso hidden by the precise eraser or white ink
  still has its stroke, so it migrates to a working area the eye shows (the silent death at compaction is gone). A
  legacy link with no ink stays dormant and is pruned only with its page. links.ini unwritable: nothing migrates,
  the fallback keeps working.

### 9.9 Copy page [phase 2]

Copy page (editor.rs:2228, `store_duplicate_page`) copies the page's links (same seeds, areas and targets) to the
copy's pid, migrating any legacy link first since the source page is loaded.

---------------------------------------------------------------------------------------------------------------

# Part S. Smart shapes and the Index map

## 10. Smart shapes [phase 4] (calls `shape.rs`)

### 10.1 Trigger (editor.rs)

- Draw, then hold the nib still within 1.0 screen mm (11.8 px) for `shape_hold_ms` (default 550; Quick 400, Normal
  550, Slow 900).
- `draw_sample` (editor.rs:1702) keeps the anchor in O(1): a sample more than 1 mm from the anchor moves it and resets
  its kernel time (`k_ms`).
- `editor_tick` (at least every 30 ms, pen down too) checks the hold by wall time, never by sample spacing (a still
  relayed pen may send nothing), and only when `a.input_drained`, so a backlog is never mistaken for stillness. Once
  per still period.
- Scope: ink strokes of all nine pens, the highlighter and the side button's highlighter. Never erase strokes, white
  ink, the precise or whole-stroke eraser, read-only pages, or the link lasso.

### 10.2 The recognizer (`shape.rs`, pure, called only)

- Assumed call surface (verify against `claude/rust-index-pure`): one function that takes the raw stroke's points in
  page px with their times, the view zoom (so sizes are screen mm: page px = mm x dpi / 25.4 / zoom), the paper kind
  (paper-aware snaps) and the writing-burst flag, and returns either no shape (with a reason for the log) or a kind
  (line, rectangle, square, quad, triangle, ellipse, circle; v1.5 arrow, polyline, arc behind the corpus gate) with
  the snapped points. Thresholds, gates, paper snaps, the vertex rules and the corpus are the pure branch's.
- The app supplies: the burst flag (a stroke started within 600 ms and 8 mm of the last lift needs tolerances 0.7x as
  wide and a 1.5x hold: the hold part is the editor's), the tool, the paper, the zoom.

### 10.3 Accident defences, strongest first

1. A hold only previews; moving on restores your line and the stroke continues untouched.
2. Gates that can say no (the recognizer's): size in screen mm, closure, winding, the scribble budget, fit residuals.
   On the owner's 47 real strokes a hold-only trigger would fire 21 times; these gates gave no preview at written
   size.
3. The writing-burst gate.
4. One Undo removes the shape.
5. A panel toggle plus an adjustable hold, both assignable.
6. Never on erasers, white ink or the lasso.

### 10.4 Preview

1. At an accepted hold, region = union(raw bbox, shape bbox), padded by the brush's max radius + 2.
2. Copy that region of `page_canvas` (the with-raw pixels).
3. `draw_page(region)` renders the page without the stroke in progress (the raw stroke is not added until the
   lift), pumped every 64 rows.
4. Stamp the snapped stroke with the brush (`brush.rs`), calling `work_pump` every 8 points.
5. `show(region, RefreshMode::Fast)`: one DU frame; pen mode kept.

- Moving more than 1.5 mm from the anchor (hysteresis over the 1.0 mm hold radius) puts the saved copy back (a copy
  back into the canvas) with one DU frame. The raw brush state was never touched.
- Lift or leave: the shape is added with ONE `history_push_add`; the raw stroke is never added. No redraw; the region
  joins the settle rect for the usual GL16.
- Holding longer does nothing more. No GL16 and no toast while the pen is down. If the region copy cannot be
  allocated, that stroke gets no preview. An under-ink tile backup comes later only if the shape-log shows region
  renders over about 300 ms.
- Live view (section 3.1): the raw stroke's provisional ink on `/live/pen` is replaced by the page stream's snapped
  stroke.

### 10.5 Storage

The snapped stroke is an ordinary `Stroke` (page.rs:7-21): the raw stroke's tool id (0-9), params, ext bytes, tip
variant and seed; only the points differ, and they are exactly the snapped points `shape.rs` returns (their spacing,
direction, closure, pressure, tilt and times are the recognizer's rules, not re-planned here). No format change, no
ext flag. Older versions draw it as the pen stroke it is.

### 10.6 Undo and shape-last

- One pen-down is one moment, so one Undo removes the shape (the owner's documented undo rule; decided, not asked).
- `shape-last` ('Shape the last stroke' / 'Dar forma al último trazo'): all the gates, no hold, on the page's newest
  ink stroke when the top undo step is that stroke's Add. One `ActKind::Replace` step (raw out, shape in), so Undo
  brings the freehand stroke back; then `draw_page(union)` pumped and a GL16. Otherwise 'No shape in the last
  stroke.' / 'El último trazo no es una forma.' A zero-accident way in for writers who turn the hold off.

### 10.7 The toggle (D7) and settings

- Panel: `P_SHAPES` (new P_ id) directly under the eraser, in the pen-behaviour group. On: the filled glyph F0831 on a
  plain button; off: the outline glyph F0832 with a 4 px diagonal slash (`canvas_slash`). NEVER inverted (on this
  panel inverted means the current tool). `PANEL_BUTTONS` 13; `panel_fit` gives about 0.93 scale sideways with 4
  pins (94 px buttons, about 8 mm); `test_panel.rs` encodes it.
- Tap: `settings.shapes` flips, `settings_save`, GL16 of the button. Toasts 'Smart shapes on. Hold the pen still at
  the end of a shape.' / 'Formas inteligentes activadas. Mantén el lápiz quieto al final de una forma.'; 'Smart shapes
  off.' / 'Formas inteligentes desactivadas.'
- Tool picker: a labelled row under the ink row with two 520 px chips, 'Smart shapes: On' / 'Formas inteligentes:
  activadas' and 'Hold: Normal' / 'Espera: Normal' (with 'Quick' / 'Rápida' and 'Slow' / 'Lenta').
- Actions (registered in phase 4, so the toggle ships assignable): `shapes` (TOGGLE), `shape-hold` (CYCLE, toast
  'Shape hold: Normal (0.55 s)' / 'Espera de forma: Normal (0,55 s)'), `shape-last` (ONE_SHOT).
- settings.ini: `shapes` 0|1 default 1 (on by default, as the brief asks; 2 and 3 reserved), `shape_hold_ms`
  300..1500 default 550.
- One-time tip after the first snap (`hints` bit 5): 'Shape made. Undo takes it away; the panel's shapes button turns
  this off.' / 'Forma hecha. Deshacer la quita; el botón de formas del panel lo desactiva.'

### 10.8 Device tuning

An empty `grimoire/shape-log` file logs each hold's features and decision (size in mm, winding, residuals, corners,
kind or reason, ms taken), never coordinates. One evening of the owner's writing sets the final thresholds, which
are handed to the pure branch.

---------------------------------------------------------------------------------------------------------------

## 11. The Index map [phase 5] (`mapview.rs`, calls `graph.rs`)

### 11.1 Name and entry

- One Index destination, titled 'Index' / 'Índice' with a Graph | List switch. List goes to the Pages view's Index
  tab (the existing list, `draw_index`, pagesview.rs:293), whose header gains a Graph button.
- Opened from the bar's Index button, that Graph button, or the new `index-map` action ('Index map' / 'Mapa del
  índice'). The `index` action keeps its 1.2.0 meaning: the list. A link to The Index still opens the list.
- Full screen like the Pages view, drawn from its own offscreen canvas (screen size, 4.6 MB, freed on close);
  `compose` only blits it when `s.map.open`, so settles and toasts never re-render edges. It joins the menu family and
  the checklist. Open and close are GC16. `Owner::Map` takes its pointer.

### 11.2 Data (no page loads)

- Built from nav.ini, links.ini and `nb.page_ids` with no page loads: this is `graph.rs`'s build, called with `&Nav`
  and `&NotebookInfo` (its data structures are the pure branch's).
- Edges: links between pages, deduplicated per unordered pair, with direction bits and multiplicity. Links to The
  Index: a 16 px lozenge badge (`canvas_lozenge`, canvas.rs:218) at the source dot's upper right, not edges. Degree =
  distinct linked neighbours.
- Nodes in three bands: FORCE nodes (named pages, pages with any link, the open page); SATELLITES (unnamed pages with
  no links that have a named page before them, on phyllotaxis rings around their section head from
  `nav_section_of`, nav.rs:468; more than 60 per section: 60 plus '+N', all at 2x zoom); the TRAY (unnamed, unlinked
  pages before the first named page: 'Not linked yet (%d)' / 'Aún sin enlaces (%d)', below upright, right sideways,
  40 px pitch, notebook order; more than 200 collapses to 'Not linked yet: %d pages. Show' / 'Aún sin enlaces: %d
  páginas. Mostrar').

### 11.3 Layout: what this plan calls, and where it runs

- `graph.rs` owns the model, the seed placement, the relaxation, satellites, tray, incremental placement and the
  radial Around layout. Assumed call surface: build (Nav, NotebookInfo, open page) -> model; seed(model); relax(model,
  steps, pump); place_satellites(model); around(model, centre) -> positions; incremental(model, changed); and the
  graph.ini codec if the pure branch provides it (otherwise `mapview.rs` reads and writes graph.ini, section 13.3).
- Where it runs (ours): up to 300 force nodes in-process with `work_pump` every 2 steps (about 0.25 s on the
  Kindle); more, or Tidy up, in a niced `bgjob.rs` child (a new work kind, fork without exec, `libc::_exit`) that
  writes graph.ini atomically while the view shows the seed. The result is swapped in with one GL16 only while the
  map is open and idle (no input for 500 ms). 2000 force nodes with 4000 links take about 3.4 s in the child.
- A full relayout only when more than 10% of the force nodes changed, or on Tidy up; nothing moves between visits.
- Floats differ between arm64 and armv7, so positions persist in graph.ini and tests assert invariants, never
  coordinates. App-side float code follows PORTING's float rules (`fmla`, no `mul_add`).

### 11.4 Look (orphan shading, as the owner asked)

- 1-bit-safe marks, so DU previews match GL16 settles and inversion keeps them distinct.
- Fill by degree, progressively lighter: 0 a hollow 3 px ring; 1 a 25% Bayer dither (4x4 on 2x2-px cells,
  `canvas_disc_pattern`) inside a ring; 2-3 a 50% dither; 4 or more solid.
- Radius at fit = 14 + 16 x ln(1 + d) / ln(1 + dmax) px (14-30). Unnamed force nodes 0.75x; satellites 6 px.
- A named orphan (in the Index, linked by nothing) is hollow but labelled.
- Open page: a concentric double ring 6 px apart. Selected: an 8 px ring, its edges at 5 px, outgoing edges solid and
  incoming dashed; its label becomes an inverted chip.
- Edges: 2.5 px anti-aliased lines (`canvas_line_aa`, made row-span and bit-identical in this phase), plus 1 px per
  extra link up to 5 px, drawn under white knockout discs (r + 4).
- Labels: named pages in bold 34 px, at most 320 px wide (`font_draw_left_name`), on a white knockout box, placed
  greedily (selected, open page, neighbours, then named pages by degree), 4 candidate spots each, a colliding label
  skipped until zoomed; 'p. N' for unnamed pages only at 2x or in Around.

### 11.5 Screen

- Upright header, 190 px: Back (150 x 150) at x = z + 16, right of the clock corner zone (section 3.2); the title
  'Index' / 'Índice' with Graph | List / Grafo | Lista; an 'All | Around' segment ('Todas | Alrededor', 2 x 300); Fit /
  Ajustar (existing msgid).
- Bottom row, 170 px: - and + (220 x 140); 'Unlinked: Show' / 'Unlinked: Hide' ('Sin enlaces: Mostrar' /
  'Sin enlaces: Ocultar'); 'Tidy up' / 'Ordenar', shown once positions exist.
- Selecting a dot replaces the bottom row with a 320 px card: the name at 64 px; 'Page 12. Links to 3, linked from
  2.' / 'Página 12. Enlaza a 3, enlazada desde 2.'; Open / Abrir (existing msgid), Around it / Alrededor, Rename /
  Renombrar (existing msgid; the keyboard host over the map).
- Sideways: the same, with the tray on the right.

### 11.6 Interaction

- Tap a dot: the nearest within max(r x zoom, 59 px = 5 mm) by the view's own hit test (`HitList` holds only 128
  entries, ui.rs:842); GL16 of the old and new selection plus the card.
- Tap the selected dot again, or Open: `go_to_page` (GC16), close the map, push the origin on the back list.
- Tap empty space: deselect (the corner zone excepted).
- One-finger or pen drag: pan; an offset blit by DU at most every 180 ms, then at release a pumped re-render and one
  GL16. A pen drag raises the capture-blocked flag. No finger shapes in the map (G8.4).
- Pinch: zoom 0.25x-6x, a nearest-neighbour scaled DU preview, then GL16; - and + step 1.5x; Fit resets. (Pinches
  reach the map: unlike the Pages view, `editor_event` routes Pinch* to `mapview` while it is open.)
- Around: radial rings (centre, depth 1, depth 2), only members drawn; instant and deterministic.
- Default view: All, centred on the open page at a zoom that fits its 2-step neighbourhood, or everything if dots stay
  at least 10 px. The last mode and the Unlinked choice are remembered per notebook in graph.ini.
- Back or Close menu returns to the page; `pages` swaps to the Pages view.
- Rename (D3): `nav_set_name` + `nav_save`, as the card.

### 11.7 Performance and memory

- Culled to the viewport. Drawing order: edges, knockouts, dots, labels; `work_pump` every 64 edges. About 50-70 ms
  for 100 nodes; worst case (2000 force nodes, 4000 edges at fit) 1-2 s pumped, shown behind the seed frame.
- The 4.6 MB canvas cannot be allocated: 'Not enough memory for the map.' / 'No hay memoria para el mapa.' and the
  List opens instead.

### 11.8 Also in phase 5

- The bar's '%d pages link here' chip (opens the map Around this page).
- The Index tab rows get a small '%d links here' / '%d enlaces aquí' count.
- Probes: `editor.map` (`0`, `graph`), `editor.map.nodes`, `.edges`, `.force`, `.satellites`, `.tray`, `.orphans`,
  `.selected`, `.mode`, `.layout` (`seed`, `running`, `done`), `.node.PID` (`tier r x y visible`), `.labels`.

---------------------------------------------------------------------------------------------------------------

# Part D. Actions, data, e-ink

## 12. Actions: every toggle assignable

### 12.1 One registry (`actions.rs`) [phase 3]

- A UI-free registry beside `shortcuts.rs` holds per action: id (memory only) and file name; label + msgctxt; icon
  and group; pair (the second action of a two-press row); kind; sources mask KEY | PAD | TOUCH | SHAPE | PEN.
- library.rs's `LearnRow` (library.rs:732), the group tables (`K_EDITING` 743, `K_TOOLS` 753, `K_PAGES` 762,
  `K_MENUS` 774, `K_GROUPS` 794) and `action_label` (829-900) move there, with their TRANSLATORS comments and
  msgctxts unchanged. shortcuts.rs keeps parsing and the key/pad map; gesturemap.rs keeps the touch, shape and pen
  bindings. The Buttons sheet, the Gestures sheet, the probes and SHORTCUTS.md's table all read it.
- Phase 1 does not build the registry: its new names join library.rs's existing tables (G7, section 12.4).

### 12.2 Dispatch

`editor_shortcut_at(a, action, x, y, src) -> bool` (phase 1, G7); `editor_shortcut(a, action)` (editor.rs:2816)
stays as the wrapper with `src = Key`, anchored at the last pen point or the screen centre. Zoom actions anchor at the
pen tip, the two-finger midpoint or the finger shape's centre. With src Touch, Shape or Pen, next-page never creates a
page (G6.3).

### 12.3 Kinds and feedback

TOGGLE (self-inverse; a toast only when nothing visible changes), MENU (opens; its own binding closes; another menu's
swaps), STEP pairs (two-press rows), CYCLE (a toast names the new state), ONE_SHOT, MOMENTARY (`hold-eraser` only:
KEY | PAD, and from phase 6 PEN in the pen button's hold and drag slots, which have a release: the eraser at the
press, back at the release; touch patterns and finger shapes have no release, so they cannot hold it).

### 12.4 New action names, by phase (37 today -> 68)

| Phase | Name | Kind | Group | Label EN / ES | Sources |
|---|---|---|---|---|---|
| 1 | `links` | MENU | Index | 'Links on this page' / 'Enlaces de esta página' | KEY PAD TOUCH (+SHAPE 3, +PEN 6) |
| 1 | `go-to-page` | MENU | Index | 'Go to page' / 'Ir a la página' (the existing msgid and translation) | same |
| 1 | `go-back` | ONE_SHOT | Index | 'Back to the page before' / 'Volver a la página anterior' (unbound by default) | same |
| 1 | `link-areas` | TOGGLE | Index | 'Show link areas' / 'Mostrar zonas de enlace' (no toast) | same |
| 1 | `landscape-fit` | TOGGLE | Pages | 'Whole page sideways' / 'Página entera de lado' (G12) | same |
| 3 | `zoom-out`, `zoom-in` | STEP | Pages | 1.25x per press around the anchor | KEY PAD TOUCH SHAPE |
| 3 | `scroll-up`, `scroll-down` | STEP | Pages | 75% of the screen, continuing onto the previous or next page with the zoom kept | same |
| 3 | `invert` | TOGGLE | Screen | Full refresh through the Display sheet's inverted path | same |
| 3 | `light-down`/`light-up`, `warmth-down`/`warmth-up` | STEP | Screen | non-blocking `light_cached()`; unknown level: 'Light not ready yet.' / 'La luz aún no está lista.'; toast 'Light 12 of 24' / 'Luz 12 de 24' (and 'Warmth 12 of 24' / 'Calidez 12 de 24') | same |
| 3 | `rotation-lock` | TOGGLE | Screen | locks the CURRENT orientation (settings `rotation_lock`); toasts 'Rotation locked.' / 'Rotación bloqueada.' and 'Rotation follows the Kindle.' / 'La rotación sigue al Kindle.' | same |
| 3 | `pinch-zoom` | TOGGLE | Screen | with a toast | same |
| 3 | `theme` | CYCLE | Screen | Classic / Gothic; `ui_set_theme`, a leave/enter relayout as `app_set_rotation` does, then GC16 | same |
| 3 | `side-button` | CYCLE | Screen | 0..3; the toast is the Settings sentence without '(tap to change)' | same |
| 3 | `mirror-pdf` | TOGGLE | Screen | with a toast | same |
| 3 | `sleep-screen` | CYCLE | Screen | over `sleep_mode`, toast naming the mode | same |
| 3 | `sleep` | ONE_SHOT | Screen | save now plus a public `app_request_sleep` | same |
| 3 | `gestures` | TOGGLE | System | 'Gestures' / 'Gestos'; off by a gesture toasts 'Gestures off. Settings > Gestures turns them back on.' / 'Gestos desactivados. Ajustes > Gestos los vuelve a activar.' | KEY PAD TOUCH SHAPE |
| 3 | `shortcuts` | TOGGLE | System | 'Enable shortcuts' / 'Activar atajos' (existing msgids); a key or controller binding would switch off its own source | TOUCH SHAPE |
| 3 | `bluetooth-controller` | TOGGLE | System | 'Bluetooth controller' / 'Mando Bluetooth' (existing msgids); the same reason | TOUCH SHAPE |
| 3 | `language` | CYCLE | System | 'Language' / 'Idioma' (existing); over the installed languages, a leave/enter relayout as `theme` does; the toast names the language | KEY PAD TOUCH SHAPE |
| 3 | `sleep-after` | CYCLE | System | 'Sleep after' / 'Reposo tras' (existing); over the Display sheet's values (dispmenu.rs:232-233), the toast names the value | same |
| 3 | `update-channel` | CYCLE | System | 'Update channel' / 'Canal de actualizaciones'; Settings > Updates' three choices (`settings.updates`), the toast names the channel | same |
| 4 | `shapes` | TOGGLE | Editing | section 10.7 | KEY PAD TOUCH SHAPE |
| 4 | `shape-hold` | CYCLE | Editing | section 10.7 | same |
| 4 | `shape-last` | ONE_SHOT | Editing | section 10.6 | same |
| 5 | `index-map` | MENU | Index | 'Index map' / 'Mapa del índice' | same |
| 6 | (every action; `hold-eraser` in the pen button's hold and drag slots only, momentary) | | | PEN source added | +PEN |

- Moved, not new (phase 3): `index`, `add-link` and `name-page` go to the Index group. Existing names keep their
  meaning and file names.
- Not added (the pen plan's names that already exist): panel = `fold-panel`, zoom-reset = `fit-page`, next-pen /
  previous-pen = `next-tool` / `previous-tool`.
- Not an action: calibration (a flow, not a setting). The Gestures sheet's footer and SHORTCUTS.md say so, and say
  which sources the switches that could turn off their own source take. Every toggle or important setting is an
  action: pinch zoom, follow rotation via rotation-lock, mirror PDF, invert, side button, theme, sleep screen, sleep
  after, light and warmth, the eye, Smart shapes and their hold, gestures, Enable shortcuts, the Bluetooth controller,
  language, the update channel, landscape fit, panel fold, eraser mode and white ink.
- Phase 1 places its five names in library.rs's tables: `links`, `go-to-page`, `go-back`, `link-areas` in a new
  'Index...' / 'Índice...' group (`N_GROUPS` 4 -> 5, the Buttons list 6 -> 7 rows), `landscape-fit` in `K_PAGES` (8 ->
  9 rows). The Buttons card is centred (library.rs:583), so its rows move: the raw Buttons-sheet taps in
  v_controller.qs, v_controller_forget.qs, v_controller_switch.qs, v_controller_triggers.qs and y_all_screens.qs are
  updated in phase 1, each checked with `library.buttons_group` and `library.learning`.

### 12.5 Groups (shared by both sheets; each at most 11 rows, so it fits sideways) [phase 3]

- Editing (10 from phase 4): undo, redo, tool:eraser, hold-eraser (Buttons; the Gestures sheet from phase 6, pen
  only), eraser-mode, white, clear-page, shapes, shape-hold, shape-last.
- Tools (11): unchanged.
- Pages (8 rows): previous / next page, first / last page, zoom out / in, scroll up / down, new-page, fit-page,
  pages, landscape-fit.
- Index (8 from phase 5): links, index-map, index, go-to-page, go-back, add-link, name-page, link-areas.
- Menus (7): unchanged.
- Screen (10 rows): invert, light pair, warmth pair, rotation-lock, pinch-zoom, theme, side-button, mirror-pdf,
  sleep-screen, sleep.
- System / Sistema (6 rows; new in phase 3): gestures, shortcuts (Gestures sheet only), bluetooth-controller
  (Gestures sheet only), language, sleep-after, update-channel.
- Lists: Buttons = 7 groups + Reset + Done = 9 rows; Gestures = master row + 7 groups + Reset + Done = 10 rows.

### 12.6 Bindings and limits [phase 3]

- `ShortcutBinding.gamepad: bool` (shortcuts.rs:49-54) becomes `src: ActionSrc`; `shortcuts_lookup`, `_learn`,
  `_forget`, `_pads_for` take the source.
- Per source, a binding does one action, and PAD, TOUCH, SHAPE and PEN give an action at most one binding each; keys
  keep shortcuts.ini's many-to-one. Built-in duplicates stay legal (the D-pad starters; the pen's triple and hold).
  Learning a binding another action holds moves it; the toast names the action that lost it.
- `SHORTCUT_MAX` 64 -> 192 (shortcuts.rs:5); `write_pads`' 4096-byte buffer becomes a computed size; library.rs's
  fixed row arrays (`[0i32; 4]` and friends at 909-912, the 16-entry group arrays) get `GROUP_ROWS_MAX = 12` with a
  `const` assertion over every group.

### 12.7 Settings bottom row [phase 3]

KOReader shrinks from 420 to 300 px; Controller, Gestures and Updates share the middle: (in.w - 300 - 380 - 4 x 30) /
3 = 280 px upright Gothic, 282 Classic, 306 sideways, a label column of 248 px or more, so 'Actualizaciones' (234
px) fits. Centres about 580, 890 and 1200 at y 1930 upright; 863, 1199 and 1536 at y 1695 sideways. Gestures uses
glyph F07CB (new `IconKind`), 'Gestures' / 'Gestos', not gated by Enable shortcuts. The raw scenario taps
(Controller 746 1930, Updates 1152 1930, sideways 1062 1695 at y_all_screens.qs:631), SIM.md and the TRANSLATORS
note ('about 240 px') are updated; `test_i18n.rs` checks the three words fit at 34 px in both styles.

### 12.8 Panel [phases 1 and 4]

`P_LINKS` after Next (phase 1) and `P_SHAPES` under the eraser (phase 4); `PANEL_BUTTONS` 11 -> 12 -> 13
(panel.rs:9); `panel_height` (55) and `panel_fit` (70) must match `draw_panel` (editor.rs:512); sideways with 4 pins
`panel_fit` gives about 0.99 scale at 12 and 0.93 at 13. `test_panel.rs` encodes the fits. Scenarios use symbolic
`tap panel NAME` before either button lands. The collapsed panel (editor.rs:519-538) shows only the tool icon:
unchanged.

---------------------------------------------------------------------------------------------------------------

## 13. Data and formats

The `.qpg` format never changes: no header bump, no tool id >= 10, no ext flag, no chunk holding page coordinates.
Snapped shapes are ordinary strokes; areas, overlays, the bar, the card, the trail and the map are never in pages.

### 13.1 links.ini v2 [phase 2] (2.0.0-beta.5, unreleased, so free to change: R1)

```
# Grimoire: links on pages
link.12=0a1b2c3d,p7
area.12=0a1b2c3d,410,1220,380,140
```

- `link.<pid>=<seed>,<target>`: byte for byte today's grammar (`link_parse`, nav.rs:310). Target `index` or
  `p<pid>`, 1-15 bytes from '!' to '~'.
- `area.<pid>=<seed>,<x>,<y>,<w>,<h>`: decimal page px at `store_notebook_page_size`; 0 <= x, y; 1 <= w, h; x + w and
  y + h <= 6000. The last line for a pid+seed wins. An area line with no matching link line is dropped at the next
  save and logged (`qlog!`). A malformed area line is carried verbatim and its link falls back to the ink box. After a
  page-size change an area is clamped; one left empty means no area.
- Links with an area use it; links without one use their ink box (the beta.5 rule).
- Size: about 63 B per worst-case pair, so 4000 links take about 252 KB plus up to 16 KiB of carried lines: links.ini
  gets its own cap `NAV_LINKS_FILE_MAX = 512 KiB`; nav.ini keeps 256 KiB; `NAV_LINKS_MAX` 4000 and
  `NAV_LINKS_PER_PAGE` 64 unchanged.
- The device's f9d13f5 build on the new file: reads every `link.` line, and ink links keep working there because
  migration never removes ink; carries `area.` lines as unknown lines up to 16 KiB (about 430 links); past that, or
  past 256 KiB, refuses the file (links read-only there, never overwritten). 1.2.0 to beta.4 never read links.ini.
- Writes stay whole and atomic (`write_file_atomic`); an unreadable file is never overwritten.

### 13.2 gestures.ini [phase 1 read; phase 3 write and shapes; phase 6 pen live]

See G10 (grammar, keys, shape lines, overlay model, carry limits 1 KiB per line and 16 KiB total, 64 KiB reader, 32
shape lines). It absorbs the pen plan's pen.ini (D8) and cannot live in settings.ini (dotted `pen.` and `touch.` keys
are calibration there).

### 13.3 graph.ini [phase 5] (per notebook folder; a cache)

```
# Grimoire: the Index map's layout (a cache: deleting it loses nothing)
gen=5f3a9c21
mode=all
unlinked=show
pos.12=-310,442
```

- `gen` is FNV-1a 32 of the sorted force-node and edge sets; `pos` values are integer world units (1 = 1 px at zoom
  1); `pin.<pid>=1` reserved for v2. Unknown lines carried (255 B each, 16 KiB total) under a 256 KiB cap. Unreadable:
  ignored and not overwritten that session. A gen mismatch updates incrementally or recomputes. Older versions never
  read it; raw ZIP exports leave it out (a cache); nav.ini and links.ini stay in the ZIPs.

### 13.4 settings.ini keys (undotted numeric keys, appended after `language`)

New keys go after `language` (store.rs:2902, "Last of the known keys"), before the carried unknown lines, in the order
they ship; every existing line keeps its bytes and order. Missing means the default; older builds carry them as
unknown; 1.0.0-beta drops them and the default returns. Each key: a `Settings` field, `settings_default`
(store.rs:2451), a `keep_bool`/number arm in `settings_cb`, a `put_int` in `settings_save` (2848), a `settings.KEY`
probe (app.rs numeric table), a FORMAT.md row, and `test_probe.rs` accepts it (undotted).

| Key | Values | Default | Phase |
|---|---|---|---|
| `gestures` | 0/1 | 1 | 1 |
| `landscape_fit` | 0/1 | 1 | 1 |
| `hints` | bitmask: 0 first-open tip, 1 first link, 2 eye tip, 3-4 bar header hint count, 5 first snap, 6 pan hint, 7 finger-shape reject tip, 8 sideways hint. Bits 0 and 8 (tips about changed behaviour) show only to an upgrading owner: a settings.ini that existed with `settings_version=3` and no `gestures` key (memory-only `Settings.upgraded`; `SETTINGS_VERSION` is not bumped) | 0 | 1 (bits 0, 2-4, 6, 8), 2 (1), 3 (7), 4 (5) |
| `link_areas` | 0/1 | 0 | 1 |
| `rotation_lock` | 0 = follow `follow_rotation`, 1..4 = locked at rotation 0..3 | 0 | 3 |
| `shapes` | 0/1 (2, 3 reserved) | 1 | 4 |
| `shape_hold_ms` | 300..1500 | 550 | 4 |

### 13.5 Memory only

History kinds `Link` (n >= 1 records) and `Replace` (shape-last); the back list (32 page ids, push rules in 7.7,
cleared at close); Recent targets (4); the box cache (64); the canvas cache (2 plus `page_canvas`); the shape region
copy; the map
canvas; the finger capture and trail.

### 13.6 Name search

store.rs's private `fold_next` (store.rs:1444) becomes `pub(crate)`, unchanged, and i18n.rs builds `i18n_fold_next`
and `i18n_name_match` on it (prefix first, then word start, then substring; accent-folded, case-insensitive);
`cmp_name` and its callers are unchanged. No new file.

### 13.7 Docs (each in the phase that brings the behaviour)

FORMAT.md (links.ini v2, gestures.ini, graph.ini, the settings keys, the moving-between-versions rows: area links are
inert before beta.5 and partly read in f9d13f5; older versions ignore gestures.ini and graph.ini; .qpg unchanged);
SHORTCUTS.md (registry, sources, gestures, "not only with Enable shortcuts", the unassignable list); DESIGN.md (new
screens, the refresh table, DU corrected from about 120 to 188 ms); SIM.md (commands, probes, `# rust-only`);
TESTING.md and KINDLE.md (owner scripts and device checks); po/es-glossary.md (enlace, zona, mapa, gesto, figura,
forma).

---------------------------------------------------------------------------------------------------------------

## 14. E-ink and performance

### 14.1 Measured waveforms

DU 188 ms (294 ms cold), GL16 271 ms, GC16 341 ms (the whole panel; a Full refresh always covers the whole panel).
CPU figures are host times x20. `RefreshMode::Fast` = DU, `Ui` = GL16, `Full` = GC16 (display.rs:16-24).

### 14.2 Refresh plan per surface

| Surface | Refresh |
|---|---|
| Two-finger gesture | nothing while moving; the action's own refresh is the feedback; recognition under 20 us per frame, firing on lift adds 13-30 ms |
| Pan / zoom | Fast at most every 180 ms of t_ms (today's), Ui at PinchEnd if it moved |
| Panel press cancelled by a second finger | Fast restore (was Ui) |
| Bar open / close | the 1.37 Mpx card composed (50-90 ms) then GL16; about 0.35 s, no flash |
| Chip press | immediate DU invert (`app_press_feedback`) |
| Peek | one GL16 present of the page area and the card together; cache hit about 0.35-0.45 s; miss adds a pumped load and render (0.1-0.3 s, up to 1.2 s) |
| Commit | one GC16 (resets the 30-count) |
| Cancel / Back | GL16, or GC16 after 3 or more peeks; Back chip GL16 |
| Eye toggle | GL16 of the button plus the union of the boxes, flushed separately when far apart |
| Find, Rename, New page..., map Rename | the keyboard host: field then key as separate Fast flushes (Find: field, rows, key, through the split host, 8.1), one GL16 900 ms after typing pauses |
| Link long press | four border strips by DU with their own flush, then the card GL16, never one union; close: GL16 of card and strips |
| Link mode | strip GL16; lasso Fast per batch (own flush); on lift GL16 of the lasso's box union |
| Finger-shape trail | G11: DU frames at most 25 Hz with their own flush; match GL16 of the trail; reject thinning DU then GL16 at 250 ms; pen abort Fast plus the writing settle |
| Record card | GL16 of its reason line only |
| Smart shapes | preview DU, cancel DU, commit through the normal settle; never GL16 or a toast while the pen is down |
| Index map | open and close GC16; selection GL16 of small rects; pan and zoom DU at most every 180 ms, then GL16 at release; a finished bgjob layout one GL16 when idle 500 ms |
| Page-turn end toast | toast only |
| `landscape_fit` toggle sideways | `view_reset` + Full |

Screens only call `app_invalidate`; `app_flush` presents once per loop pass and keeps the strongest mode over one
union (app.rs:1329-1400), so far-apart rects or a DU that must not merge into a GL16 get their own `app_flush`.

### 14.3 Flash budget

- The every-30th-Ui promotion to Full (app.rs `app_flush`) stays: writing's GL16 settles count toward it, and an
  area-weighted rule would change ghost cleanup under writing.
- GC16 only for deliberate screen changes: commit, go to a page, the Pages view, map open and close, theme, invert,
  rotation, the sideways toggle.
- Each commit's GC16 resets the counter, so heavy bar use costs at most one surprise flash per roughly ten sessions
  without a commit. Revisit only after device testing.

### 14.4 Never block the pen (its evdev buffer overflows after about 100 ms)

- No `MXCFB_WAIT_FOR_UPDATE_SUBMISSION`, no second wait on a marker, no new display calls. The only wait stays
  pen_leave before a non-DU refresh.
- Pumped or split work:
  - page renders every 64 rows (as today);
  - page reads: a pumped variant of the editor's own reader, `store_load_page_pumped(nb, index, w, h, out)`, is
    `store_load_page` (store.rs:1729, with its side effects: a damaged file set aside, side-restored paper written
    back, so a peek shows a page exactly as opening it would) with `work_pump()` every 256 strokes inside the
    deserializer (an internal `page_deserialize_with(buf, out, pump)` behind `page_deserialize`, store.rs:2246);
    `store_load_page` stays the unpumped wrapper, its callers and bytes unchanged, and the exporter's
    `store_read_page` is untouched. Used by peek, prefetch and idle migration (phase 1, migration phase 2). A commit
    moves the peeked `Box<Page>` into the session cache instead of reading the page again (7.5);
  - shape dabs every 8 points (phase 4); the finger-shape match and the slip bank through the pump hooks
    fingershape takes (phase 3);
  - map edges every 64, layout every 2 steps, big layouts in a bgjob child (phase 5);
  - prefetch one 160-row band per idle tick; idle migration one page per idle tick.
- `main.longest_unpumped_ms` (new runner probe, phase 1): the longest gap between two `work_pump` calls or input
  reads while the app works, not counting the time inside `app_flush`'s present or a settings or file save (the
  gap restarts after each), since those are not work that should pump. The headless runner is a debug-profile build
  on a shared host, so the probe prints `ok` under a host bound of 250 ms (it catches a missing pump, such as a whole
  page read unpumped) and the gap otherwise; the scenarios (big notebook, prefetch, migration, map, 32 shapes)
  expect `ok`. The Kindle's ~100 ms rule is checked by unit tests that count pumps per 256 strokes and per 64 rows.
- `input_pump` drains touch into touchgest, and frames carry kernel SYN times, so a backlog after a render neither
  bunches a swipe, nor fakes a hold, nor overflows the touch buffer. The busy flag counts every down contact, so the
  device key scan's 330-370 ms block never starts mid-gesture.
- `light_get` never runs on the input path; `light_cached` replaces it there (phase 3).

### 14.5 Primitives (`canvas.rs`)

- Phase 1: `canvas_dash_rect(c, r, thick, on, off, halo)` (the eye; phase 2 the long-press border and the dotted
  rectangle).
- Phase 2: `canvas_dash_poly(c, pts, thick, on, off, halo)` (the lasso); phase 3 adds its dot mode (the trail; or
  `fs_draw_trail` if fingershape.rs provides it).
- Phase 4: `canvas_slash` (the Shapes button).
- Phase 5: `canvas_line_aa` (canvas.rs:245) becomes row-span, bit-identical over 300 random edges and widths, about
  40x faster on long edges; `canvas_disc_pattern(c, cx, cy, r, level)` (4x4 Bayer on 2x2 cells).
- Everything new is 1-bit-safe. Golden shots (Gothic frieze included) must not move.

### 14.6 Memory

Canvas cache at most 3 x 4.6 MB; map canvas 4.6 MB only while open; shape region copy at most 4.6 MB only during a
hold; graph arrays about 2000 x 32 B; finger capture 256 points. Every allocation failure (`try_reserve_exact`) has a
slower path that draws the same pixels, or a stated refusal.

### 14.7 Live view

The screen stream (`live_push` in `app_flush`) carries `App.screen` before the toast and chip, so viewers see the bar,
the card, the eye, the trail and the map, as they see the panel. `/live/pen` (feature A) carries pen samples only;
non-ink pen contacts raise the capture-blocked flag (section 3.1).

---------------------------------------------------------------------------------------------------------------

# Part P. Phases, mapping, rules, risks, questions

## 15. The six phases

Each phase is one Kindle build (`make rust-kindle`) that the owner installs and tests. Each ships alone.

### 15.1 Rules for every phase

- Branch per phase from `claude/rust-port` after the previous phase merged (never push to `main`, `claude/rust-port`
  or `claude/beta5-features`); small logical commits with CLOUD.md's attribution lines; the owner's local session
  reviews and merges.
- The gate is `make rust-check`. On the Linux cloud machine, Apple-libm float fixtures and outline-font CRCs may
  differ: say which and why, never edit an expectation to pass on Linux; the Mac gate is authoritative.
- Every UI string of the phase in English and Spanish in the same change: po/es.po, `make l10n`, then `msgfmt --check
  --no-hash --endianness=little -o po/es.mo po/es.po`; TRANSLATORS width notes; every new surface visited in
  `y_all_screens` upright and sideways in both styles under CHECK_FIT; `make l10n-check`.
- Scenarios that use a new command or feature carry a new header `# rust-only`. c-check passes an explicit flag,
  `$(MAKE) BUILD=$(CBUILD) SCEN_SKIP_RUST_ONLY=1 ... scenarios` (and the same for l10n-check), and the scenarios rule
  skips a `# rust-only` file when `$(SCEN_SKIP_RUST_ONLY)` is set. Comparing `$(BUILD)` with `$(CBUILD)` would never
  match: `CBUILD ?= $(BUILD)/c` re-expands to build/c/c inside the sub-make (Makefile:19-21, 581). The selftest's
  new lines go in separate files, `tests/selftest/pass-rust.qs` and `fail-rust.qs`, run only without the flag; and
  under the flag l10n-check skips `y_all_screens.qs` (rust-only from phase 1: new screens, moved Buttons and Display
  coordinates) and still runs the msgfmt checks and tests/l10n/templates.qs. Phase 1 adds the handling and documents
  it in SIM.md. Plainly: c-check then stops guarding every existing scenario a phase converts (section 20, question
  4).
- Raw panel and Settings coordinates in scenarios become symbolic (`tap panel NAME`) before the panel or the Settings
  row changes.
- New probe keys go in SIM.md's Checks table, one use per family in tests/selftest/pass-rust.qs and a failing one
  in fail-rust.qs (the rust-only selftest files that c-check skips; pass.qs and fail.qs stay as they are). New `GRIMOIRE_*` hooks join the scenarios rule's unset list and `SIM_HOOKS` in `test_scenarios.rs`.
- Existing observables of untouched flows stay byte-identical: settings.ini lines (new keys only appended), qlog
  lines, `display.presents` of existing scenarios except the ones a phase changes on purpose (listed per phase).
- The hard rules (section 17) hold in every phase.

### 15.2 Phase 1: gestures and the Links bar

**Goal.** The touch pipeline extracted into a pure recognizer fed by the Kindle and the runner alike, with its input
fixes; the two-finger built-ins (D6) with the mirror rule and per-axis panning (D5); the sideways whole-page setting;
contact-level runner input; the touch log and opt-in trace. The Links bar: list, peek and commit, back list, the
Find sheet, the eye, the Links panel button.

**Modules touched.** `input.rs`, `input_evdev.rs`, `platform_none.rs`, `sim_sdl.rs`, `mainloop.rs`, `power.rs`,
`app.rs`, `editor.rs`, `pagesview.rs` (Go to page... opens Find), `shortcuts.rs` (`ActionSrc`, five names),
`library.rs` (Buttons tables: Index group, Pages row), `dispmenu.rs` (the sideways row), `panel.rs` (12 buttons),
`canvas.rs` (`canvas_dash_rect`), `store.rs` (keys, pumped `store_load_page`, `fold_next` made `pub(crate)`),
`i18n.rs`, `ui.rs` (glyphs), `lib.rs`; Makefile (`# rust-only`); docs SIM.md, FORMAT.md, SHORTCUTS.md, DESIGN.md,
KINDLE.md, TESTING.md.

**New modules.** `touchgest.rs` (pure), `gesturemap.rs` (pure, read-only overlay), `linkbar.rs`, `findsheet.rs`.
app.rs gains the split keyboard host (8.1); the keyboard dock waits for phase 2.

**Data and settings.** settings.ini `gestures` (1), `landscape_fit` (1), `hints` (bits 0, 2, 3-4, 6, 8), `link_areas`
(0), appended after `language` in that order. gestures.ini read (touch lines live; shape and pen lines parsed and
carried). `grimoire/touch-trace` switch file and `grimoire/touch-trace.txt` (4 MB cap, never exported). Memory: back
list, canvas cache, box cache.

**Runner commands and probes.** Commands: `finger ID down|move|up X Y [MAJOR] [TOOL]`, `frame [MS]`, `swipe N X Y DX DY
[STEPS] [SEP] [LANDGAP]`, `tapn N X Y [SEP] [MS]`, `tap panel NAME` / `pentap panel NAME`; `pen hover` / `pen leave`
also drive touchgest's pen context (the runner's `pentap` ends with a leave); a monotonic frame clock. No window
emulation (no SDL window simulator in the crate). Probes: `touch.contacts`,
`touch.class`, `touch.suppressed`, `touch.gesture`, `touch.reject`, `gestures.ACTION`, `settings.gestures`,
`settings.landscape_fit`, `settings.hints`, `settings.link_areas`, `editor.scrolls`, `editor.areas`, `editor.bar`,
`editor.bar.entries`, `editor.bar.N`, `editor.bar.peek`, `editor.bar.rect`, `editor.back`, `editor.canvas_cache`,
`editor.find`, `editor.find.rows`, `editor.find.N`, `editor.panel_at.X.Y`, `editor.modal` (new values `links`,
`find`), `notebook.last_page`, `main.longest_unpumped_ms`, plus feature B's clock probe in corner scenarios.

**Scenarios and unit tests.**
- Unit: `rust/tests/test_touchgest.rs` (G15.4), `rust/tests/test_gesturemap.rs`, `editor.rs` view tests
  (`landscape_fit` on and off; `zoom_max` and `zoomed()` bit-identical when off), `test_probe.rs` (new keys
  undotted), `test_shortcuts.rs` (counts), `test_panel.rs` (12 buttons, 0-4 pins), `test_i18n.rs` (fits), a canvas
  test for `canvas_dash_rect` (dash lengths, halo, corners), a store test that the pumped reader returns the same
  page as `store_load_page` and pumps every 256 strokes, `rust/tests/test_linkbar.rs` (grouping, reading order, the
  read-only and unreadable links.ini states, which no fixture can make portably, and the layout heights), sim_sdl.rs
  tests for each new command's batching and for an `expect` between two `frame` lines with contacts down (the frame
  clock never runs backwards), a test that the touch-log formatter writes no coordinates while not grabbed.
- Scenarios (all `# rust-only`; PHASE1-BRIEF.md 20.1 has their steps, and this list matches it): `t_touch_1_contacts`,
  `g_gest_1`, `g_gest_1b_unbound`, `g_gest_2_zoomed` .. `g_gest_8_corner` (with `g_gest_3b_sideways_width`), `s_fit_1`
  (G15.5); the root `z_bar_0_make` (no notebook fixture with links exists, so it builds one: 5 pages, page 3 named
  'Árbol', page 5 'Arena', page 1 with three links, two to page 3 and one to page 2; with a new key fixture
  `tests/fixtures/index-keys`); `z_link_areas_1` (the action sets `settings.link_areas` 1 and `editor.areas` 3; a shot
  shows the tags; the refreshed rects lie within the boxes' union; `editor.live_matches_render` stays 1 after a
  stroke across an area; the exported PDF has no overlay; persists across reopen; sideways and zoomed shots);
  `z_bar_1_list` (swipe up gives `editor.bar open`, `entries 2`, `editor.bar.1 p3 x2`, `editor.bar.2 p2`; shots
  upright and sideways under CHECK_FIT; X, 2-down, close-menu and the links key close it) and `z_bar_1b_list_es`;
  `z_bar_2_peek` (tap entry 1: `editor.bar.peek 3`, `editor.page 1`, `notebook.last_page 1`, one `ui` present of the
  page area and the card, no `full`; entry 2 peeks page 2; a finger tap above the bar on a link: bar 0, page 2, one
  `full`, the link not followed; `editor.back 1 1`; go-back returns; a resting finger on a link when the pen arrives
  does not follow; the legacy hold still opens the picker); `z_bar_3_twofinger` (during a peek a 2-down swipe returns
  to page 1 and the Pages view does not open); `z_bar_4_pen` (a pen line outside commits, `editor.strokes`
  unchanged); `z_bar_5_cancel` (X after 3 peeks returns with Full; one peek then X a `ui`; the back chip keeps the
  bar); `z_bar_6_leave` and `z_bar_6b_reopen` (a sleep or power hold during a peek; reopening lands on page 1);
  `z_bar_7_prefetch` (after `wait 1500`, `canvas_cache 1 0 2`; `main.longest_unpumped_ms is ok`; go-back after a
  commit returns); `z_bar_8_pager` (13 targets: the pager, entry 13 on screen 3); `z_bar_9_corner` (a corner tap
  over a link, with the bar up and while peeking toggles the clock and neither follows, commits nor closes; a Links
  button tap while peeking cancels to the origin); `z_find_1` (type 3, the row: `editor.page 3`; go-back returns;
  from the Pages view's Go to page... one `full`; type 9: 'There is no page 9 (5 pages).'), `z_find_2_names` ('arb'
  lists 'Árbol'; 'ar' lists Árbol then Arena; 'arx' lists 'No page names match.'), `z_find_3_es` (upright and
  sideways under CHECK_FIT; `keyboard.draws` within its bound); selftest lines (in `pass-rust.qs`) that draw a stroke
  and then `tap panel undo` hits P_UNDO upright and sideways with 0-4 pins; `y_all_screens` (Display sheet row, the
  bar, Find; new shots after 65).
- Changed on purpose (each in its own commit, suite green before and after, each converted file `# rust-only`):
  raw panel taps become symbolic: `grep -lE '^(tap|pentap) 17[0-9][0-9] ' tests/scenarios/*.qs` lists 84 files
  that tap in the panel's column, about 65 of them below Next (the ones `P_LINKS` moves); each conversion is checked
  by `editor.panel_at.X.Y` naming the hit under the old coordinate. The Buttons-sheet taps (the Index group moves the
  centred list: v_controller.qs, v_controller_forget.qs, v_controller_switch.qs, v_controller_triggers.qs,
  y_all_screens.qs). The Display-sheet taps (the new row makes the card taller and `card_rect` re-centres it at (h -
  card) / 3, dispmenu.rs:81-87, so every row and Done move, in both hosts and orientations: at least i_display,
  m_sleep, v_controller_actions, y_all_screens, y_koreader_hosted, y_koreader_not_hosted, y_pagesize_1_ksc,
  z_tool_5_sheets_turn and z_tool_b_button_over_sheets, found by grepping for the Display sheet, plus SIM.md's
  documented Display coordinates), checked with the Display probes. Rotate scenarios that need the width fit get
  `# settings: landscape_fit=0` (at least f_batch2, y_all_screens, y_pagesize_1_ksc, y_pagesize_2_back,
  z_4_erase_zoom_rotate; this alone keeps them C-compatible). `j_pages_keyboard` (Go to page... now opens Find).
- Stay green unchanged: every pinch scenario (f_batch2, k_pinch_gate, p_power, v_controller_actions, y_all_screens,
  z_4_erase_zoom_rotate, y_pagesize_2_back, z_a_precise_other_size), z_index_1, z_index_2_links (inked links still),
  v_shortcuts_links, y_live_ink, h_live_vs_settled.

**E-ink.** Gestures draw nothing while moving; pan/zoom Fast every 180 ms then Ui; the cancelled panel press restores
Fast; bar open/close GL16; chip DU; peek one GL16 of the page area and the card; commit GC16; cancel GL16 or GC16
after 3 peeks; eye GL16 split; Find through the split keyboard host (field, rows, key); page-turn end toast only; the
sideways toggle Full. No new display call, no wait.

**Owner acceptance on the Kindle.**
1. Upright: two fingers down open Pages; two up close them. Two fingers left and right turn pages; at the last page a
   toast and no new page; at the first page a toast.
2. Two fingers up (or the new Links button after Next) open the Links bar on a page with your beta.5 links: one row
   per entry, ×2 where two links go to the same page. Two fingers down, X or the Links button again close it.
3. Tap entries to hop: no flash, the bar keeps the starting page's links. Tap the page to stay (one flash), or press X
   or '‹ Page 3' to go back. A finger or the pen on a link under that tap does nothing else.
4. Areas shows dashed numbered boxes where each lasso is; it is remembered after a restart.
5. Go to finds pages by number or by name, accents ignored; Back (a key bound to go-back) returns.
6. Sideways: the whole page shows (on by default, said in these notes), and the two-finger swipes work anywhere.
   Display > 'Sideways, show the whole page' off: vertical drags pan, left and right still turn pages. Zoomed in:
   two fingers pan; the panel's page chip and Links button open the menus; the pan hint shows once.
7. The clock corner: a finger tap in the top-left corner toggles the clock with the bar up, while peeking, and over a
   link (the link is not followed); a two-finger swipe that starts there fires and does not toggle it.
8. Pinch zoom feels as before (no zoom drift while panning). A finger resting on a link no longer follows it when the
   pen comes near. After a long page render, taps still register.
9. Live view: committing a peek with the pen shows no ink in the browser.
10. Report: swipe right after writing, and how often 'Move the pen away, then swipe.' appears; rest your palm and
    thumb while writing (nothing should fire).
11. Create the empty file `grimoire/touch-trace`, run the touch script in docs/KINDLE.md (about 15 minutes in all:
    the 3-minute two-finger part, 20 two-finger swipes each way, 10 two-finger taps, 5 three-finger taps, writing
    with the palm resting, 10 swipes straight after lifting the pen, then the one-finger set for phase 3: Z, N, S,
    circle, alpha, triangle and V shapes, 10 one-finger swipes each way, 5 minutes of writing, 2 minutes holding and
    reading), and send `grimoire.log` and `touch-trace.txt`.
- Known in this build: New link uses today's inked flow; holding a link on the page still opens the Pages picker;
  holding a bar entry does nothing; three-finger gestures are off.

**Dependencies.** Features A and B merged on `claude/rust-port`; their interfaces verified (section 3). No pure
module from `claude/rust-index-pure`.

### 15.3 Phase 2: ink-free links and the long-press card

**Goal.** R1 and R2: links.ini area lines, area-first hit test with smallest wins, migration of beta.5 links, link
edits in undo, ink-free link making with Link to..., the long-press card on the page and in the bar, Copy page keeps
links.

**Modules touched.** `nav.rs` (`NavLink.area`, area lines, `NAV_LINKS_FILE_MAX`, area-first `nav_link_areas` /
`nav_link_at`, `nav_link_migrate`), `history.rs` (`ActKind::Link`, `history_undo_kind`, drop-last helpers),
`editor.rs` (migration in `page_at` and idle ticks, `links_dirty` flushes, `Owner::Lasso`, link-mode strip, card
hosting, `undo_redo` link steps, Copy page links, removal of `link_box_drawn` and the box undo), `linkbar.rs` (Link
to... pick mode, row hold), `findsheet.rs` (PICK mode), `keyboard.rs` (the `dock_top` field that `modal_rect`
honours, starting at y = z + 8 at the top, 8.1), `pagesview.rs` (hold path, PG_UNLINK, PG_TO_INDEX retired; Choose
from the pages... kept for New link, hidden from the card), `canvas.rs` (`canvas_dash_poly`), `store.rs` (`hints`
bit 1); docs FORMAT.md.

**New modules.** `linkcard.rs`.

**Data and settings.** links.ini v2 (section 13.1). `hints` bit 1. Memory: link history records, Recent targets.

**Runner commands and probes.** No new commands. Probes: `editor.link.N.area` (`x y w h` or empty),
`editor.link_card` (`0`, `1`, rows), `editor.link_card.rect`, `editor.linkto` (`0`, `pick`), `editor.linkto.rect`,
`editor.lasso` (points). `editor.links` keeps its existing value format.

**Scenarios and unit tests.**
- Unit: `test_nav.rs` (round trip with and without areas; an f9d13f5-format file reads unchanged; malformed area
  carried; orphan area dropped; a 4000-link file fits the cap; smallest wins when nested; migration takes the
  unpadded box and ignores erase strokes; a lasso hidden by the precise eraser migrates; a dormant link stays);
  `rust/tests/test_history.rs` (new: Add, Erase, Clear unchanged; Link undo and redo order interleaved with ink; a
  group of N; eviction; the failed-save drop).
- Scenarios: `z_link_migrate` (a fixture copied from a real beta.5 device notebook: open it, `editor.link.1.area`
  equals the lasso's box, stroke count unchanged, links.ini has the area line; erase the lasso whole and tap: it still
  follows; reopen: persists; a never-visited page migrates after idle ticks with `main.longest_unpumped_ms is ok`;
  Copy page: the copy has the link; the PDF /Link rect equals the area); `z_link_undo` (delete, undo, redo, with
  `editor.links` and links.ini after each step); `z_index_2_links` rewritten (add-link then a pen ring:
  `editor.strokes` unchanged, `editor.linkto pick`; tap an entry: the link exists, area within 2 px of the ring's
  box; a finger tap inside follows; Undo removes it, Redo restores it; a dot-sized ring gives the too-small toast; a
  ring low on the page docks the sheet at the top below the clock corner; New page with 'Ideas': npages +1, last page
  named, the link goes to it, `editor.page` unchanged; Cancel: 'No link made.' and no strokes; a lasso drawn zoomed
  maps to the right page rect; shots upright and sideways, ES fit); `z_link_card` (touch down on a link, `wait 700`,
  up: `editor.link_card 1`, the Pages view not open, a shot shows border and card with the page visible; Delete: links
  0, Undo: 1; Rename to 'Cap'; naming an unnamed target adds an Index row and toasts; Redraw changes the area, Undo
  restores it; Change where it goes shows Link to... without 'Choose from the pages...' and `editor.modal` never
  reads `pages` while the page stays shown; a tap outside closes without following; a corner tap toggles the clock
  and leaves the card; placement shots near each edge, upright and sideways); `z_bar_10_card` (a ×2 entry's Delete
  removes both, one Undo restores both; no Redraw row); `y_all_screens` (card, strip, Link to...).
- Changed on purpose: `z_index_2_links`, the Pages-view hold scenarios, `v_shortcuts_links` where it used the inked
  flow.

**E-ink.** Strip GL16; lasso Fast per batch with its own flush; dotted rectangle GL16 at the lift; card: strips DU
(own flush) then card GL16; close GL16; link undo composes the affected boxes (GL16 only if the eye is on).

**Owner acceptance on the Kindle.**
1. Open your beta.5 notebooks: every link still works. Areas shows each link's area where its lasso was.
2. Erase an old lasso completely: the link still follows.
3. Make a new link: the dashed line follows the pen and no ink is left behind. Link to... lists Recent targets, sits
   at the top when you circled low on the page (below the clock), and New page... names and links a new last page.
   Undo removes a new link.
4. Hold a link 0.6 s: the dotted border appears, then the card beside it, with the page in view. Try Rename, Add page
   N to the Index, Change where it goes, Redraw the area and Delete; Undo brings a deleted link back.
5. Hold an entry in the bar: the same card; a ×2 entry deletes or retargets both in one Undo step.
6. Copy page keeps the links. The exported PDF's links land on the areas.
- Known: a visible border is freehand ink until phase 4 brings Smart shapes.

**Dependencies.** Phase 1. No pure module.

### 15.4 Phase 3: Settings > Gestures with recording, every toggle assignable

**Goal.** D4, D8 and "everything with a toggle or important setting can be assigned": the action registry and the
remaining actions; Settings > Gestures with record mode; gestures.ini writes; recorded one-finger shapes (capture,
trail, match, reject) through `fingershape.rs`; thresholds frozen from the phase 1 traces; the hover rule, three
fingers and one-finger swipes.

**Modules touched.** `touchgest.rs` (record mode, frozen T_palm and windows, BROAD live, hover rule, three fingers),
`input_evdev.rs` (`hover_learn` on contact frames whether or not Live view's hover hook is set, `TgPen.hover` from
`raw_hover`; the trace's `E` lines for shapes), `gesturemap.rs` (writes, shape
lines, `=nothing`, carry 1 KiB / 16 KiB), `shortcuts.rs` (`ShortcutBinding` source, `SHORTCUT_MAX` 192, computed
`write_pads`), `library.rs` (registry reads, Settings bottom row, `Modal::Gestures = 17`, BindKind, record card,
Gesture events in record mode before the `ph < 0` return at library.rs:2405), `editor.rs` (finger capture in
`Owner::Link`, trail, match dispatch with `ActionSrc::Shape`, the remaining actions' dispatch), `app.rs`
(`app_request_sleep`, rotation lock), `light.rs` (`light_cached`), `store.rs` (`rotation_lock`, `hints` bit 7),
`sim_sdl.rs` (`shape`, `fpath`, `trace`), `canvas.rs` (dot mode), `ui.rs` (Gestures glyph).

**New modules.** `actions.rs` (registry). Called: `fingershape.rs` (G14).

**Data and settings.** gestures.ini written (atomic; map changes only after success; Reset deletes). settings.ini
`rotation_lock`, `hints` bit 7. `shape.<action>` lines (format 1, up to 32).

**Runner commands and probes.** Commands `shape NAME X Y SIZE_MM [MS] [ROT] [SEED] [REV]`, `fpath X1 Y1 ... [MS]`,
`trace FILE`. Probes `fshape.last`, `fshape.fired`, `fshape.state`, `fshape.trail`, `gestures.shape.ACTION`,
`gestures.shape_count`, `library.gestures_group`, `library.gestures.learning`, `settings.rotation_lock`.

**Scenarios and unit tests.**
- Unit: `test_actions.rs` (EN and ES labels, group, kind and sources for every action; groups at most 11 rows; pairs
  symmetric; `hold-eraser` not TOUCH or SHAPE; `shortcuts` and `bluetooth-controller` not KEY or PAD; every
  SHAPE-capable action has labels);
  `test_shortcuts.rs` (counts, learn-everything within 192); `test_touchgest.rs` (hover rule both sides of 60 mm;
  unlearned mapping falls back; T_palm guard; record mode shares constants; record-mode one-finger pen rejection);
  `test_gesturemap.rs` (shape lines with overlays, `=nothing` leaves shapes, carry limits, unreadable never written,
  Reset deletes); `test_i18n.rs` (the three bottom words fit 248 px at 34 px caps). fingershape's own tests belong to
  the pure branch.
- Scenarios: `v_actions_screen` (every new action by key, through `settings.*` and display probes; a light step with
  the level unknown toasts and never calls the light read; with rotation-lock on, `rotate 1` keeps the orientation;
  scroll-down past the end lands on the next page at the same zoom; a theme switch relayouts the panel);
  `v_gestures_1` (Pages row, swipe 2 down: the card shows '2 fingers down'; Use: `gestures.pages` is `touch.2.down`
  and gestures.ini absent; learn `touch.2.tap` for Undo: the line appears; learn it for Redo: the toast names Undo);
  `v_gestures_2_reject` (LANDGAP 300 gives 'together'; a diagonal gives 'straight'); `v_gestures_3_forget_reset`;
  `v_gestures_es` (upright and sideways, both styles, CHECK_FIT); `g_gest_10_hover`, `g_gest_11_three` (rows shown
  with sim slots 10, hidden with 2), `g_gest_12_onefinger` (a learned `touch.1.left` fires only from the page, never
  the panel); `s_shape_1_record` .. `s_shape_9_traces` (G15.5; `s_shape_4_guards` includes a shape that starts in the
  corner zone and still fires, and a corner tap that only toggles the clock; `s_shape_6_scroll` a pan plus a shape
  bound to `pages`; `s_shape_7_bar` the bar's own shape closes it to the origin); `v_gestures_shape_clear_reset`; a
  32-shape match with `main.longest_unpumped_ms is ok`; `y_all_screens` (the Gestures sheet, its seven groups, the
  record card, the Buttons sheet's seven groups); `v_actions_system` (language cycles and relayouts; Enable
  shortcuts and the Bluetooth controller toggle by a gesture; turning gestures off by a gesture toasts how to turn
  them back on).
- Changed on purpose: the 10 raw Settings bottom-row taps (v_controller*.qs, u_update_*.qs, y_all_screens.qs:19,
  102, 631) move to the new centres.

**E-ink.** Record card reason line GL16 only; trail DU frames at most 25 Hz with their own flush; match GL16 of the
trail with the action's refresh; reject thinning DU then GL16 at 250 ms; never GC16 for a shape; the remaining
actions per their kinds (invert and theme Full).

**Owner acceptance on the Kindle.**
1. The Settings bottom row reads KOReader | Controller | Gestures | Updates | Done, in English and Spanish, upright
   and sideways.
2. In Gestures, record a two-finger tap for Undo and try it. Record a gesture that is taken: the card names the action
   that loses it. Make bad tries on purpose (fingers apart, diagonal, pen near): each is refused with its reason.
3. Record a finger shape (a Z, an N or a loop) for Pages, three times; then draw it anywhere on the page, upright,
   sideways and zoomed in: the Pages view opens. A V or a check mark is refused as too easy to make by accident.
4. Write and rest your palm for a few minutes: no finger shape fires. A tap in the clock corner still toggles the
   clock; a shape you start there still fires.
5. Hold a row to clear it. Reset restores the built-ins and asks first when shapes exist.
6. Controller > Buttons shows the seven groups. Bind light, invert, rotation lock, sleep screen, theme, language,
   sleep after and 'Whole page sideways' to keys, and to gestures or shapes; Enable shortcuts and the Bluetooth
   controller to a gesture or shape.
7. Swiping with the pen hovering away from your fingers now works. Three-finger rows appear only if your panel
   reported three contacts.

**Dependencies.** Phase 1's traces read and the constants frozen (T_palm, landing windows); the freeze gate passes
(the writing, holding and reading traces fire 0 finger shapes against an 8-shape stress set; the owner's shape traces
accept at least 95% with 0 wrong). `fingershape.rs` merged from `claude/rust-index-pure` with the G14 surface (any
missing piece added there by the pure branch).

### 15.5 Phase 4: smart shapes

**Goal.** The brief's "smart tool system": hold-to-snap preview, one Undo, the toolbar toggle on by default, the
picker row, shape-last.

**Modules touched.** `editor.rs` (dwell anchor in `draw_sample`, hold check in `editor_tick`, preview, cancel,
commit, `P_SHAPES`, picker row, actions), `history.rs` (`ActKind::Replace`), `panel.rs` (13 buttons), `canvas.rs`
(`canvas_slash`), `store.rs` (`shapes`, `shape_hold_ms`, `hints` bit 5), `actions.rs` (three actions), `ui.rs`
(glyphs F0831, F0832). Called: `shape.rs`.

**New modules.** None of ours.

**Data and settings.** `shapes` (1), `shape_hold_ms` (550), `hints` bit 5; `grimoire/shape-log` switch file.

**Runner commands and probes.** No new commands (`pen line`, `pen curve`, `wait` suffice). Probes
`editor.stroke_shape`, `editor.shape_preview`, `settings.shapes`, `settings.shape_hold_ms`, and a new
`editor.panel_style.NAME` (`plain`, `selected`, `disabled`; `editor.panel_button` is a height today), so a scenario
asserts the Shapes button never reads `selected`.

**Scenarios and unit tests.**
- Unit: `test_history.rs` (Replace); `test_panel.rs` (13 buttons, scale >= 0.93 sideways at 4 pins); corpus and
  recognizer tests belong to the pure branch (`test_shape`), with the owner's strokes (assets/eraser-sample.qpg and
  the two evidence pages) as the negative corpus.
- Scenarios: `z_shape_1_circle` (pen circle, `wait 700`, up: `editor.stroke_shape ellipse`, strokes +1; one Undo gives
  0); `z_shape_2_move_cancels`; `z_shape_3_writing_never` (eraser-sample strokes replayed with waits never set
  `editor.shape_preview`); `z_shape_4_toggle` (panel, key and picker; persists; shots in both orientations; the button
  never inverted); `z_shape_5_zoom` (gates in screen mm at 2x); `z_shape_6_link_mode` (the lasso never snaps);
  `z_shape_7_last` (shapes off, a box, shape-last replaces, Undo gives the raw stroke back); `editor.live_matches_
  render` after each; `y_all_screens` (the picker row, the panel at 13).

**E-ink.** Preview DU, cancel DU, commit through the normal settle; never GL16 or a toast while the pen is down;
shape-last GL16 of the union; toggle GL16 of the button.

**Owner acceptance on the Kindle.**
1. Draw boxes, circles, triangles and lines, and hold the pen still: a clean shape appears, and lifting keeps it.
   Hold, then keep moving: your line returns and the stroke continues. One Undo removes a shape.
2. The panel's Shapes button (filled when on, slashed outline when off, never black) toggles it; the picker row sets
   the hold time.
3. Write a page of blackletter and a page of normal notes with pauses: nothing should snap. If anything does, create
   the empty file `grimoire/shape-log`, write for an evening, and send the log.
4. With shapes off, 'Shape the last stroke' (bound to a key or gesture) fixes a box, and Undo gives your line back.
5. Draw a box around a link's words: it is ordinary ink and the link still works (R1's visible border).
6. Live view: the browser shows the snapped shape once you lift.
- Arrows, polylines and arcs ship in this build only if they pass the corpus gate.

**Dependencies.** `shape.rs` merged from `claude/rust-index-pure`. Phase 3 (registry).

### 15.6 Phase 5: the Index map

**Goal.** The brief's "more detailed view of the index": every page a dot, orphans hollow and lightly linked pages
dithered, satellites and the tray, Around, the bar's backlinks chip, Index tab counts.

**Modules touched.** `editor.rs` (map hosting, `Owner::Map`, Pinch routing to the map, menu family), `pagesview.rs`
(Graph button, '%d links here'), `linkbar.rs` (Index button opens the map; the '%d pages link here' chip), `bgjob.rs`
(layout work kind), `canvas.rs` (`canvas_line_aa` row-span, `canvas_disc_pattern`), `actions.rs` (`index-map`).
Called: `graph.rs`.

**New modules.** `mapview.rs`.

**Data and settings.** graph.ini per notebook (section 13.3).

**Runner commands and probes.** No new commands (`waitbg` exists). Probes `editor.map.*` (section 11.8).

**Scenarios and unit tests.**
- Unit: canvas tests (the span line equals the old loop pixel for pixel over 300 random edges and widths; pattern
  coverage within 1/16 of each level; a host timing guard under 0.05 ms per long edge; golden shots unchanged);
  graph.ini round trip with unknown lines and a gen mismatch (wherever the codec lives); the graph model and layout
  tests are the pure branch's.
- Scenarios: `z_map_1` (a 40-page fixture with 30 links and 6 names: `editor.map.nodes 40`, satellites and tray as
  expected; shots upright and sideways, EN and ES, both styles, CHECK_FIT; tap at a node: selected; again:
  `editor.page` changes and the back list grows by 1); `z_map_2_around`; `z_map_3_pan_zoom` (presents fast, then ui);
  `z_map_4_big` (a generated 2000-page notebook: `editor.map.layout` seed then done after `waitbg`;
  `main.longest_unpumped_ms is ok`); `z_map_5_corner` (Back sits right of the clock zone; a corner tap toggles the
  clock and does not deselect or go back); the bar chip opens Around this page.

**E-ink.** Open and close GC16; selection GL16 of small rects; pan and zoom DU at most every 180 ms then GL16; the
bgjob layout swapped in with one GL16 when idle 500 ms.

**Owner acceptance on the Kindle.**
1. Open the map from the bar's Index button or the Index tab's Graph button. Every page is a dot: well-linked pages
   solid, lightly linked ones dithered, unlinked pages small hollow dots around their section's named page or in the
   'Not linked yet' tray; the open page has a double ring.
2. Tap a dot for its card; tap it again to go there; go-back returns.
3. Pan and pinch: rough previews while moving, crisp when you let go. Around this page shows one or two links deep.
4. Your biggest notebook opens quickly on a rough placement and tidies itself once the layout finishes, without pen
   lag.
5. The bar's 'N pages link here' chip opens the map around the page; Index tab rows show how many links point at
   each entry.

**Dependencies.** `graph.rs` merged from `claude/rust-index-pure`. Phase 2 (areas and link data are final).

### 15.7 Phase 6: pen-button gestures

**Goal.** D8 completed: pen-button gestures recorded in the same Gestures sheet and stored in gestures.ini; today's
hard-coded triple-click and hold `clear-page` become built-ins; the PEN source on every assignable action.

**Modules touched.** `editor.rs` (`pen_button`, editor.rs:2038, and its fields at 103-110 replaced by penbutton calls;
the 1200 ms hold check in `editor_tick` moved), `gesturemap.rs` (pen slots live), `library.rs` (pen rows and the pen
record path; the Settings pen-button row keeps only the side-button cycler), `actions.rs` (PEN in every sources mask;
`hold-eraser` in the hold and drag slots only, momentary), `sim_sdl.rs` (if `side on|off` click sequences need a
helper), `app.rs`
(capture-blocked for bound drag slots).

**New modules.** `penbutton.rs` (pure: click, double, triple with the owner's 350 ms multi-click gap, hold at today's
1.2 s, drag; Button, Button2 and Eraser).

**Data and settings.** gestures.ini `pen.<Button|Button2|Eraser>.<tap|double|triple|hold|drag>` lines live (values an
action, `nothing`, or `pan|zoom` in a drag slot). Built-ins `pen.Button.triple=clear-page`, `pen.Button.hold=
clear-page`; `pen.Button.double=links` is not a default (D6).

**Runner commands and probes.** Existing `side on|off` and pen commands; probes `gestures.ACTION` include pen keys;
`pen.gesture` (last recognized).

**Scenarios and unit tests.**
- Unit: `rust/tests/test_penbutton.rs` (the 350 ms gap both sides, hold 1.2 s both sides, a drag is not a click, ink
  with the button held keeps today's meaning); `test_gesturemap.rs` (pen slots, built-in duplicates legal).
- Scenarios: `v_gestures_4_pen` (learn double-click for Links; a double-click on the page opens the bar after the
  350 ms gap; a triple-click still clears the page; a pen-button hold bound to `hold-eraser` erases while held and
  stops at the release); f_batch2 and z_tool_b_button_over_sheets stay green; the record
  card body switches to the pen wording ('Draw a shape with one finger, swipe or tap with two fingers, or use the
  pen's button.').

**E-ink.** As the bound action's; a bound drag slot draws nothing on the page.

**Owner acceptance on the Kindle.**
1. In Settings > Gestures, record a pen double-click for Links: on the page it opens the bar after the short gap.
2. A triple-click still clears the page; a 1.2 s hover hold still clears it.
3. Clear the triple-click in the sheet: it no longer clears the page.
4. Live view: a bound pen drag shows no ink in the browser.
5. Bind the pen button's hold to Hold eraser in the Gestures sheet: the pen erases while you hold the button and
   writes again when you let go.
6. Every new screen fits in English and Spanish, upright and sideways.

**Dependencies.** Phase 3 (the sheet and the registry). No pure module from `claude/rust-index-pure`.

---------------------------------------------------------------------------------------------------------------

## 16. The synthesized plan's P1-P7 mapped to the six phases

| Synthesized step | Old phase | New phase | Notes |
|---|---|---|---|
| 1 Device touch log | P1 | 1 | plus the opt-in touch-trace file and the one-finger owner script |
| 2 touchgest extraction and fixes | P1 | 1 | plus `k_ms`, `TOUCH_F_BROAD`, `TOUCH_F_CANCEL`; leaves feature A's pen hook alone |
| 3 Contact-level runner input | P1 | 1 | stamps t_ms offsets (no virtual app clock), a monotonic frame clock; no window emulation (no SDL window in the crate) |
| 4 Symbolic panel taps | P1 | 1 | before `P_LINKS` |
| 5 kbhost | P1 | 1, 2 | small: `app_keyboard_pointer`/`_tick` exist; adds the toast floor and the split host (1), the dock (2) |
| 6 Canvas primitives | P1 | 1, 2, 4, 5 | each primitive in the phase that first needs it |
| 7 Classification, dispatch, built-ins | P2 | 1 | edge bands dropped (D5); `landscape_fit` added (G12) |
| 8 Area cache and the eye | P2 | 1 | legacy boxes in 1, areas in 2 |
| 9 The Links bar, P_LINKS | P2 | 1 | row hold not wired until 2 (no Pages-view jump) |
| 10 Peek, commit, cancel, prefetch, back list | P2 | 1 | corner tap never commits |
| 11 Find sheet | P2 | 1 (GO), 2 (PICK) | |
| 12 Area lines, hit test, migration, Copy page | P3 | 2 | |
| 13 ACT_LINK | P3 | 2 | `ActKind::Link` applied by the editor |
| 14 Ink-free making, Link to... | P3 | 2 | |
| 15 The link card | P3 | 2 | |
| 16 Registry, limits, BindKind | P4 | 3 | sources gain SHAPE |
| 17 The remaining actions | P4 | 3 | |
| 18 Settings > Gestures, record mode | P4 | 3 | plus finger-shape recording (shape design steps 18a-18d) |
| 19 Hover rule, three fingers, one-finger swipes | P4 | 3 | one-finger swipes classified with the finger capture |
| (shape design) finger shapes at runtime | (P4) | 3 | calls `fingershape.rs` |
| 20 shape.c recognizer | P5 | 4 | now `shape.rs` on the pure branch: only the wiring here |
| 21 Smart shapes in the editor | P5 | 4 | |
| 22 shape-last, ACT_REPLACE, v1.5 kinds | P5 | 4 | |
| 23 Graph model and layout, graph.ini | P6 | 5 | model and layout are `graph.rs` on the pure branch; graph.ini I/O and bgjob wiring here |
| 24 The Index map view | P6 | 5 | Back moved right of the clock zone |
| 25 Pen-button gestures | P7 | 6 | ships alone; `penbutton.rs` built here (no pen lane in Rust) |
| 26 Docs, Spanish, fit sweep, Index tab counts | P7 | every phase; counts in 5 | |

The old P1 "ships alone, nothing new on screen" build no longer exists: its measuring and hardening ship inside
phase 1, and phase 3 freezes the thresholds from phase 1's traces.

---------------------------------------------------------------------------------------------------------------

## 17. Hard rules (from CLOUD.md and PORTING.md, as they apply here)

1. No device access, ever; never run a GUI binary.
2. No em dashes in code comments, docs or UI.
3. Every UI string in English and Spanish, in the same change; strings fit both orientations (CHECK_FIT); `tr` /
   `n_` / `c_` / `nc_` / `ng_` with a `// TRANSLATORS:` line above carrying the width budget; formats through
   `cformat!`/`csnprintf!`. Changed meanings get new msgids, never a changed translation.
4. Never change the `.qpg` page format: no header version bump, no tool id >= 10, no ext flag.
5. Never add an `MXCFB_WAIT_FOR_UPDATE_SUBMISSION` wait or a second wait on one display marker. No new display calls:
   screens use `app_invalidate` and one `app_flush` per loop (plus explicit separate flushes where stated).
6. The main loop never blocks ~100 ms (the pen's evdev buffer overflows): long work pumps input (`work_pump`), as
   listed in section 14.4; nothing on the main thread holds it longer than ~100-175 ms while the pen is near.
7. Pen mode tolerates only DU: no GL16 and no toast while the pen is down.
8. Byte-exact formats (PORTING rule 1): settings.ini keeps every existing line's bytes and order, new undotted keys
   appended after `language`; shortcuts.ini and controller.ini unchanged; links.ini may change (R1); new files
   (gestures.ini, graph.ini, touch-trace.txt) are documented byte for byte in FORMAT.md.
9. Logs: new log lines are `qlog!` with printf formats; existing lines unchanged; the touch log never writes
   coordinates while input is not grabbed (the PIN).
10. The runner contract: existing commands keep their exact event sequences (`tap`, `touch`, `drag`, `pinch` stay
    legacy direct injection with PinchMove value 0); every existing `app_probe` key and value is unchanged; new keys
    are documented; an unknown command still fails the run.
11. Timing: `now_ms` and `k_ms` are wrapping u32 compared with `wrapping_sub`; recognizers time by the event's time,
    never by `now_ms` in a batch; holds and pauses are checked in ticks only when `a.input_drained`.
12. Global state: module state is `thread_local! RefCell` with short borrows; never call another module,
    `work_pump()` or a callback while holding a borrow (the touch extraction's frame build and the pen hook stay
    outside borrows); no global App pointer (hooks read atomics).
13. unsafe only for FFI, ioctls, fork/exec/_exit, signals, mmap and raw fds, each with `// SAFETY:` (the EVIOCGABS
    and EVIOCGMTSLOTS calls of phase 1 are ioctls).
14. Processes: bgjob children via libc fork, ending with `libc::_exit`; no other threads (the map's layout child in
    phase 5 follows bgjob's rules).
15. Float rules: f32 ops with a double operand done in f64 and rounded back; never `mul_add`, use `fmla`; `zoom_max`
    keeps today's expression order so the ceiling is bit-identical. fingershape.rs is integer on purpose.
16. Memory: allocations whose size comes from data use `try_reserve_exact` and keep a failure path.
17. Existing behaviour not named by a ruling or this plan stays byte for byte (PORTING rule 2 governs everything this
    plan does not deliberately change; every deliberate change is listed in its phase).
18. Commit in small logical commits on the phase's own branch with CLOUD.md's attribution lines; push the branch;
    never push to `main`, `claude/rust-port` or `claude/beta5-features`.

---------------------------------------------------------------------------------------------------------------

## 18. Risks

- Pen near. 'Write, lift, swipe' often starts with the pen hovering; phase 1 rejects multi-finger gestures then, so
  the headline gesture can feel refused. The rate-limited toast keeps it from feeling like a hang; phase 3 relaxes the
  rule from the log. The owner tests this in phase 1.
- The panel's slot count, TOUCH_MAJOR range and palm flagging are unknown until the phase 1 log: three fingers and the
  size guard stay hidden or log-only until then.
- D5 without edge bands: zoomed in (or sideways with the setting off), navigation is only by the panel buttons, keys,
  and (phase 3) finger shapes. The pan hint and the always-visible panel buttons mitigate; D5 decides this, so it is
  not asked.
- `landscape_fit` changes today's sideways default; mitigated by the hint, the Display row, the action and the phase
  1 notes; at least five rotate scenarios need the header.
- Pan with pinch zoom off is new behaviour where the page scrolls.
- How a two-finger drag feels changes: no zoom drift while panning, swipes no longer move the view, kernel time.
  k_pinch_gate guards the dead zone, not the feel; the owner tests it.
- Peek state could leak into edits. Mitigated by design (a display state; index, page, history and hit tests stay on
  the origin; every way out commits or cancels explicitly); scenarios cover autosave, sleep and leaving during a peek.
- Peek cost and memory: a cold peek of a dense or PDF page can take up to 1.2 s; GL16 peeks build ghosting until the
  commit's or cancel's GC16; the cache takes up to about 14 MB; Kindle RAM headroom is not measured; every allocation
  has a slower fallback.
- Migration is a one-way rewrite of the owner's links.ini. Using the 7 mm grown box or counting erase strokes would
  silently change tappable areas, so fixtures are copies of real device notebooks. f9d13f5 carries area lines only up
  to 16 KiB (about 430 links), then goes read-only there and never overwrites the file.
- Areas are invisible to every other reader (older versions, the share page, Live view's page layer); only the PDF
  carries them, as /Link annotations. The eye only helps on the device.
- Smallest-wins replaces last-drawn-wins, so beta.5 links with overlapping boxes may resolve differently (acceptable
  while beta.5 is unreleased; test_nav covers it).
- A link step writes a notebook-wide sidecar from inside a page's history: a failed save must drop the step after
  `nav_load` restores the file; redo after the target page is deleted must do nothing.
- Renaming from a link can create a section start, which moves the Index tab's section highlight for later pages.
  The wording and the toast name it, but it may still surprise.
- Finger shapes: one-turn shapes are refused by default (open question 1); a similar unrecorded shape can fire the
  recorded one (a square can fire a circle); synthetic palm-like drifts reach the matcher at 2.4 per 1000, so BROAD,
  the palm flag and the pen rules are mandatory and shapes do not ship until the writing traces fire 0 times; the slip
  check takes 1-2 s at Use; thresholds were tuned on synthetic fingers.
- Smart shapes: very large or slow handwriting can show a preview and then revert (two DU frames); the corpus is small
  (47 strokes, read-time stamps); the region-render preview may be slow on dense pages (tile backup if the shape-log
  shows renders over about 300 ms).
- The Index map: large layouts share bgjob's single queue, so a PDF mirror can delay them and the seed placement must
  look acceptable meanwhile; floats differ between arm64 and armv7 (tests use invariants); the `canvas_line_aa`
  rewrite touches every Gothic drawing and must stay bit-identical.
- Panel growth to 13 buttons and the moved Settings bottom row break raw coordinate taps: symbolic taps and the
  updated raw taps land before or with them.
- The Buttons sheet becomes parametrized by BindKind, which can regress the Controller flow: v_controller*.qs stay
  green at every step.
- A new keyboard surface that bypasses the keyboard host brings back the Index keyboard lag: `keyboard.draws` is
  asserted in every typing scenario.
- The pure modules' real APIs may differ from the surfaces assumed here (they are empty slots today): each phase
  verifies its module's surface first and adapts the call sites only.
- Feature contracts: if feature B decides corner taps at the down, phase 1 asks it to decide at the lift (or hand a
  cancel up), or shapes from the corner are lost (phase 3); if feature A's flag means something else, section 3.1's
  placement is revisited; non-ink pen contacts that forget the flag show phantom ink in the browser.
- Live view streams `App.screen`, so viewers see the bar, the card, the eye, the trail and the map (open question 3).
- `c-check` parity: new scenarios need `# rust-only`; forgetting it fails the C run; and every converted existing
  scenario (about 70 in phase 1, more as the panel and Settings rows change) stops being guarded by the C run (open
  question 4).

---------------------------------------------------------------------------------------------------------------

## 19. Decisions taken in this plan (not owner questions)

- No edge bands; on a scrolling axis every two-finger translate pans, per axis (D5 decides it).
- `pinch_zoom` off disables zoom only; a classified pan still pans.
- Previous page on the first page toasts 'This is the first page.', mirroring D6's last-page toast.
- `pen.Button.double=links` is not a default (D6 lists the built-ins exhaustively); it may stay as a user example in
  FORMAT.md.
- The clock corner is one finger only; a two-finger tap there is `touch.2.tap`. The corner wins over every one-finger
  tap consumer, including the bar's commit while peeking: a corner tap toggles the clock and nothing else happens.
- The Index map's Back moves right of the clock zone; Link to... and Find docked at the top start below it; the card
  and the eye's tags avoid it.
- A PenDown during an `Owner::Link` session ends that session silently and draws.
- In phase 1 a bar row hold is not wired, so no new jump to the Pages view is introduced before the card; the page's
  own hold keeps today's picker until phase 2.
- For an entry with one link the bar's card equals the page's card; for N > 1, Redraw the area is absent (D1).
- Card row order: Rename, Change where it goes, Redraw the area, Delete link (R2 names the rows, not their order).
- Rename (D3) is a notebook-wide name change and is not in the page's undo; link edits are.
- The match toast names every action except self-toasting or full-screen ones.
- Finger shapes act on the notebook page (and the record card) only in v1, not in the Pages view or the Index map.
- The 32-shape ceiling is kept.
- `landscape_fit` lives in the Display sheet (Settings has no room sideways, library.rs:1395).
- The Pages view's 'Go to page...' opens the Find sheet over the page.
- The keyboard host stays in app.rs; no kbhost module.
- New scenarios carry `# rust-only` so the C parity run keeps working (open question 4 covers what it then stops
  guarding).
- Finger shapes work while the Links bar is up (it follows from D4, "anywhere on the page", and D2, "tap the page to
  stay", which is a tap, not a moving finger): the bar's own shape cancels to the origin, another menu's shape swaps
  (after a commit if peeking), anything else is refused and fades.
- While the bar is up, two fingers down always cancel to the origin, whatever gestures.ini binds (D2 names them
  unconditionally); the mirror rule covers the other menus.
- A finger tap on the panel while the bar is up commits (stays) and presses nothing, except the Links button, which
  cancels (it is "the Links button again").
- Change where it goes from the card hides 'Choose from the pages...' (R2: the page stays in view).
- `gestures` is assignable to every source (a gesture can switch them off, the sheet's row turns them on);
  `hold-eraser` is not assignable to touch patterns or finger shapes (no release; open question 2); Enable shortcuts
  and the Bluetooth controller take only the sources they cannot switch off; language, sleep after and the update
  channel are actions; calibration is not.
- Phase 1 reads gestures.ini (README phase 3 brings the sheet that writes it): a hand-edited `=nothing` can turn off a
  built-in before the sheet exists, and the bar's fixed cancel keeps D2 true whatever is bound.
- Phase 1 details settled against the code in PHASE1-BRIEF.md and folded in here: `TgContact` carries `slot`, `nx`
  and `ny` (calibrate.rs:310 reads them); the touch absinfo goes on a new log line; `main.longest_unpumped_ms`
  prints `ok` or the gap; the pen toast arrives as `PinchEnd.value = 1`; the one-time tips of bits 0 and 8 show only
  to upgrading owners and `SETTINGS_VERSION` is not bumped; peeks read through the editor's reader, the commit moves
  the peeked page into the session cache, and phase 1 needs no edit generation; the box cache rebuilds lazily;
  `go-back` while the bar is up is special-cased before `sheet_open`; the commit-then-swap is explicit code in the
  menu branch; 'Go to page' reuses its existing translation 'Ir a la página'; `z_bar_0_make` builds the linked
  fixture; the read-only and unreadable states are unit tests; the keyboard dock has no phase 1 caller and moves to
  phase 2; the Rust crate has no window emulation.

---------------------------------------------------------------------------------------------------------------

## 20. Open owner questions

At most five, each truly open (no ruling decides it), each with a recommended answer.

1. **One-turn finger shapes.** V, check mark, L, U and C are refused at recording as "Too easy to make by accident",
   because a slip of the hand can draw them. Keep them refused, or offer "Use anyway" with stricter gates (a larger
   minimum size, a faster start, the pen away for 3 s)?
   Recommended: keep them refused in phase 3; revisit once the phase 1 traces measure how often real slips happen.
2. **Hold eraser from a finger.** Your brief says the Gestures sheet assigns anything the controller can. Hold eraser
   works while a button is held, and a two-finger swipe or a finger shape has no "held" part, so this plan lets keys,
   controller buttons and (phase 6) the pen button's hold hold the eraser, but not finger gestures. Should a finger
   gesture get a latched form instead ("eraser until the pen next lifts")?
   Recommended: no latched form; bind the existing Eraser tool (`tool:eraser`, a toggle) to a gesture or shape if you
   want the eraser from a finger, and keep Hold eraser for buttons, which have a release.
3. **Live view viewers see the overlays.** Viewers of Live view's screen stream see the Links bar, the link card, the
   eye's boxes, the finger-shape trail and the Index map, as they see the panel today. Hide them from viewers (needs
   skip rects in the screen stream), or accept?
   Recommended: accept for now; it shows viewers what you are doing, and hiding them costs a stream change unrelated
   to the Index hook.
4. **What `make c-check` still guards.** c-check runs the scenarios against the C binaries, which have none of this.
   New scenarios are marked `# rust-only` and skipped there. But phase 1 also changes about 70 existing scenarios:
   the Links button moves every panel button below Next (about 65 files tap them by coordinate), the Index group
   moves the Buttons list, the new Display row moves that sheet, and Go to page... opens Find. Converted, they must
   be `# rust-only` too, so c-check stops guarding them (and `y_all_screens`). Options: (a) accept that, and retire
   c-check for scenarios once phase 1 lands; (b) put the Links button after Library at the panel's end, which keeps
   the upright panel coordinates and saves about 65 conversions, though the button then sits away from Next and
   Previous; (c) keep C-compatible raw-tap copies of the converted scenarios.
   Recommended: (a). The C tree is reference only, the Rust gate is the authority, and the Links button belongs beside
   the page buttons, where the hand already is.
