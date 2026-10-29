# Phase 6 implementation brief: pen-button gestures (Rust)

This brief is self-contained: with it and the repository you can build phase 6. It applies
`docs/index-hook/PLAN-V2.md` (the authority; 15.7 is this phase, with G6.1, G10, 12.3 and 12.4), the owner's ruling D8
in `docs/index-hook/README.md` ("One Gestures sheet for touch and pen gestures (the pen plan's pen.ini folds into
gestures.ini)"), `docs/index-hook/CLOUD.md` and `rust/PORTING.md`. Brief decisions are marked in place and listed in
section 13.

Line numbers are Rust lines at `c2333a9`; phases 2-5 move them, so search by the function name. Phase 3's names are
cited from `PHASE3-BRIEF.md`; verify each against the merged code first (1.1).

The owner's words for this phase, verbatim:

> everything with a toggle or important setting can be assigned in shortcuts or gestures.

> One Gestures sheet for touch and pen gestures

---------------------------------------------------------------------------------------------------------------

## 0. What phase 6 is

One Kindle build the owner installs and tests:

1. **`penbutton.rs`** (new, pure): the pen's side button as gestures: click (`tap`), double, triple, hold and drag, with
   the owner's 350 ms multi-click gap and today's 1.2 s hold.
2. **Today's hard-coded pen button becomes built-ins**: `pen.Button.triple=clear-page` and `pen.Button.hold=clear-page`
   live in gesturemap (G6.1), so the owner can clear or rebind them. `pen_button` (editor.rs:2726) and its fields go.
3. **The PEN source on every action** (PLAN 12.4's last row): any action can take a pen-button gesture; `hold-eraser`
   only in the hold and drag slots, as a momentary eraser.
4. **Pen rows in Settings > Gestures**: rows show pen gestures; the record card accepts the pen button; `hold-eraser`
   appears in the Editing group with the pen as its only kind.
5. **Drag slots**: a bound drag does its action instead of the side button's ink; `pan` and `zoom` values move or zoom
   the page with the pen.
6. **gestures.ini `pen.*` lines live** (D8): the lines phase 1 parsed and carried now act.

### 0.1 Out of scope

| Not in phase 6 | Why |
|---|---|
| A latched finger form of Hold eraser | PLAN 20, question 2 |
| `pen.Eraser.*` slots (the eraser end is an end, not a button: its contact erases) | parsed and carried, never live in v1 (brief decision) |
| `pen.Button2.*` on a pen without a second button | rows hidden, lines carried (2.4) |

Known in this build (owner notes): a single click bound to an action waits 350 ms before it acts (to tell it from a
double or triple click); triple-click now means three clicks each within 350 ms of the last (it was three within
1.2 s).

---------------------------------------------------------------------------------------------------------------

## 1. Branch, base, and what must be true first

- Base: the finished phase 5 branch (`claude/rust-index-p5`) as the owner's session merged it (phase 6 needs phase 3's
  sheet and registry; it follows phase 5 in order). Branch: `claude/rust-index-p6`. Never push to `main`,
  `claude/rust-port` or `claude/beta5-features`.
- Set up the machine with `docs/index-hook/CLOUD.md`.

### 1.1 Names this phase builds on (verify each first)

