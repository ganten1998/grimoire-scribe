# Phase 4 implementation brief: smart shapes (Rust)

This brief is self-contained: with it and the repository you can build phase 4. It applies
`docs/index-hook/PLAN-V2.md` (the authority; section numbers such as 10.4 are its sections; Part S section 10 is this
phase's design), the owner's brief and rulings in `docs/index-hook/README.md` (R1's "a visible border is optional
ordinary ink, drawn with smart shapes" and D7's Shapes toggle), `docs/index-hook/CLOUD.md` and the contract
`rust/PORTING.md`. Its style follows the earlier briefs. Brief decisions are marked in place and listed in section 16.

Line numbers are Rust lines at `c2333a9`; phases 2 and 3 move them, so search by the function name. Phase 2's and 3's
names are cited from `PHASE2-BRIEF.md` and `PHASE3-BRIEF.md`; verify each against the merged code first (1.1).

The owner's words for this phase, verbatim:

> if users want visible borders, they can draw them with a new smart tool system (should be more aware and less
> accidental input prone than amazon's, for drawing circles that snap, squares, lines, triangles, whatever else) on by
> default with a toggle button in the toolbar.

---------------------------------------------------------------------------------------------------------------

## 0. What phase 4 is

One Kindle build the owner installs and tests:

1. **Hold to snap** (PLAN 10.1, 10.3, 10.4): draw a shape, hold the nib still (0.55 s by default): one DU frame shows
   the clean shape in the same pen and size. Lift to keep it; move on and your own line comes back and the stroke
   continues. Handwriting never snaps.
2. **The recognizer** `shape.rs`, merged from the pure branch and finished (section 1.2): line, rectangle, square,
   quad, triangle, ellipse, circle; paper-aware snaps; the writing-burst gate.
3. **One Undo removes a shape** (PLAN 10.6): the shape is the stroke; the raw stroke is never added.
4. **Shape the last stroke** (`shape-last`, PLAN 10.6): all the gates, no hold, one undoable Replace step.
5. **The Shapes panel button** (D7, PLAN 10.7): directly under the eraser, filled when on, slashed outline when off,
   never inverted; `PANEL_BUTTONS` 12 -> 13.
6. **The tool picker row**: 'Smart shapes: On' and 'Hold: Normal' chips.
7. **Three actions** in the registry: `shapes` (TOGGLE), `shape-hold` (CYCLE), `shape-last` (ONE_SHOT).
8. **settings.ini** `shapes` (default 1, on), `shape_hold_ms` (default 550); `hints` bit 5 (the first-snap tip).
9. **The shape-log** switch for device tuning (PLAN 10.8).

### 0.1 Out of scope

| Not in phase 4 | Phase |
|---|---|
| The Index map | 5 |
| Pen-button gestures (a pen-button binding of `shapes` arrives with the PEN source) | 6 |
| Arrows, polylines and arcs (`ShapeParams::extras`) unless they pass the corpus gate (1.2) | v1.5 |
| An under-ink tile backup for slow previews (only if the shape-log shows region renders over about 300 ms) | later |
| Snapping erase strokes, white ink, the precise or whole-stroke eraser, read-only pages, the link lasso | never |

Known in this build (owner notes): arrows, polylines and arcs are off unless this build's notes say they passed; a
very large or slow handwritten loop can show a preview and revert (two DU frames, no harm).

---------------------------------------------------------------------------------------------------------------

## 1. Branch, base, and what must be true first

- Base: the finished phase 3 branch (`claude/rust-index-p3`) as the owner's session merged it. Branch:
  `claude/rust-index-p4`. Never push to `main`, `claude/rust-port` or `claude/beta5-features`.
- Set up the machine with `docs/index-hook/CLOUD.md`.

### 1.1 Names this phase builds on (verify each first)

| Name | Where | What phase 4 needs |
|---|---|---|
| `draw_sample` | editor.rs:2344 | the dwell anchor, O(1) per sample |
| `editor_tick`, `a.input_drained` | editor.rs:5214, mainloop.rs | the hold check by wall time |
| `draw_page(a, r)`, `show(a, r, mode)`, `page_canvas`, the settle rect | editor.rs:1797, 1699 | the preview |
| `brush.rs` stamping, `stroke_new`, `stroke_append` | brush.rs, page.rs | the snapped stroke's pixels and points |
| `history_push_add(h, index, &Stroke)`, `ActKind` with phase 2's `Link = 4`, `apply`, `history_undo_kind` | history.rs:81, PHASE2-BRIEF 4.1 | `ActKind::Replace = 5` |
| `PANEL_BUTTONS = 12`, `panel_height`, `panel_fit`, `draw_panel`, the `P_*` ids up to `P_LINKS = 24`, `editor.panel_at`, `tap panel NAME` | panel.rs:10, editor.rs:40-70, 512 | `P_SHAPES`, 13 buttons |
| the tool picker (`ts`) and its ink row | editor.rs | the new row |
| `actions.rs` (`ActionInfo`, groups, `SRC_*`), `editor_shortcut_at` | PHASE3-BRIEF 5, 6 | three actions in Editing |
| phase 2's link lasso (`Owner::Lasso`, the strip) | PHASE2-BRIEF 6 | never snaps |
| feature A's quick ink retirement and `live_reset()` | live.rs:666 | the snapped stroke replaces the raw quick ink |
| `erasersample.rs` and `assets/eraser-sample.qpg` | | the owner's negative strokes |

### 1.2 shape.rs from the pure branch: merge it, then finish it

Read `docs/index-hook/PURE-MODULES.md` on `claude/rust-index-pure-p4zqq5` first and take its newest commit if the shape
review is finished there; otherwise 7cbcf0f. Bring in only shape:

```
git checkout <pure commit> -- rust/src/shape.rs rust/tests/test_shape.rs rust/tests/shape_corpus
```

plus `pub mod shape;` and the prelude line by hand (as phase 3 did for fingershape). The commit message names the
source commit.

**The API as it is** (PLAN 10.2 assumed one function; use these):

| Need | shape.rs |
|---|---|
| recognize | `shape_recognize(pts: &[PenSample] /* page px, the stroke whole, its resting samples included */, prm: &ShapeParams) -> ShapeResult { kind: ShapeKind, reason: ShapeReason, features: ShapeFeatures, pts: Vec<PenSample> }` |
| parameters | `shape_params(dpi, zoom, paper: PaperKind, page_w) -> ShapeParams { dpi, zoom, paper, page_w, burst, extras }`; set `burst` from `shape_in_burst(last_lift: Option<&PenSample>, first: &PenSample, dpi, zoom)`; `extras` false |
| the snapped stroke | `shape_stroke(raw: &Stroke, &r, dpi) -> Option<Box<Stroke>>` (the raw stroke's tool, params, seed and ext; only the points differ) |
| the hold radius and burst hold | `SHAPE_HOLD_MM = 1.0`, `SHAPE_BURST_HOLD = 1.5` (the timing is the editor's) |
| the log | `ShapeFeatures` (no coordinates), `ShapeReason` |

**Open review items to close (commit 2, each with a test in `test_shape.rs`):**

1. *A grid snap of one line end undid the angle snap (high).* a7dd214 ("a snapped line keeps its angle on the grid")
   addresses it: confirm with a test that a line drawn at 2 degrees on dot-grid paper comes out level AND with both
   ends on grid nodes when both are within `SHAPE_GRID_SNAP_MM`, and level with its ends unsnapped otherwise.
2. *The closure gate's measure*, 3. *the end-of-stroke dwell in the corpus*, 4. *the corpus noise level*: check each
   against the later commits (9eb49e9, f5b946b, f89e8e6); close what they fixed with a named test, fix what remains.
5. *PORTING rule 16:* `ShapeResult.pts` and every data-sized buffer in shape.rs use `try_reserve_exact`; on failure
   `shape_recognize` returns `ShapeKind::None` with `ShapeReason::Degenerate` (the stroke stays as drawn).
6. *The corpus gate for v1.5 kinds:* run the corpus with `extras` on. Arrows, polylines and arcs ship on only if
   they keep 0 snaps on the 1900 synthetic negatives and on the owner's eraser-sample strokes at written size, and at
   least 95% of their positives; otherwise `extras` stays false (the default) and the owner notes say so.
7. Update PURE-MODULES.md's shape row.

### 1.3 Pre-flight

`make rust-check` on the base; record the Linux-only failures.

---------------------------------------------------------------------------------------------------------------

## 2. Hard rules that apply to every commit

PHASE2-BRIEF.md section 2's rules 1 to 11 apply as written. Phase 4 adds:

12. Pen mode tolerates only DU: the preview, its cancel and nothing else while the pen is down; no GL16, no toast.
13. The `.qpg` format never changes: a snapped shape is an ordinary `Stroke` of the raw stroke's tool (0-9), params,
    ext, tip and seed; no flag says it was snapped. Older versions draw it as the pen stroke it is.
14. One pen-down is one moment (the owner's undo rule): a snapped stroke is one `history_push_add`; the raw stroke is
    never added, never in the history, never saved.
15. The raw brush state is never touched by a preview: the raw stroke keeps growing in its buffer while the preview is
    up, and comes back exactly when the preview is cancelled.
16. Floats: shape.rs is f64 inside with `fmla` (PORTING); the editor's new code follows the same rules (no `mul_add`).
17. Never block ~100 ms with the pen down: the region render pumps every 64 rows, the stamping every 8 points.

---------------------------------------------------------------------------------------------------------------

## 3. The trigger (`editor.rs`; PLAN 10.1)

- EditorState gains `sh: ShapeHold { anchor: (f32, f32) /* screen px */, anchor_k: u32, still_fired: bool, preview:
  Option<ShapePreview>, last_lift: Option<PenSample>, last_kind: ShapeKind, last_reason: ShapeReason }`.
- `draw_sample` (O(1)): for an ink stroke in scope (below), a sample more than `SHAPE_HOLD_MM` screen mm (11.8 px at
  300 dpi) from the anchor moves the anchor there and resets `anchor_k` to the sample's `k_ms` (`t_ms` when 0) and
  `still_fired`.
- `editor_tick` (it must run at least every 30 ms while a stroke is down; verify the main loop's poll timeout while
  the pen is down and, if it is longer, shorten it only while `s.owner == Owner::Draw` and `settings.shapes`): when
  `a.input_drained`, `settings.shapes`, the stroke is in scope, `!still_fired`, and `now - anchor_k >= hold`, where
  `hold = shape_hold_ms`, times `SHAPE_BURST_HOLD` when the stroke started in a burst: set `still_fired` and run the
  recognizer (3.1). Wall time, never sample spacing: a still pen may send nothing. Once per still period.
- Scope: ink strokes of the nine pens, the highlighter and the side button's highlighter. Never: erase strokes, white
  ink, the precise or whole-stroke eraser, read-only pages, phase 2's lasso (`Owner::Lasso` is not `Owner::Draw`),
  phase 3's finger shapes (fingers never draw).
- The burst flag: at PenDown, `shape_in_burst(s.sh.last_lift.as_ref(), &first, dpi, zoom)`; `last_lift` is the last
  ink stroke's last sample, set at every ink PenUp.

### 3.1 Recognize

`shape_recognize(&stroke_points_in_page_px, &prm)` with `prm = shape_params(dpi, s.zoom, page.paper, pgw)` and
`burst`. The stroke's points are the in-progress stroke's buffer (`PenSample`s in page px). On a recognizer cost over
the pen's budget (a very long stroke), `shape_recognize` bounds its own work (MAX_DIAGS); the editor calls `work_pump`
before and after it. `kind == None`: nothing visible; the shape-log records the reason. Otherwise the preview.

---------------------------------------------------------------------------------------------------------------

## 4. The preview (PLAN 10.4)

1. `region = union(raw bbox, shape bbox)` in screen px, padded by the brush's max radius + 2, clipped to the page's
   screen rect.
2. Copy that region of `page_canvas` (the pixels with the raw stroke) into `ShapePreview.saved` (`try_reserve_exact`;
   failure: no preview for this stroke, the shape-log says `nomem`).
3. `draw_page(region)` renders the page without the stroke in progress (it is not in the page until the lift), with
   `work_pump` every 64 rows.
4. Stamp the snapped stroke (`shape_stroke(&raw, &r, dpi)`) into `page_canvas` with the brush, `work_pump` every 8
   points.
5. `show(region, RefreshMode::Fast)`: one DU frame. Pen mode is kept.

While the preview is up:
- samples within 1.5 screen mm of the anchor (the hysteresis over the 1.0 mm hold radius) are appended to the raw
  stroke but not drawn (live ink of them is suppressed: they would draw over the preview);
- a sample more than 1.5 mm from the anchor cancels: copy `saved` back into `page_canvas`, draw the samples received
  since the hold as ordinary live ink, `show(region, Fast)` (one DU), drop the preview; the stroke continues as if
  nothing happened, and a later still period may preview again;
- holding longer does nothing more.

At the lift (PenUp or the pen leaving) with a preview up: the snapped stroke is added to the page with ONE
`history_push_add` (the raw stroke never is); no redraw (the canvas already shows it); the region joins the settle
rect for the usual GL16 after the pen leaves. `s.sh.last_kind` is set (the `editor.stroke_shape` probe). The first time
ever (`hints` bit 5 clear) the tip toasts after the settle: 'Shape made. Undo takes it away; the panel's shapes button
turns this off.'

**Live view** (PLAN 3.1): the raw stroke's quick ink reached `/live/pen` while drawing (it was ink). Feature A retires
quick ink up to a tile watermark when a present covers it (live.rs, module comment). Check with a live scenario that
after a snap and its settle the browser model holds no raw quick ink; if it does, call `live_reset()` (live.rs:666, the
browsers clear their quick ink for it) at the commit.

---------------------------------------------------------------------------------------------------------------

## 5. Undo and shape-last (`history.rs`, `editor.rs`; PLAN 10.6)

- A snapped stroke is one Add: one Undo removes it (the owner's undo rule; nothing new in history for the hold).
- `shape-last` ('Shape the last stroke'): when the top undo step is the Add of the page's newest ink stroke and that
  stroke is in scope: `shape_recognize` with no hold (the same gates; the recognizer's hold cut only straightens the
  last 2 mm), at the zoom of the current view and `burst` false. A shape: one new `ActKind::Replace` step, then
  `draw_page(union of both boxes)` pumped and `Ui` of that union. Otherwise toast 'No shape in the last stroke.'.
- history.rs:

```rust
pub enum ActKind { Add = 0, Erase = 1, Paper = 2, Clear = 3, Link = 4, Replace = 5 }
/// Swaps page.strokes[index] for `shape` and records both; returns the dirty rect (both boxes).
pub fn history_push_replace(h: &mut History, page: &mut Page, index: i32, shape: Box<Stroke>) -> Rect;
```

  `apply` swaps the two strokes at `index` for undo and redo (dirty = the union of their boxes); memory follows the
  Add/Erase steps' ownership (history.rs `ownership_follows_the_page`). `editor.undo_kind` gains `replace`.
- Undo after a snap gives the page without the shape (the freehand stroke was never in the page); Undo after
  shape-last gives the freehand stroke back.

---------------------------------------------------------------------------------------------------------------

## 6. The toggle, the picker row, the actions, the settings (PLAN 10.7)

- **Panel:** `P_SHAPES` (new id after phase 2's last `P_*`), directly under the eraser in the pen-behaviour group.
  On: filled glyph F0831 on a plain button; off: outline glyph F0832 with a 4 px diagonal slash (new
  `canvas_slash(c, r, thick)`). NEVER inverted: on this panel inverted means the current tool. `PANEL_BUTTONS` 13;
  `panel_height` and `panel_fit` match `draw_panel`; sideways with 4 pins `panel_fit` gives about 0.93 scale (94 px
  buttons, about 8 mm); `test_panel.rs` encodes it. Panel name for `tap panel` and `editor.panel_at`: `shapes`.
- **Tap:** `settings.shapes` flips, `settings_save`, `Ui` of the button. Toasts 'Smart shapes on. Hold the pen still at
  the end of a shape.' / 'Smart shapes off.'.
- **Raw panel taps:** every button below the eraser moves by one slot. Phase 1 converted the panel taps to `tap panel
  NAME`; grep again (`grep -lE '^(tap|pentap) 17[0-9][0-9] ' tests/scenarios/*.qs`) and convert any raw tap left
  below the eraser before `P_SHAPES` lands, each checked with `editor.panel_at.X.Y`.
- **Tool picker:** a labelled row under the ink row with two 520 px chips: 'Smart shapes: On' / 'Smart shapes: Off' (a
  tap toggles) and 'Hold: Normal' (a tap cycles Quick 400, Normal 550, Slow 900 ms). The picker's height grows by one
  row; it must still fit sideways (CHECK_FIT).
- **Actions** (registry, Editing group, which becomes 10 rows): `shapes` (TOGGLE; label 'Smart shapes'), `shape-hold`
  (CYCLE; label 'Shape hold time'; toast 'Shape hold: Normal (0.55 s)' and its Quick and Slow forms), `shape-last`
  (ONE_SHOT; label 'Shape the last stroke'); sources KEY | PAD | TOUCH | SHAPE (PEN in phase 6). gesturemap now accepts
  them in `touch.*` and `shape.*` lines.
- **settings.ini:** `shapes` 0|1 default 1 (2 and 3 reserved: read as 1), `shape_hold_ms` 300..1500 default 550
  (out of range: clamped), appended after phase 3's `rotation_lock`, with the full key checklist (PLAN 13.4: field,
  default, `settings_cb` arm, `put_int`, probe, FORMAT.md row, `test_probe.rs`). `shape-hold` cycles 400 / 550 / 900;
  a hand-set value in between shows as the nearest named one and the next press moves on from it.

---------------------------------------------------------------------------------------------------------------

## 7. The shape-log (PLAN 10.8)

- Switch: an empty `<store_root()>/shape-log` file, checked at `app_init` (like `touch-trace`). While present, each
  hold (and each shape-last) writes one `qlog!` line to grimoire.log (brief decision: no separate file):

```
shape: hold %s %s diag %.1f straight %.3f gap %.3f wind %.2f turn %.2f corners %d rms %.3f max %.3f axis %.3f burst %d zoom %.2f ms %d render %d
```

  kind name (or `none`), reason name, the `ShapeFeatures` fields (screen mm, ratios), the burst flag, the zoom, the
  recognizer's ms and the region render's ms. Never coordinates. A lift that kept the shape adds `shape: kept`, a
  cancel by moving `shape: moved on`, so the log tells snaps the owner kept from ones he rejected.
- The owner's evening with the shape-log sets the final gates; the tuning commit changes `shape.rs`'s constants and
  `test_shape`'s corpus together (the module is ours now).

---------------------------------------------------------------------------------------------------------------

## 8. E-ink refresh choices (no new display call, no new wait)

| Event | Waveform | Notes |
|---|---|---|
| Preview at the hold | Fast, the region | pen mode kept; one DU |
| Cancel by moving on | Fast, the region | one DU |
| Lift with a shape | nothing new; the region joins the settle rect | the usual GL16 after the pen leaves |
| shape-last | Ui of the union | pen not down |
| Shapes button | Ui of the button | |
| picker row chip | the picker's own press feedback | |

Never a GL16 or a toast while the pen is down; the first-snap tip waits for the settle.

---------------------------------------------------------------------------------------------------------------

## 9. The runner and docs/SIM.md

- Probes: `editor.stroke_shape` (the last snap's kind name, `none` before any), `editor.shape_preview` (`0`, or the
  kind while a preview is up), `editor.shape_reason` (the last hold's reason name), `settings.shapes`,
  `settings.shape_hold_ms`, `editor.panel_style.NAME` (`plain`, `selected`, `disabled`; for any panel button), and
  `editor.undo_kind` gains `replace`.
- New command (brief decision; PLAN 15.5 said none, but the owner's real strokes are the negative test): `pen strokes
  FILE.qpg [N] [HOLD_MS]` replays the first N strokes of a page file (default all) as pen down, moves at their own
  spacing and up, each stroke placed where it was on its page, with a still hold of HOLD_MS (default 0) before each
  lift. It reads the file with the existing page reader; an unreadable file fails the run.
- SIM.md: the command, the probes, and a paragraph "Holding the pen still in a scenario" (`pen down`, `pen move`s,
  `wait 700`, `pen up`; the runner ticks during `wait`).
- Selftest: `pass-rust.qs` gains `expect editor.shape_preview is 0` and `expect settings.shapes is 1`; `fail-rust.qs`
  gains `expect editor.shape_preview is circle`.

---------------------------------------------------------------------------------------------------------------

## 10. Unit tests

- `test_shape.rs`: the items of 1.2.
- `test_history.rs`: Replace: push swaps the stroke and returns both boxes; undo restores the raw stroke at the same
  index; redo the shape; interleaved with Add, Erase and phase 2's Link steps the order holds; eviction drops both
  strokes.
- `test_panel.rs`: 13 buttons; scale at least 0.93 sideways with 4 pins; heights match `draw_panel` upright and
  sideways for 0-4 pins.
- canvas: `canvas_slash` (thickness, inside its rect, 1-bit).
- editor.rs `#[cfg(test)]`: the dwell anchor (a sample at 0.9 mm keeps the anchor, at 1.1 mm moves it); the hysteresis
  (1.4 mm keeps the preview, 1.6 mm cancels); the hold time with and without the burst factor.
- `test_probe.rs`: `shapes` and `shape_hold_ms` undotted.
- `test_i18n.rs`: the toasts, the picker chips at 520 px, the action labels.

---------------------------------------------------------------------------------------------------------------

## 11. Scenarios

Every new file starts with `# rust-only`. Each scenario that draws checks `editor.live_matches_render is 1` after its
strokes.

- **`z_shape_1_circle`**: `pen down` at a point on a circle, `pen move`s around it, `wait 700`, `pen up`:
  `expect editor.stroke_shape is ellipse` or `circle` (record which on the first run), strokes +1; `display.presents`
  holds one `fast` of the region during the hold and no `ui` or `full` until the pen leaves; one Undo gives strokes +0.
- **`z_shape_2_move_cancels`**: a box, hold (preview up: `editor.shape_preview` not 0), then move on 5 mm and lift:
  `editor.shape_preview is 0`, `editor.stroke_shape is none`, the page has the freehand stroke (its point count equals
  the drawn samples).
- **`z_shape_3_writing_never`**: `pen strokes assets/eraser-sample.qpg 47 700`: every stroke held 0.7 s at its end;
  `editor.stroke_shape is none` after each (the owner's real strokes never snap at written size), `editor.shape_reason`
  recorded.
- **`z_shape_4_toggle`**: the panel button, the `shapes` key and the picker chip each toggle; persists across a
  restart (`# root:` a second file); `expect editor.panel_style.shapes is plain` on and off (never `selected`); shots
  upright and sideways; with shapes off a held circle stays freehand.
- **`z_shape_5_zoom`**: pinched in to 2x: a 15 mm (screen) circle snaps; a circle 15 mm on the page but 7 mm on screen
  at 0.5x does not (`TooSmall`).
- **`z_shape_6_link_mode`**: phase 2's strip armed, the lasso held still 1 s: no preview, the lasso stays a lasso.
- **`z_shape_7_last`**: shapes off; a rough box; the `shape-last` key: `editor.stroke_shape` is a rectangle kind,
  `editor.undo_kind is replace`, one `ui`; Undo gives the freehand stroke back (point count); on handwriting:
  'No shape in the last stroke.'.
- **`z_shape_8_link_border`**: a box drawn and snapped around a link's area: strokes +1, `editor.links` unchanged, a
  finger tap inside still follows the link (R1's visible border).
- **`z_shape_9_live`**: as `y_live_ink` starts Live view: a snapped circle leaves the browser model with the snapped
  stroke only (the check that decides 4's `live_reset`).
- `y_all_screens`: the picker with its new row, the panel with 13 buttons (upright, sideways, 0-4 pins), EN and ES,
  both styles, CHECK_FIT.
- Changed on purpose: any raw panel tap below the eraser (6); scenarios that probe `editor.panel_button` heights at 12
  buttons.
- Stay green: every phase 1-3 scenario; `y_live_ink`, `h_live_vs_settled`; every pen scenario (`settings.shapes` is on
  by default, so a scenario that holds the pen still at the end of a closed stroke may now snap: find them by running
  the suite after commit 5, and add `# settings: shapes=0` only where the stroke is not about shapes, saying why).

---------------------------------------------------------------------------------------------------------------

## 12. UI strings (English / Spanish)

| Where | English | Spanish |
|---|---|---|
| toast | Smart shapes on. Hold the pen still at the end of a shape. | Formas inteligentes activadas. Mantén el lápiz quieto al final de una forma. |
| toast | Smart shapes off. | Formas inteligentes desactivadas. |
| picker chip | Smart shapes: On / Smart shapes: Off | Formas inteligentes: activadas / Formas inteligentes: desactivadas |
| picker chip | Hold: Quick / Hold: Normal / Hold: Slow | Espera: Rápida / Espera: Normal / Espera: Lenta |
| toast | Shape hold: Quick (0.4 s) / Shape hold: Normal (0.55 s) / Shape hold: Slow (0.9 s) | Espera de forma: Rápida (0,4 s) / Espera de forma: Normal (0,55 s) / Espera de forma: Lenta (0,9 s) |
| action labels | Smart shapes; Shape hold time; Shape the last stroke | Formas inteligentes; Tiempo de espera de forma; Dar forma al último trazo |
| toast | No shape in the last stroke. | El último trazo no es una forma. |
| tip (bit 5) | Shape made. Undo takes it away; the panel's shapes button turns this off. | Forma hecha. Deshacer la quita; el botón de formas del panel lo desactiva. |

Budgets: toasts about 1700 px at 44 px bold or two lines; chips 520 px at 44 px; action labels as phase 3's rows.
po/es-glossary.md: "forma" (smart shapes) apart from "figura" (finger shapes).

---------------------------------------------------------------------------------------------------------------

## 13. Docs

- **docs/FORMAT.md:** `shapes`, `shape_hold_ms`, `hints` bit 5; a snapped shape is an ordinary stroke (`.qpg`
  unchanged); the `shape:` log lines and the `shape-log` switch.
- **docs/SHORTCUTS.md:** the three actions.
- **docs/DESIGN.md:** the hold, the preview, the panel button's look, the picker row, the refresh table.
- **docs/SIM.md:** section 9. **docs/KINDLE.md / TESTING.md:** the owner checks and the shape-log steps.
- **docs/index-hook/PURE-MODULES.md:** shape merged, its items closed.

---------------------------------------------------------------------------------------------------------------

## 14. Commit plan and the gate

Each message ends with CLOUD.md's attribution lines; each leaves `make rust-check` green.

1. `shape: from claude/rust-index-pure-p4zqq5 (<commit>)` (1.2).
2. `shape: the review items closed; allocations with a fallback; the v1.5 corpus gate` (1.2 items 1-7).
3. `history: ActKind::Replace` (5, `test_history.rs`).
4. `canvas: canvas_slash` (its test).
5. `editor: hold to snap (the dwell anchor, the hold check, the preview, its cancel, one Add at the lift)` (3, 4,
   `z_shape_1`, `_2`, `_5`, `_6`, `_9`; 11's last item). Push.
6. `runner: pen strokes; the owner's strokes never snap` (9, `z_shape_3_writing_never`).
7. `store, actions: shapes, shape_hold_ms, three actions` (6's settings and actions, l10n).
8. `panel, editor: the Shapes button (13 buttons), the picker row` (6, `test_panel.rs`, `z_shape_4_toggle`, raw taps,
   l10n).
9. `editor: shape-last` (5, `z_shape_7_last`, `z_shape_8_link_border`).
10. `editor: the shape-log; hints: the first-snap tip` (7, bit 5).
11. `y_all_screens and l10n-check: the picker row, 13 buttons`; `docs: phase 4` (13). Push.

The gate is PHASE2-BRIEF 16.2's. shape.rs's floats may differ in the last bit between hosts (f64 inside, `fmla`
fuses only on aarch64): its tests assert kinds, reasons and tolerances, never exact coordinates; if one differs on
Linux, report it, never edit it.

---------------------------------------------------------------------------------------------------------------

## 15. Owner acceptance on the Kindle (by hand, numbered)

1. Draw boxes, circles, triangles and lines, and hold the pen still at the end: a clean shape appears in the same pen
   and size; lifting keeps it. Hold, then keep moving: your line comes back and the stroke goes on. One Undo removes a
   shape.
2. The panel's Shapes button, under the eraser (filled when on, slashed outline when off, never black) toggles it;
   the tool picker's row toggles it too and sets the hold time (Quick, Normal, Slow).
3. Write a page of blackletter and a page of ordinary notes with your usual pauses: nothing should snap. If anything
   does, create the empty file `grimoire/shape-log`, write for an evening, and send `grimoire.log`.
4. With shapes off, bind 'Shape the last stroke' to a key or a gesture: it fixes a rough box, and Undo gives your line
   back.
5. Draw a box around a link's words: it is ordinary ink and the link still works (your visible border).
6. Zoomed in, a shape the same size on screen snaps the same way.
7. Live view: the browser shows the snapped shape once you lift.
- Known: arrows, polylines and arcs only if these notes say they passed the corpus gate.

---------------------------------------------------------------------------------------------------------------

## 16. Brief decisions (fold into PLAN-V2 section 19)

1. shape.rs's real API (`shape_recognize`, `shape_params`, `shape_in_burst`, `shape_stroke`) replaces PLAN 10.2's
   assumed one; the editor passes the stroke whole and the recognizer cuts the hold's rest itself.
2. While a preview is up, samples within the 1.5 mm hysteresis join the raw stroke but are not drawn; a cancel draws
   them.
3. `ActKind::Replace = 5` (after phase 2's `Link = 4`), `history_push_replace(h, page, index, shape) -> Rect`.
4. The shape-log writes `qlog!` lines to grimoire.log (no separate file), with `kept` and `moved on` lines.
5. A new runner command `pen strokes FILE.qpg [N] [HOLD_MS]` so the owner's real strokes are an end-to-end negative
   test.
6. `shape-hold` cycles 400 / 550 / 900 ms; other hand-set values show as the nearest.
7. If the browser keeps raw quick ink after a snap, the commit calls `live_reset()`.
8. The main loop wakes at least every 30 ms while an ink stroke is down and shapes are on (only if it does not
   already).