| Name | Where | What phase 6 needs |
|---|---|---|
| `pen_button(a, ev)`, `btn_down`, `btn_down_at`, `btn_used_for_ink`, `btn_hold_fired`, `clicks`, `nclicks` | editor.rs:2726-2757, 190 | replaced by penbutton |
| the 1200 ms hold in `editor_tick` | editor.rs:5255-5269 | moves into `penbutton_tick` |
| `InType::PenButton`, `InEvent.side_button`, `pen_stroke(st, eraser, side_button)` | input.rs, editor.rs:2280, 2697, 4826 | the button's state; today's side-button ink |
| `settings.side_button` (what the button's ink is, 0..3) | store.rs | an unbound drag keeps it |
| `GestureKey::Pen { button, kind }`, `PenBtn`, `PenKind`, pen slots parsed and carried | gesturemap.rs (PHASE1-BRIEF 7) | live slots |
| `gesturemap_set`, `gesturemap_clear_action`, `gesturemap_format`, `gesturemap_save` | PHASE3-BRIEF 4 | pen lines written |
| `actions.rs` `SRC_PEN`, `ActionKind::Momentary`, the groups | PHASE3-BRIEF 5 | PEN in every mask |
| the Gestures sheet, `BindKind` GESTURES, the record card, `row_text` | PHASE3-BRIEF 7-9 | pen rows and the pen record path |
| `editor_shortcut_at(.., ActionSrc::Pen)`, the page-turn rule (G6.3) | editor.rs | dispatch |
| `hold_eraser` (EditorState) and `SHORTCUT_HOLD_ERASER` | editor.rs | the momentary eraser |
| feature A's capture-blocked flag (`editor_live_style` / `editor_pen_not_ink`) | editor.rs:2684-2725 | a bound drag is not ink |
| the runner's `side on|off` (sim_sdl.rs:1422), `pen hover`, `pen down/move/up` | sim_sdl.rs | scenarios |

### 1.2 Pre-flight

`make rust-check` on the base; record the Linux-only failures. No pure module comes from `claude/rust-index-pure`.

---------------------------------------------------------------------------------------------------------------

## 2. `penbutton.rs` (new, pure, host-tested; PLAN 15.7)

Model it on `pensync.rs`: no App, no I/O, no allocation. `lib.rs` foundations and prelude.

```rust
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PbIn { pub t_ms: u32, pub button: PenBtn, pub down: bool }          // a button press or release
#[derive(Clone, Copy, Debug, Default)]
pub struct PbPen { pub prox: bool, pub touching: bool }                        // the pen's state at that moment
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PbOut { None, Fire(PenBtn, PenKind), DragBegin(PenBtn), DragEnd(PenBtn), HoldBegin(PenBtn), HoldEnd(PenBtn) }
pub struct PbConfig { pub gap_ms: u32 /*350*/, pub press_ms: u32 /*450*/, pub hold_ms: u32 /*1200*/,
                      pub waits: [bool; 3] /* per button: double or triple bound, so a click must wait */ }
pub struct PenButtons { /* per button: down, down_at, used for ink, clicks, last release, hold fired */ }
pub fn penbutton_event(p: &mut PenButtons, c: &PbConfig, e: PbIn, pen: PbPen) -> PbOut;
pub fn penbutton_pen(p: &mut PenButtons, c: &PbConfig, pen: PbPen, t_ms: u32) -> PbOut;  // the nib touched or left
pub fn penbutton_tick(p: &mut PenButtons, c: &PbConfig, now: u32) -> PbOut;             // pending clicks, the hold
pub fn penbutton_probe(p: &PenButtons, key: &str, out: &mut String) -> bool;           // "pen.*"
```

Rules (times by the event's `t_ms`, wrapping u32):

1. **A click**: the button pressed and released within `press_ms` (450, today's) while the nib did not touch. A press
   during which the nib touched is ink or a drag, never a click (today's `btn_used_for_ink`).
2. **Multi-click**: a press within `gap_ms` (350) of the previous click's release adds to the count. The count resolves
   `gap_ms` after the last release (`penbutton_tick`): 1 `Tap`, 2 `Double`, 3 `Triple`; a fourth press restarts at 1.
   When neither `double` nor `triple` is bound for that button (`waits` false), a click fires `Tap` at once at its
   release (no wait). The built-in triple is bound, so with the defaults a click waits.
3. **Hold**: the button down for `hold_ms` (1200, today's) while the pen hovers (in proximity, nib up) fires `Hold` once
   (`HoldBegin`); its release gives `HoldEnd`. A hold is not a click.
4. **Drag**: the nib touches while the button is down: `DragBegin` at the touch, `DragEnd` at the nib's lift (or the
   button's release, whichever first); the stroke between is the drag's (2.2).
5. Proximity lost (pen leave) cancels a pending count and ends a hold or drag.

### 2.1 Today's behaviour, and what changes on purpose

| Today (editor.rs `pen_button`, `editor_tick`) | Phase 6 |
|---|---|
| click = press of at most 450 ms not used for ink | the same |
| three clicks within 1.2 s clear the page | three clicks each within 350 ms of the last: `Triple`, built-in `clear-page` |
| hold 1.2 s (not used for ink, no owner, no sheet) clears the page | `Hold`, built-in `clear-page`, the same conditions at dispatch |
| the button held while writing = the side button's ink (`settings.side_button`) | the same while `drag` is unbound |

Both built-ins dispatch through `editor_shortcut_at(a, SHORTCUT_CLEAR_PAGE, x, y, ActionSrc::Pen)`, whose gates (no
owner, no sheet) match today's conditions. The 350 ms gap is the owner's (PLAN 15.7); list it in the commit message
and the owner notes.

### 2.2 Drag slots

- Unbound (`nothing` or no line): today's meaning, the stroke is the side button's ink.
- Bound to an action: at `DragBegin` the action is dispatched once (`ActionSrc::Pen`, anchored at the nib), and the
  contact draws nothing: the editor gives the contact `Owner::PenDrag` (new, numbered after the last owner), which
  raises feature A's capture-blocked flag (so `/live/pen` shows no quick ink) and swallows its moves.
- `pan`: the contact pans the page where it can scroll (as a two-finger PAN: offsets only, `Fast` every 180 ms, `Ui`
  at the end); nothing where it cannot.
- `zoom`: vertical motion zooms around the contact's start (up zooms in, 1.25x per 10 mm), `Fast` every 180 ms, `Ui` at
  the end; clamped to `zoom_min..zoom_max`.
- `hold-eraser` (MOMENTARY, 2.3).

### 2.3 Hold eraser from the pen button (PLAN 12.3)

`hold-eraser` is assignable to the pen only in the `hold` and `drag` slots (they have a release). Either one makes the
button momentary: from the button's press (no 1.2 s wait: brief decision, a momentary eraser must start at once) until
its release, `s.hold_eraser` is on and every contact erases, exactly as a key bound to `hold-eraser` does today; the
release turns it off. With `hold-eraser` bound, that button's clicks and multi-clicks still work (a click releases
within 450 ms with nothing erased).

### 2.4 Buttons

- `Button` (BTN_STYLUS, `InEvent.side_button`): live.
- `Button2`: live only when the pen node reports BTN_STYLUS2 (EVIOCGBIT at `input_open`, a new `InputCaps.pen_button2`
  and a new `InEvent.side_button2`, logged on a NEW open-time line `input: pen buttons %d`); otherwise its rows are
  hidden and its lines carried. The runner reports it (so scenarios can test it) behind `GRIMOIRE_SIM_PEN_BUTTON2=1`.
- `Eraser`: carried, never live (0.1).

---------------------------------------------------------------------------------------------------------------

## 3. gesturemap and the registry

- Pen slots become live: `gesturemap_action(m, GestureKey::Pen { .. })`; built-ins `pen.Button.triple=clear-page` and
  `pen.Button.hold=clear-page` (built-in duplicates stay legal, PLAN 12.6); `pen.Button.double=links` is not a default
  (D6).
- Values: an action whose sources include PEN, `nothing`, or `pan` / `zoom` in a `drag` slot only. A `pan` or `zoom`
  value elsewhere, or `hold-eraser` outside `hold` and `drag`, is refused and counted.
- `gesturemap_key_for(m, action, ActionSrc::Pen)` returns the first pen slot in the order Button.tap, .double,
  .triple, .hold, .drag, then Button2's.
- `gesturemap_clear_action` clears pen slots too; a cleared built-in is written `=nothing`.
- `PbConfig.waits[button]` comes from the map: double or triple bound (not `nothing`) for that button; recomputed after
  every load and write.
- actions.rs: `SRC_PEN` joins every action's sources; `hold-eraser` becomes KEY | PAD | PEN (the slot rule above is
  gesturemap's); `shortcuts` and `bluetooth-controller` TOUCH | SHAPE | PEN. Two pseudo-actions for the drag values are
  not actions (brief decision): the Gestures sheet's Pages group gains two rows, 'Pan with the pen' and 'Zoom with the
  pen', which record only a pen-button drag and write `pen.Button.drag=pan` / `=zoom`.

---------------------------------------------------------------------------------------------------------------

## 4. The editor

- `pen_button` and its fields go; `EditorState.pb: PenButtons`. Every `InType::PenButton` (editor.rs:4826) and every
  pen proximity, touch and lift feed `penbutton_event` / `penbutton_pen`; `editor_tick` calls `penbutton_tick` (only
  when `a.input_drained`, so a queued release is never read as a hold).
- `PbOut::Fire(button, kind)`: `gesturemap_action`; dispatch with `editor_shortcut_at(a, action, nib x, y,
  ActionSrc::Pen)` (the last pen point; the screen centre if none). Next-page from the pen never adds a page (G6.3).
  `pen.gesture` records `Button.double` and the like.
- `HoldBegin`/`HoldEnd` with `hold-eraser` bound: momentary (2.3). With another action bound: dispatched at
  `HoldBegin`.
- `DragBegin`/`DragEnd`: 2.2.
- While a sheet or menu is in front, a pen-button gesture acts as a key bound to the same action would (menus close or
  swap; the rest are refused), as today's hold is refused over sheets.
- `editor_live_style` / `editor_pen_not_ink`: `Owner::PenDrag` is not ink (raised at PenDown).

---------------------------------------------------------------------------------------------------------------

## 5. Settings > Gestures with the pen (PLAN 6.14, G9)

- Rows: a pen binding reads 'Undo: pen button double-click'; with a touch or shape binding too, the row shows the first
  in the order touch, shape, pen, plus ' +1' when another exists (brief decision: one line per row, sideways too).
- `hold-eraser` appears in the Editing group with the pen as its only kind.
- The record card body becomes 'Draw a shape with one finger, swipe or tap with two fingers, or use the pen's button.'
  (a new msgid; the phase 3 one is retired). The card listens to `PenButton` events and pen proximity while it is up
  and feeds its own `PenButtons` (the same `PbConfig`): a recognized pen gesture shows its name at 64 px ('pen button
  double-click') with Use / Try again / Cancel, and the taken-gesture line if another action holds it. A pen-button
  gesture on a row that cannot take it ('Hold eraser' and a click) says 'Hold eraser needs the button held: hold it or
  drag with it.'.
- A pen tap on the card's buttons is a tap, never ink (the Library: not the page).
- The Settings sheet's pen-button row keeps only the side-button cycler (what the button's ink is); the clear-page
  wording it had for the triple click and hold moves to the Gestures sheet's rows (check the Settings strings that
  mention them and retire those that no longer apply).

---------------------------------------------------------------------------------------------------------------

## 6. E-ink

| Event | Waveform |
|---|---|
| a fired pen gesture | the bound action's own |
| a bound drag | nothing on the page; `pan` / `zoom` Fast every 180 ms, Ui at the end |
| momentary eraser | as erasing with a held key |
| record card line | Ui of the line |

No GL16 or toast while the pen is down: a toast an action makes at `DragBegin` waits for the pen's lift (the existing
toast rule).

---------------------------------------------------------------------------------------------------------------

## 7. The runner and docs/SIM.md

- Existing: `side on|off` (a press or release at the pen's place), `pen hover`, `pen leave`, `pen down/move/up`,
  `wait`.
- New helper (brief decision, for readable scenarios): `side click N [GAP_MS] [PRESS_MS]`: N clicks, each pressed
  PRESS_MS (default 80) with GAP_MS (default 150) between a release and the next press, while the pen hovers where it
  is, stamped on the runner's clock.
- Hook `GRIMOIRE_SIM_PEN_BUTTON2=1` (scenarios rule unset list, `SIM_HOOKS`).
- Probes: `pen.gesture` (the last recognized: `Button.tap`, `Button.double`, `Button.triple`, `Button.hold`,
  `Button.drag`, empty before any), `pen.clicks` (the pending count), `gestures.ACTION` lists pen keys
  (`pen.Button.double`), `editor.hold_eraser` (`0`/`1`).
- SIM.md: the helper, the hook, the probes, and the timing note (a click waits 350 ms when double or triple is bound).
  Selftest: `pass-rust.qs` `expect pen.gesture is`; `fail-rust.qs` `expect pen.gesture is Button.double`.

---------------------------------------------------------------------------------------------------------------

## 8. Unit tests

- `rust/tests/test_penbutton.rs`: the gap at 349 and 351 ms; the press at 450 and 451 ms; the hold at 1199 and 1200 ms
  while hovering, none while touching; a drag is not a click; ink with the button held and `drag` unbound keeps
  today's meaning (no output but `DragBegin/End`); `waits` false fires `Tap` at the release; a fourth press restarts;
  pen leave cancels a pending count; wrapping u32 times.
- `test_gesturemap.rs`: pen slots read, written, cleared to `=nothing`; built-in duplicates legal; `pan`/`zoom` only in
  drag slots; `hold-eraser` only in hold and drag; Button2 and Eraser lines carried.
- `test_actions.rs`: every action has PEN; `hold-eraser` is KEY | PAD | PEN.
- `test_i18n.rs`: the pen gesture names in a row with a thumbnail column; the new card body sideways.

---------------------------------------------------------------------------------------------------------------

## 9. Scenarios

Every new file starts with `# rust-only`.

- **`v_gestures_4_pen`**: Settings > Gestures, the Links row, `side click 2`: the card shows 'pen button
  double-click'; Use: `gestures.links is pen.Button.double`, gestures.ini has the line; on the page `side click 2`,
  `wait 400`: `editor.bar is open`; `side click 3` still clears the page (`editor.strokes is 0`); a 1.3 s hover hold
  (`side on`, `wait 1300`, `side off`) still clears it.
- **`v_gestures_4b_clear`**: hold the Clear page row to clear it: `pen.Button.triple=nothing` and `pen.Button.hold=nothing`
  are written; three clicks no longer clear the page.
- **`v_gestures_4c_hold_eraser`**: bind the hold slot to Hold eraser in the sheet: `side on`, a `pen line` across ink:
  strokes erased, `editor.hold_eraser is 1`; `side off`: `0`, a `pen line` draws ink.
- **`v_gestures_4d_drag`**: `pen.Button.drag=pages` in gestures.ini: `side on`, `pen down`, `pen move`, `pen up`: the
  Pages view opens, `editor.strokes` unchanged, nothing on `/live/pen` (as `y_live_ink` checks); `pen.Button.drag=pan`
  zoomed in: the view moves; unbound: today's side-button ink.
- **`v_gestures_4e_single`**: `pen.Button.tap=undo` with the triple built-in: one click undoes after 350 ms
  (`wait 400`); with `pen.Button.triple=nothing` and no double, it undoes at once.
- `y_all_screens`: the Gestures sheet's rows with pen gestures, the Editing group with Hold eraser, the card with the pen
  wording; EN and ES, both styles, upright and sideways, CHECK_FIT.
- Changed on purpose: any scenario that clicks three times with gaps over 350 ms to clear a page (grep for `side on`
  sequences); keep the clear and tighten the gaps, saying why.
- Stay green: `f_batch2`, `z_tool_b_button_over_sheets`, every side-button ink scenario, every earlier phase's.

---------------------------------------------------------------------------------------------------------------

## 10. UI strings (English / Spanish)

| Where | English | Spanish |
|---|---|---|
| gesture names | pen button click / pen button double-click / pen button triple-click / pen button hold / pen button drag | clic del botón del lápiz / doble clic del botón del lápiz / triple clic del botón del lápiz / botón del lápiz mantenido / arrastre con el botón del lápiz |
| gesture names (Button2) | second pen button click ... drag (the same five) | clic del segundo botón del lápiz ... (the same five) |
| sheet rows | Pan with the pen / Zoom with the pen | Mover con el lápiz / Ampliar con el lápiz |
| row suffix | +%d | +%d |
| card body | Draw a shape with one finger, swipe or tap with two fingers, or use the pen's button. | Dibuja una figura con un dedo, desliza o toca con dos dedos, o usa el botón del lápiz. |
| card line | Hold eraser needs the button held: hold it or drag with it. | Borrar mientras se pulsa necesita el botón pulsado: mantenlo o arrastra con él. |

Retired: phase 3's card body ('Draw a shape with one finger, or swipe or tap with two fingers.'); leave its po entry to
`make l10n`. Budgets as phase 3's (rows about 1000 px at 44 px with the thumbnail column).

---------------------------------------------------------------------------------------------------------------

## 11. Docs, commit plan, the gate

**Docs.** FORMAT.md: `pen.*` lines live (slots, values, `pan`/`zoom`, the hold-eraser rule, Button2 and Eraser
carried); SHORTCUTS.md: the PEN source, the pen gestures, the 350 ms wait, the momentary eraser; DESIGN.md: the pen rows
and the card; SIM.md section 7; KINDLE.md and TESTING.md the owner checks.

**Commits** (each ends with CLOUD.md's attribution lines and leaves `make rust-check` green):

1. `penbutton.rs: click, multi-click, hold and drag recognition` (2, `test_penbutton.rs`).
2. `gesturemap, actions: pen slots live, the PEN source, the drag values` (3, `test_gesturemap.rs`, `test_actions.rs`).
3. `editor: the pen button through penbutton (built-in triple and hold), drag slots, the momentary eraser` (4, 2.2,
   2.3; runner `side click`; `v_gestures_4d_drag`, `v_gestures_4e_single`; 9's changed scenarios). Push.
4. `input_evdev: a second pen button where the pen has one` (2.4).
5. `library: pen rows in Settings > Gestures, the pen in the record card` (5, `v_gestures_4`, `_4b`, `_4c`, l10n).
6. `y_all_screens and l10n-check: pen rows`; `docs: phase 6`. Push.

**The gate** is PHASE2-BRIEF 16.2's.

---------------------------------------------------------------------------------------------------------------

## 12. Owner acceptance on the Kindle (by hand, numbered)

1. In Settings > Gestures, record a pen double-click for Links: on the page it opens the bar after the short pause.
2. A triple-click (quick) still clears the page; holding the button 1.2 s with the pen hovering still clears it.
3. Hold the Clear page row to clear its gestures: the triple-click no longer clears the page.
4. Bind the pen button's hold to Hold eraser: the pen erases while you hold the button and writes again when you let go.
5. Bind the pen button's drag to Pages (or pan with the pen when zoomed in): dragging with the button held does that and
   leaves no ink; Live view shows no ink for it.
6. Writing with the button held and nothing bound to its drag still gives your side-button ink.
7. Every new screen fits in English and Spanish, upright and sideways.

Known in this build: a single click bound to an action waits a third of a second; a triple click needs quick clicks.

---------------------------------------------------------------------------------------------------------------

## 13. Brief decisions (fold into PLAN-V2 section 19)

1. `penbutton.rs`'s API and rules of section 2: click at most 450 ms, the 350 ms gap between a release and the next
   press, the count resolved 350 ms after the last release, and no wait when neither double nor triple is bound.
2. The momentary eraser starts at the button's press in either the hold or the drag slot (no 1.2 s wait).
3. A bound drag dispatches its action once at the nib's touch and draws nothing (`Owner::PenDrag`, capture-blocked);
   `pan` and `zoom` are drag values, not actions, set by two dedicated rows in the Pages group.
4. `pen.Eraser.*` is never live in v1; `pen.Button2.*` is live only on a pen that reports BTN_STYLUS2.
5. A row with several gestures shows the first (touch, shape, pen) plus ' +N'.
6. A runner helper `side click N [GAP_MS] [PRESS_MS]` and the hook `GRIMOIRE_SIM_PEN_BUTTON2`.
