# Phase 3 implementation brief: Settings > Gestures, recorded finger shapes, every toggle assignable, forgiving two-finger gestures (Rust)

This brief is self-contained: with it and the repository you can build phase 3. It applies
`docs/index-hook/PLAN-V2.md` (the authority; section numbers such as G8 or 12.4 are its sections), the owner's rulings
in `docs/index-hook/README.md` (D4, D5, D8 and "everything with a toggle or important setting can be assigned" are the
ones phase 3 serves), `docs/index-hook/CLOUD.md` and the contract `rust/PORTING.md`. Its style follows
`PHASE1-BRIEF.md` and `PHASE2-BRIEF.md`. Where this brief is more precise than the plan, or departs from it because the
code or the device says otherwise, it says so in place ("brief decision"); section 19 lists them for PLAN-V2 section 19.
A marked brief decision wins for phase 3 code; anything else follows the plan. Raise a disagreement with the owner's
session rather than guessing.

Line numbers are Rust lines at `c2333a9` (2.0.0-beta.5: phase 1, the clock corner and Live view merged). Phase 2 moves
them; search by the function name. Phase 2's names are cited from `PHASE2-BRIEF.md`, where they were designed: verify
each against the finished phase 2 code first (section 1.1).

The owner's words for this phase, verbatim:

> New menu at the bottom of "Settings" in between "Controller" and "Updates" called "Gestures" it will have the
> ability to assign anything the controller category can, and you would record a gesture to get whatever you want

> i wanted more of a gesture system where a user can record a basic shape or pattern and use them anywhere

> everything with a toggle or important setting can be assigned in shortcuts or gestures.

And on the phase 1 build (2026-10-28), about two-finger page turns and pinch zoom:

> I have to find the right pressure and speed

---------------------------------------------------------------------------------------------------------------

## 0. What phase 3 is

Phase 3 is one Kindle build that the owner installs and tests. It contains:

1. **Forgiving two-finger gestures** (section 3): every touchgest threshold in one tuning table, four rule changes that
   the device log points at (the swipe's spread rule, the last lift, the pinch's classification and its lost
   finger), looser defaults, a measurement log line, a trace replay tool (`tgstudy`) and a threshold study run on the
   owner's touch traces. The thresholds are frozen from that study (PLAN 15.4's "frozen from the phase 1 traces").
2. **The pen-hover rule** (G3.1 rule 6, section 3.8): a two-finger gesture works while the pen hovers far away.
3. **Three fingers where the panel has them** (section 3.9). The owner's panel reports two contacts, so on the owner's
   Kindle the three-finger rows stay hidden.
4. **The action registry** `actions.rs` (PLAN 12.1) and the **remaining actions** (PLAN 12.4, sections 5 and 6), so
   every toggle and important setting is an action.
5. **Bindings by source** (PLAN 12.6, section 7): PAD, TOUCH, SHAPE give an action at most one binding each;
   `SHORTCUT_MAX` 192; the Buttons sheet parametrized by `BindKind`; seven groups.
6. **Settings > Gestures** (G9, section 8): the Settings bottom row KOReader | Controller | Gestures | Updates | Done,
   the sheet, rows with thumbnails, hold to clear, Reset.
7. **Recording** (G9.2, G9.3, section 9): touchgest's record mode and the record card, for two-finger patterns,
   one-finger swipes and finger shapes.
8. **gestures.ini written** (G10, section 4): atomic, `=nothing`, `touch.1.*`, `shape.<action>` lines.
9. **Finger shapes on the page** (G8, G11, section 10): capture, the dotted trail, matching, firing, the fading
   reject, through `fingershape.rs` from the pure branch.
10. **`hints` bit 7** (the one-time reject tip) and settings.ini `rotation_lock`.

### 0.1 Explicitly out of scope (later phases; do not build them now)

| Not in phase 3 | Phase |
|---|---|
| Smart shapes, `shape.rs`, the Shapes panel button, `shapes`, `shape-hold`, `shape-last` | 4 |
| The Index map, `graph.rs`, `index-map` | 5 |
| Pen-button gestures, `penbutton.rs`, the PEN source, pen rows in the sheet, `hold-eraser` in the Gestures sheet | 6 |
| A latched finger form of Hold eraser (PLAN 20, question 2) | not planned |
| One-turn finger shapes (V, check mark, L, U, C): refused at recording (PLAN 20, question 1) | not planned |
| Finger shapes in the Pages view, the map, the Library (except the record card) | not planned in v1 |
| Hiding the trail from Live view's screen stream (PLAN 20, question 3) | not planned |

Known in this build (say so in the owner notes):
- three-finger rows do not appear on the owner's Kindle: its panel reports two contacts (`input: touch slots 0..1` in
  the beta.5 log);
- pen-button rows arrive in phase 6; the pen button keeps today's triple click and hold;
- V, a check mark, L, U and C are refused as too easy to make by accident;
- a finger shape is matched by how it looks: a square drawn when only a circle is recorded may fire the circle's
  action (PLAN 18).

---------------------------------------------------------------------------------------------------------------

## 1. Branch, base, and what must be true first

- Base: the finished phase 2 branch (`claude/rust-index-p2`, all of PHASE2-BRIEF.md's commits 0-16) as the owner's
  session merged it. Branch: `claude/rust-index-p3` (or the name your task gives). Never push to `main`,
  `claude/rust-port` or `claude/beta5-features`.
- Set up the machine with `docs/index-hook/CLOUD.md`.

### 1.1 Names this phase builds on (verify each before writing code)

Find each with `grep -n`; note any difference; adapt only the call sites named here. If one is missing, stop and ask
the owner's session.

| Name | Where (c2333a9 unless said) | What phase 3 needs from it |
|---|---|---|
| `TouchGest`, `TgView { .., record, keyboard_open }`, `TgPen`, `TgOut`, `TgOutcome`, `TgReject`, `G_2_*`, `G_3_*`, `touchgest_init/frame/cancel/tick/set_view/busy/probe`, the log and trace formatters | touchgest.rs:49-1346 | the tuning table, record mode, the hover rule, three fingers, the measure line |
| the rules' constants `LAND_MS` .. `FAR_MM` | touchgest.rs:174-215 | become `TgTune` fields (3.2) |
| `GestureMap`, `gesturemap_parse/load/action/key_for/mirror/key_of_id/key_name`, the carried lines, the `hold-eraser` refusal (e0579ca) | gesturemap.rs | writes, shape lines, `touch.1.*`, registry-based validation |
| `ActionSrc { Key, Pad, Touch, Shape, Pen }` | shortcuts.rs:64 | `Shape` used from now |
| `SHORTCUT_*` ids up to `SHORTCUT_LAST = 53` (`SHORTCUT_STATUS = 47`, `SHORTCUT_STATUS_CORNER = 53`), `K_ACTIONS`, `ShortcutBinding { .., gamepad }`, `SHORTCUT_MAX = 64`, `ini_read_bounded` | shortcuts.rs | new ids after 53, sources, limits |
| `editor_shortcut_at(a, action, x, y, src)`, `editor_shortcut`, `editor_gesture`, `front_menu`, `pinch`, `touch_context` | editor.rs:4044, 4036, 4765, 3912, 4536, 1080 | Shape dispatch; the remaining actions |
| `Owner::Link` session: `link_down_at`, `link_held`, `link_press`, `LINK_HOLD_MS`, the hold in `editor_tick` | editor.rs:22, 5239 | the finger capture rides on it |
| phase 2's hold: the link card opens where `link_hold` was | PHASE2-BRIEF 8.2 | the capture is killed (`fs_kill(.., Held)`) when it opens |
| the corner: `status_square(a)`, `status_hit(a, x, y)`, `status_tapped()`, editor's `corner_tap(cancelled)` | status.rs:380, 388, 431; editor.rs:4751 | PHASE2-BRIEF calls the square `corner_zone`; a shape may start there |
| `editor_page_in_front(a)` | editor.rs:4724 | the corner is live only when the page is in front (no sheet, menu, Pages view or keyboard) |
| `canvas_dash_poly(c, pts, thick, on, off, halo, phase) -> i32` | canvas.rs (PHASE2-BRIEF 6.4) | the trail's sibling `canvas_dot_poly` (10.3) |
| `editor_pen_not_ink` / `editor_live_style` as phase 2 left them | editor.rs:2684, 2690 | unchanged: a finger shape never involves the pen |
| `Modal` (16 = Updates), `LearnRow`, `K_EDITING`, `K_TOOLS`, `K_PAGES`, `K_INDEX`, `K_MENUS`, `K_GROUPS`, `N_GROUPS = 5`, `action_label`, `ROW_HOLD_MS`, `reload_shortcuts`, the Settings bottom row | library.rs:8, 727-842, 975, 1460-1477 | moved into the registry; `Modal::Gestures = 17` |
| `hover_learn`, `raw_hover`, `hover_from_raw`, the `hover_fn` / `sample_fn` gate | input_evdev.rs:887, 974, 1005, 1094 | the hover rule |
| `InputCaps.touch_slots` (the runner says 10) | input.rs:166, sim_sdl.rs:1785 | three fingers |

### 1.2 fingershape.rs from the pure branch: merge it, then finish it

The pure module is on `claude/rust-index-pure-p4zqq5` (7cbcf0f at this writing). Read its
`docs/index-hook/PURE-MODULES.md` first: if a newer commit there says the review rounds are finished, take that commit
and its final API; otherwise take 7cbcf0f and finish the open items below yourself.

**Bringing it in (commit 1).** Take only fingershape; shape.rs and graph.rs come in phases 4 and 5:

```
git checkout <pure commit> -- rust/src/fingershape.rs rust/tests/test_fingershape.rs rust/tests/fingershape_synth
```

then add by hand, as the pure branch's `lib.rs` does, `pub mod fingershape;` in the foundations beside `pensync` and
`pub use crate::fingershape::*;` in the prelude. The commit message names the source commit. Then move the synthetic
corpus where the runner can reach it: `rust/tests/fingershape_synth/mod.rs` becomes `rust/src/fingershape_synth.rs`
(`pub mod fingershape_synth;`, pure, compiled always; only the runner's `shape` command and the tests call it, so the
Kindle binary's linker drops it), and `test_fingershape.rs` uses it through the prelude instead of `mod
fingershape_synth;` (brief decision: PLAN G14's `cfg(any(test, feature = "sim"))` cannot reach integration tests run
without `sim`).

**The API as it really is** (PLAN G14 was written before the module existed; use these names):

| PLAN G14 (assumed) | fingershape.rs (real) |
|---|---|
| `FsCapture` | `FsStroke` (about 2 KiB, fixed size; one in `EditorState`, one in the record card) |
| `fs_begin(c, x, y, t)` | `fs_begin(s, x: i32, y: i32, t: u32, px_per_100mm: i32)`: logical px rounded; `px_per_100mm = (a.dpi / 25.4 * 100.0).round() as i32` (1181 at 300 dpi) |
| `fs_add(c, x, y, t) -> FsState` | `fs_add(s, x, y, t, broad: bool) -> FsState` (`broad` = the event's `TOUCH_F_BROAD`) |
| `fs_tick`, `fs_abort`, `fs_state` | `fs_tick(s, now) -> FsState`, `fs_abort(s) -> FsLift`, `s.state`; plus `fs_kill(s, FsReason::Held)` when the hold fires |
| `fs_finish(c, &mut cand) -> FsGate` | `fs_finish(s, t_lift, FsMode::Runtime \| Record, &mut FsSample) -> FsLift { Nothing, Shape, Swipe(FsSwipe), Refused(FsReason) }` |
| `fs_trail_points(c, from, out)` | `fs_points(s) -> &[FsPt]` (the trail keeps its own "drawn up to" index) and `fs_box(s) -> Rect` (the dispatch anchor) |
| `FsShape`, `FsBank`, `fs_prepare(bank, action, s)` | `FsSample` (32 points as 64 bytes), `FsGesture` (2-3 samples plus derived clouds, spread, `thr`, `ratio`), `fs_prepare(&mut g, &samples)`; the bank is gesturemap's array of `(action, FsGesture)` |
| `fs_match(cand, bank, pump) -> FsMatch { outcome, d1, thr, d2 }` | `fs_match(&sample, &gestures[..n], pump) -> FsMatch { fired: Option<usize>, why: FsReason, best, d1, thr, d2, ratio, g_ratio }` (index into the slice) |
| `fs_record_begin/step` (resumable) | `fs_record_room(n)`, `fs_record_first(&s, pump)` (sample 1), `fs_record_consistent(&s, &earlier)`, `fs_prepare`, `fs_record_separation(&new, &others) -> FsSeparation { Clear, SameAs(i), TooClose(i) }`, `fs_record_bank(&g, pump)`; `fs_two_strokes(prev, prev_lift, prev_lift_ms, land_ms)` for "One stroke, without lifting." |
| `fs_line_parse`, `fs_line_format` | `fs_decode(v, &mut [FsSample; 3]) -> Result<usize, FsCodec { Malformed, Newer }>`, `fs_encode(&samples, &mut [u8; FS_VALUE_MAX]) -> Result<&str, FsCodec>` |
| `fs_draw_trail`, `fs_draw_thumb` | not in the module, by design: ours (10.3, 8.2) |
| `fs_synth` | `fingershape_synth` (moved as above) |
| `FS_GATES` | `FS_GATES` (a static) and `fs_gates(FsMode)`; `FS_SHAPES_MAX = 32` |

The slip bank is pumped (`pump` every `FS_BANK_PUMP` paths), not resumable: the app draws the "Checking..." line,
flushes it, then calls `fs_record_bank` with `work_pump` as the pump. Input keeps flowing into the queue; a tap on
Cancel during the 1-2 s check is handled after it (brief decision; PLAN G9.3's per-tick budget is not needed).

**Open review items to close (commit 2, each with its test in `test_fingershape.rs`):**

1. *The slip bank refuses at T_g + 10 (`FS_BANK_PAD`) where the design says T_g.* Run the corpus gates at pad 0 and at
   pad 10. If pad 0 meets the design's slip budget (0 of 12,000 swipes, 0 of 12,000 thumb arcs, at most 2 of 12,000
   single-turn slips firing; 0 wrong-action fires in 6,200), set the pad to 0 (the design). Otherwise keep 10 and write
   the numbers into PURE-MODULES.md. Either way the commit message gives both runs' numbers.
2. *Recording consistency lets the spread grow past 60.* `fs_record_consistent` accepts a sample near ANY earlier one,
   so samples 1 and 3 can be 120 apart. Make it ALL: a later sample must lie within `FS_CONSIST_MAX` of every earlier
   sample. Test: three samples where 3 is within 60 of 2 but not of 1 is `Inconsistent`.
3. *The corpus slip gates are weaker than the design's.* Set the corpus test's gates to the design's numbers (item 1's
   budget, and acceptance of at least 92% in use and 97% in recording under the harsh variation).
4. Update `docs/index-hook/PURE-MODULES.md`'s fingershape row: merged in phase 3, items closed, the pad chosen.

### 1.3 The touch traces (the owner's session asks; no device access from here)

Section 3's study needs one trace session on the owner's Kindle. The installed beta.5 build already writes the trace
(phase 1, docs/KINDLE.md "touch trace"), so the owner's session can ask for it before this phase starts, with the
script of section 3.7. The trace and log arrive as files from the owner's session; put the reviewed trace in
`tests/fixtures/touch/` (3.7). If none has arrived when commit 7 is reached, commit 7 ships the provisional defaults of
section 3.4 as they are and says so in its message and the owner notes; `tgstudy` stays for the next round.

### 1.4 Pre-flight

- `make rust-check` on the base before any change; record which checks fail on this Linux machine (section 17.2).

---------------------------------------------------------------------------------------------------------------

## 2. Hard rules that apply to every commit

PHASE2-BRIEF.md section 2's rules 1 to 11 apply as written (no em dashes; English and Spanish in the same commit with
`// TRANSLATORS:` budgets; `.qpg` unchanged; no MXCFB wait, no new display call; never block ~100 ms; no GL16 or toast
while the pen is down; byte-exact formats and logs; atomic writes; wrapping u32 time; short borrows; `try_reserve_exact`
with a fallback). Phase 3 adds:

12. fingershape.rs is integer only; nothing in the app converts its scores to floats. Coordinates go in as logical px
    rounded with `round_ties_even` (as touchgest's `ipx`).
13. A finger shape is never ink: its trail goes into `a.screen` only, never `page_canvas`, a stroke, the `.qpg`, the
    PDF, a thumbnail, the sleep image or `/live/pen`. It never uses the live ink style or pen mode.
14. The pen always wins: pen proximity or contact aborts a finger capture at once (the `TOUCH_F_CANCEL` up already
    arrives); the trail rect is restored with `RefreshMode::Fast` and joins the writing settle, never a GL16 while the
    pen is down.
15. The clock corner: a finger tap there stays the clock's (it never leaves fingershape's Pending, so no trail and no
    shape), and a shape that starts there is still a shape (the lift is moved, so `status_tapped()` is false).
16. The capture-blocked flag (feature A): pen taps on the Gestures sheet and the record card are not ink. They are in
    the Library, which feature A already treats as not-the-page; verify with `y_live_ink`'s pattern that a pen tap on
    the record card adds nothing to `/live/pen`.
17. gestures.ini is written whole and atomically (`write_file_atomic`), never when it did not read, and the in-memory
    map changes only after the write succeeded.
18. The trace and the logs never carry pen coordinates; the touch log writes nothing while input is not grabbed (the
    PIN).
19. touchgest stays pure: no App, no I/O, no allocation, O(1) per frame. The tuning table is data passed in.

---------------------------------------------------------------------------------------------------------------

## 3. Forgiving two-finger gestures: the tuning table, the study, looser defaults

### 3.1 What the beta.5 device log says (2026-10-28, the owner's phase 1 test)

The owner finds two-finger page turns and pinch zoom unforgiving ("I have to find the right pressure and speed"). The
log of that session (the owner's evidence folder `beta5-c2333a9-2026-10-28`, `grimoire-test.log`) has 111
multi-contact sessions:

| Outcome | Sessions | Multi-contact frames (median) | Duration (median, max) |
|---|---|---|---|
| fired 2.up / 2.down / 2.left / 2.right / 2.tap | 32 / 9 / 10 / 2 / 2 | 13-15 | 142-175 ms, 313 |
| rejected spread | 31 | 15 | 177 ms, 878 |
| rejected slow | 7 | 7 | 75 ms |
| rejected short | 4 | 6 | 62 ms |
| rejected straight / off | 1 / 1 | | |
| zoom | 6 | 16 | 188 ms |
| palm / cancelled | 3 / 3 | | |

Also: `input: touch slots 0..1` (the panel tracks two contacts); landing gaps 0-88 ms (`LAND_MS` 150 is never the
problem); lift gaps 0-25 ms; single fingertips report TOUCH_MAJOR median 107, 90th percentile 345, up to 642 on an axis
that claims 0..255 (so TOUCH_MAJOR may not separate a palm); pressure is reported (51-131).

What it means, rule by rule (touchgest.rs at c2333a9):

- **spread** (`swipe_lift`, the spread changed more than `SPREAD_MAX` 25% of the landing spread): the rejected
  sessions last as long and move as many frames as the fired ones, so they are real swipes. Two fingertips about 20 mm
  apart that drift 5 mm closer in a quick flick fail it. It is also where a misread pinch ends up: with nothing
  scrolling, a pinch whose midpoint travels more than its spread changes is classified a SWIPE candidate at the commit
  (ZOOM needs the spread change >= 0.6 x the travel), nothing zooms, and the lift rejects it as `spread`.
- **slow** (7, median 75 ms of two contacts): one finger lifted after about 75 ms, before the commit; the session went
  to `TapWait`; the other finger stayed down (and probably kept swiping) past `TAP_LIFT_MS`, so the tap timed out.
  A light finger that leaves the glass early: the "pressure" the owner feels.
- **short** (4, median 62 ms): the same early lift, with the first finger already more than 5 mm moved, or a swipe
  whose first lift came before 20 mm.
- **pinch zoom**: besides the misread pinch above, sessions that land in the 1 s pen window never ZOOM or PAN (rule 3's
  last item), so a pinch straight after writing does nothing; and a committed ZOOM ends at the first lift, so a finger
  that loses contact for a frame ends the zoom (no evidence either way in this log: no session re-landed a finger; the
  study checks it).

### 3.2 The tuning table (`touchgest.rs`)

Every number in touchgest.rs:174-215 becomes a field of one struct, read by the recognizer, record mode and the replay
tool alike (G9.2: "one table; a unit test asserts it"):

```rust
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TgTune {
    // landing and the pen (ms, mm)
    pub land_ms: u32, pub palm_ms: u32, pub pen_window_ms: u32, pub pen_gone_ms: u32, pub far_mm: f32,
    pub rest_ms: u32, pub rest_mm: f32,
    // the commit and its class
    pub commit_spread: f32, pub commit_px: f32, pub zoom_ratio: f32,
    pub zoom_opposite_mm: f32,   // NEW (3.3 c): both fingers moved this far in opposite ways: a zoom. 0 = off
    pub zoom_upgrade: f32,       // NEW (3.3 d): a swipe or pan whose spread changed this much becomes a zoom. 0 = off
    pub zoom_after_pen: bool,    // NEW (3.3 f): a zoom or pan may start in the pen window under the far rule
    pub grace_ms: u32,           // NEW (3.3 e): a zoom's or pan's lost finger may land again within this. 0 = off
    pub regrab_mm: f32,          // NEW (3.3 e): ...this close to where it lifted
    // the swipe
    pub tan_axis: f32, pub swipe_mm: f32, pub finger_mm: f32, pub cos_min: f32, pub finger_ratio: f32,
    pub spread_max: f32, pub swipe_ms: u32,
    pub follow_ms: u32,          // NEW (3.3 b): after the first lift the other finger may finish a swipe. 0 = off
    // the tap
    pub tap_lift_ms: u32, pub tap_ms: u32, pub tap_mm: f32,
    // size guards
    pub apart_min_mm: f32, pub apart_max_mm: f32, pub palm_major: i32,
    // the hover rule (3.8)
    pub hover_far_mm: f32, pub hover_age_ms: u32,   // 0 = the rule off
}
pub const TG_TUNE_V1: TgTune = TgTune { /* c2333a9's numbers exactly, every NEW rule off */ };
pub const TG_TUNE: TgTune = TgTune { /* section 3.4's defaults, then the study's */ };
pub fn touchgest_set_tune(g: &mut TouchGest, t: &TgTune);   // converts mm to px once, as touchgest_init does
pub fn touchgest_tune(g: &TouchGest) -> TgTune;
```

- `touchgest_init` applies `TG_TUNE`. `touchgest_set_palm_major` stays as a wrapper that sets `palm_major`.
- `Thr` (touchgest.rs:300) gains the new px fields and is rebuilt by `touchgest_set_tune`.
- With `TG_TUNE_V1`, touchgest behaves exactly as at c2333a9: every existing case in `test_touchgest.rs` runs under
  `TG_TUNE_V1` (a helper at the top of each), unchanged, and passes. New cases assert `TG_TUNE`'s edges (section 13).
- The runner and the Kindle use `TG_TUNE`. A hook `GRIMOIRE_TOUCH_TUNE=v1` (sim only, joins the scenarios rule's unset
  list and `SIM_HOOKS`) selects `TG_TUNE_V1`, so a scenario can prove an old edge if needed.

### 3.3 Rule changes (each behind a `TgTune` field, so the study can turn it off)

**(a) The spread rule asks for opposite motion.** A swipe is rejected `Spread` only when BOTH hold: the spread changed
more than `spread_max x d0`, and the fingers moved apart or together along the swipe's axis (their along-axis travels
differ by more than `spread_max x d0`). A flick whose fingers converge sideways passes. `Direction` (each finger at
least `finger_mm`, cosine at least `cos_min` to the midpoint's way, travel ratio at most `finger_ratio`) still catches a
pinch.

**(b) The last lift may finish a swipe (`follow_ms`).** At the first lift of a SWIPE candidate, or of an uncommitted
session whose other finger has moved more than `tap_mm`, when the swipe tests fail only on `Short` (or the session
never committed) and the other finger is still down: keep following that finger for up to `follow_ms` (and within
`swipe_ms` of the landing). The midpoint is then the remaining finger plus half the spread it had at the first lift.
The swipe is judged when it lifts, or when it passes `swipe_mm` (fire at once), or at the deadline (`Short`). Nothing
is emitted meanwhile, so nothing on screen changes. This turns the "slow" and "short" early lifts into the swipes they
were. A tap is not affected: a session whose fingers both stayed within `tap_mm` goes to `TapWait` as today.

**(c) Zoom by opposite motion (`zoom_opposite_mm`).** At the commit, ZOOM when `zoom_ok` and either the spread change in
px is at least `zoom_ratio x travel` (today's test, with a looser ratio) or both fingers moved at least
`zoom_opposite_mm` with a negative dot product of their displacements (they went opposite ways, whatever the midpoint
did).

**(d) A late zoom (`zoom_upgrade`).** A SWIPE candidate draws nothing while moving, so it can still become a zoom
invisibly: while it is down, when `zoom_ok`, the spread has changed by at least `zoom_upgrade x d0` and the fingers move
opposite ways (c's test), it becomes ZOOM, emits its first `PinchMove(value 1)` (the editor re-anchors there, as for any
first classified move) and zooms from then on. A PAN upgrades the same way (the view starts zooming around the
midpoint; the pan offset so far is kept). A ZOOM never downgrades. Three fingers never zoom.

**(e) A lost finger does not end a zoom or pan (`grace_ms`, `regrab_mm`).** When one finger of a committed ZOOM or PAN
lifts, the session waits up to `grace_ms` (resolved by `touchgest_tick` without a frame). A contact that lands within
`regrab_mm` of the lifted one in that time takes its place: the pair is re-anchored (`pinch_d0`, midpoint) by a
`PinchMove` with `value` 1 or 2 carrying the new pair, so the view does not jump. Otherwise `PinchEnd` goes out at the
deadline, stamped with the lift's time. A swipe or tap never waits (rule 4 of G3.1 stays: no multi-finger double taps).
The editor must re-anchor when the pair changes, without a new `InEvent` field: touchgest emits `PinchBegin(value 1)`
again for the re-grabbed pair, then moves as before. `pinch()` treats a classified `PinchBegin` while `pinching` as a
re-anchor (`pinch_d0`, `pinch_z0`, `pinch_px/py` from the current view; the class kept), not a new session (brief
decision).

**(f) A pinch after writing (`zoom_after_pen`).** Today a session that lands within `pen_window_ms` of the pen never
zooms or pans. With the flag, it may, under the same rule a swipe has: the pen gone at least `pen_gone_ms` and every
finger at least `far_mm` from the pen's last point. Single fingers keep the full window.

### 3.4 Provisional looser defaults (`TG_TUNE`, before the study)

| Field | c2333a9 (`TG_TUNE_V1`) | `TG_TUNE` (prov.) | Why |
|---|---|---|---|
| `spread_max` | 0.25 | 0.50 | 31 rejects in the beta.5 log, more than any other rule; with (a) it rarely binds |
| `swipe_mm` | 20 | 16 | a flick of 1.6 cm is still deliberate on an 18 cm wide page |
| `finger_mm` | 12 | 8 | a lagging finger in a quick flick |
| `cos_min` | 0.8 (37 degrees) | 0.7 (46 degrees) | fingers fan out in a flick |
| `finger_ratio` | 2.0 | 3.0 | one finger often travels much less |
| `swipe_ms` | 1000 | 1200 | a deliberate slow swipe |
| `follow_ms` | off | 250 | (b): the "slow" and "short" early lifts |
| `tap_lift_ms`, `tap_ms`, `tap_mm` | 150, 300, 5 | 200, 400, 6 | a lighter, slower two-finger tap (lift gaps seen: 0-25 ms) |
| `zoom_ratio` | 0.6 | 0.4 | a pinch whose midpoint drifts |
| `zoom_opposite_mm` | off | 2.0 | (c) |
| `zoom_upgrade` | off | 0.15 | (d) |
| `grace_ms`, `regrab_mm` | off | 80, 15 | (e) |
| `zoom_after_pen` | false | true | (f) |
| `apart_max_mm` | 70 | 80 | a thumb and finger pinch starts wide |
| `hover_far_mm`, `hover_age_ms` | off | 60, 50 | 3.8 |
| `land_ms`, `palm_ms`, `pen_window_ms`, `pen_gone_ms`, `far_mm`, `rest_*`, `commit_*`, `tan_axis`, `apart_min_mm` | | unchanged | the log shows no trouble there; `commit_px` stays 70 so `k_pinch_gate`'s legacy dead zone is untouched |
| `palm_major` | `i32::MAX` | `i32::MAX` until the study finds a separating value | 3.1: fingertips reach 642 |

### 3.5 The measure line (grimoire.log; a new `qlog!` line, existing lines unchanged)

At the last lift of every two-finger session, after the existing `touch: session ...` line:

```
touch: measure d0 %d along %d across %d spread %d%% fingers %d %d cos %d ms %d first-lift %d pen %d
```

All integers: `d0` (landing spread), `along`, `across` (the midpoint's travel on and off its axis) and the two fingers'
travels in tenths of a mm; `spread` the signed spread change in percent; `cos` the smaller finger cosine x 100; `ms`
landing to last lift; `first-lift` landing to first lift; `pen` ms since the pen left (-1 when it was near). No
coordinates, so it may be written whenever the session line is (grabbed only). The formatter is pure
(`touchgest_log_measure(g, grabbed, out)`) and unit-tested. With it, an ordinary `grimoire.log` from the owner is enough
for a second look at the thresholds even without a trace.

### 3.6 The replay tool `tgstudy` (a host binary)

`rust/src/bin/tgstudy.rs` (a `[[bin]]` with no required features, like `grimoire-export`; never run on the device):

```
tgstudy TRACE [--tune v1|new] [--set FIELD=VALUE ...] [--sweep FIELD=A:B:STEP] [--blocks BLOCKS.txt]
```

- Reads a `touch-trace.txt` (PHASE1-BRIEF 18.2): `F` lines become `TgFrame`s on their `k_ms`, `P` lines set `TgPen`'s
  prox/touching/left_ms (the trace has no pen coordinates, so `pen.last` is unknown: the far rule is treated as met,
  and block H below is judged on timing only; say so in the output). Between frames it calls `touchgest_tick` at 10 ms
  steps of trace time. The view is fixed per block (nothing scrolls, zoom allowed; block G scrolls both axes).
- Splits the trace into blocks at hands-off gaps of 4 s or more (the script's 5 s pauses) and names them from
  `BLOCKS.txt` (one block name per line, in order: the script of 3.7 is the default). It prints the block sizes first,
  so a miscounted block is seen before anything is concluded.
- Per session: the outcome under the chosen tuning, and the measure values of 3.5. Per block: counts per outcome; for
  the swipe blocks the share that fired the intended gesture and the wrong-direction fires; for the negative blocks
  every fire listed.
- `--sweep` reruns with one field stepped and prints a table (field value, intended fires per block, negative fires,
  wrong fires).
- It links only touchgest (pure). Unit-test the trace parser and the block splitter (`rust/tests/test_tgstudy.rs`, on a
  small synthetic trace committed under `tests/fixtures/touch/synthetic.txt`).

### 3.7 The study: the owner's script and the decision rules

**The script** (docs/KINDLE.md gains "The two-finger study", about 12 minutes; the owner's session sends it with the
switch-file steps of the existing touch trace section). Hands off for 5 seconds between blocks. Natural speed and
pressure throughout: "as you would, not carefully".

| Block | What |
|---|---|
| A, B, C, D | 20 two-finger swipes up, then down, then left, then right |
| E | 10 two-finger taps |
| F | 10 pinches out, then 10 pinches in, on an unzoomed page (they should zoom) |
| G | zoomed in: 10 two-finger drags in any direction (they should pan) |
| H | 10 swipes and 5 pinches each started at once after lifting the pen from a stroke |
| I | 10 swipes with the lightest touch that still registers |
| J | 3 minutes of writing with the palm and little finger resting as usual |
| K | 2 minutes holding the Kindle and reading, turning it once |
| L | the phase 3 one-finger set: Z, N, S, circle, alpha, triangle and V, 10 each; 10 one-finger swipes each way |

The trace goes to `tests/fixtures/touch/owner-<date>.txt` with a README naming the build, the date and the blocks
(finger coordinates only; no pen coordinates are ever in a trace).

**Decision rules** (commit 7 records the numbers in `docs/index-hook/TOUCH-STUDY.md`, a short table per field):

1. Targets with `TG_TUNE`: at least 97% of blocks A-D and I fire their intended gesture; at least 95% of E fire
   `2.tap`; at least 95% of F start zooming (the session is classed ZOOM); G pans every time; 0 fires in J
   and K; 0 wrong-direction fires anywhere.
2. For each loosened field, take the most permissive value that keeps rule 1's zero-fire and zero-wrong rows true with
   every other field at its new value; within that, the value at the 2nd (or 98th) percentile of the intended
   gestures' measure for that rule. Never looser than the table in 3.4 without a measured reason.
3. A rule change (3.3 a-f) stays on only if turning it off lowers an intended-fire row by at least 2 points, or it
   fixes a named failure in the trace (for (e): a finger that lifted and landed again within 100 ms and 15 mm during a
   zoom or pan; if the trace has none, (e) stays on as harmless but is noted).
4. `palm_major`: set it only if a value separates every single-finger contact of blocks A-I and L (fingertips) from
   the broad contacts of block J with at least a 20% margin; otherwise it stays `i32::MAX` and BROAD stays log-only
   (finger shapes then rely on the palm flag, the pen rules, fingershape's gates and rule 5).
5. The finger-shape freeze gate (PLAN 15.4's dependency): with an 8-shape stress set recorded from block L (Z, N, S,
   circle, alpha, triangle, and two of the synthetic set), blocks J and K fire 0 finger shapes, and block L's shapes
   with three recorded samples accept at least 95% of the rest with 0 wrong. `s_shape_9_traces` replays this
   (section 14). If the gate fails, stop and report to the owner's session before commit 12 (shapes on the page): the
   two-finger work, the registry and pattern recording can still ship without finger shapes.

### 3.8 The pen-hover rule (G3.1 rule 6, phase 3 part)

- `TgPen` gains `hover: Option<(f32, f32, u32)>` (logical px and its time).
- input_evdev.rs: `hover_learn` runs on every contact frame while grabbed (drop the `hover_fn.is_some() ||
  sample_fn.is_some()` condition at input_evdev.rs:1094; never at the PIN), and `TgPen.hover` is filled from
  `hover_from_raw()` / `raw_hover()` (input_evdev.rs:887, 974) whenever the mapping is learned. A device without the
  raw digitizer never learns it and keeps the phase 1 rule.
- touchgest: a session is not pen-blocked by proximity alone when `hover_far_mm > 0`, the pen is in proximity and not
  touching, `hover` is at most `hover_age_ms` old at the frame, and every finger is at least `hover_far_mm` from it.
  Touching, or gone less than `pen_gone_ms` without a fresh hover point, still blocks.
- The runner's `pen hover X Y` fills `hover` with the frame clock; `pen leave` clears it.
- Live view is untouched: `hover_learn` only learns a mapping; the hover hook still fires only when installed.

### 3.9 Three fingers (G3.1 rule 2)

- `touch.3.*` fire only when `InputCaps.touch_slots >= 3` (pushed into touchgest at `input_open` as a new
  `touchgest_set_slots(g, n)`): a third contact within 250 ms of the first makes a three-finger session (never ZOOM or
  PAN), judged by the same swipe and tap rules over the three fingers' midpoint and each finger. Below 3 slots, today's
  `Three` reject stays.
- The Gestures sheet lists three-finger kinds and the record card accepts them only with 3 or more slots.
- The runner reports 10 slots; a new hook `GRIMOIRE_SIM_TOUCH_SLOTS=N` (scenarios rule unset list, `SIM_HOOKS`) sets it,
  so `g_gest_11_three` checks both.
- This is the last commit of the touch work (commit 6) and may be dropped if the owner's session says so: the owner's
  panel has two slots.

---------------------------------------------------------------------------------------------------------------

## 4. gestures.ini written (`gesturemap.rs`; G10)

Grammar, keys and the overlay model are G10's. Phase 3 adds:

```rust
pub const GM_SHAPES_MAX: usize = FS_SHAPES_MAX;
// GestureMap gains: touch1: [i32; 4] (touch.1.<dir>), shapes: [(i32 /*action*/, FsGesture); 32], nshapes,
//                   shape_newer: carried shape lines of another format (verbatim), dirty-free (no write-back cache)
pub fn gesturemap_set(m: &mut GestureMap, k: GestureKey, action: i32) -> i32;     // returns the action that lost k, 0
pub fn gesturemap_clear_action(m: &mut GestureMap, action: i32);                   // touch, touch.1, shape (pen: phase 6)
pub fn gesturemap_shape_set(m: &mut GestureMap, action: i32, samples: &[FsSample]) -> i32;  // replaces; returns loser
pub fn gesturemap_shape_of(m: &GestureMap, action: i32) -> Option<&FsGesture>;
pub fn gesturemap_shapes(m: &GestureMap) -> (&[(i32, FsGesture)]);                // the bank fs_match reads
pub fn gesturemap_format(m: &GestureMap, out: &mut Vec<u8>) -> bool;               // pure, the file's bytes
pub fn gesturemap_save(m: &GestureMap, path: &str) -> i32;                         // atomic; 0, -1 failed, 1 removed
```

- `GestureKey::Touch { fingers: 1, kind }` is now valid for Up, Down, Left, Right; `touch.1.tap` stays refused.
- Shape lines: `shape.<action>=<value>`; the value goes to `fs_decode`. `Ok(n)`: `fs_prepare` at load (about 3 ms per
  shape; 32 shapes about 0.1 s at `app_init`, acceptable; no lazy preparation, brief decision). `Malformed`: refused and
  counted. `Newer`: carried verbatim (counts toward the carry limits) and shown as "finger shape (newer version)".
- Validation through the registry (section 5): a `touch.*` line whose action lacks TOUCH, or a `shape.*` line whose
  action lacks SHAPE, is refused (this generalizes e0579ca's `hold-eraser` refusal, which then goes).
- The writer (`gesturemap_format`): the three header comment lines of G10 (byte for byte as there), then touch lines in
  the order 1.up, 1.down, 1.left, 1.right, 2.up .. 2.tap, 3.up .. 3.tap (only slots that differ from the built-ins,
  `=nothing` for a cleared built-in), then pen lines as parsed (carried until phase 6), then shape lines in action-id
  order (`fs_encode`), then carried lines in their read order. LF line ends. Nothing set: `gesturemap_save` removes the
  file (returns 1).
- Write path: `<store_root()>/gestures.ini` via `write_file_atomic`; never when the last load returned -1 (unreadable);
  the caller applies a change to a copy, saves the copy, and swaps it into `App.gestures` only on success.
- Reset: delete the file, then `gesturemap_defaults` (an unreadable file is deleted by Reset too: it is the way out).
- `gesturemap_key_for(.., ActionSrc::Shape)` is not a key: the sheet asks `gesturemap_shape_of`.

---------------------------------------------------------------------------------------------------------------

## 5. The action registry (`actions.rs`, new, pure; PLAN 12.1)

```rust
#[derive(Clone, Copy, PartialEq, Eq, Debug)] pub enum ActionKind { Toggle, Menu, Step, Cycle, OneShot, Momentary }
pub const SRC_KEY: u8 = 1; pub const SRC_PAD: u8 = 2; pub const SRC_TOUCH: u8 = 4; pub const SRC_SHAPE: u8 = 8;
pub const SRC_PEN: u8 = 16;   // set from phase 6
#[derive(Clone, Copy)]
pub struct ActionInfo { pub id: i32, pub name: &'static str, pub label: &'static str, pub ctx: Option<&'static str>,
                        pub icon: IconKind, pub group: u8, pub pair: i32, pub kind: ActionKind, pub sources: u8 }
#[derive(Clone, Copy)]
pub struct ActionGroup { pub label: &'static str, pub title: &'static str, pub icon: IconKind, pub probe: &'static str,
                         pub rows: &'static [LearnRow] }
pub fn action_info(id: i32) -> Option<&'static ActionInfo>;
pub fn action_by_name(name: &str) -> i32;        // shortcut_action keeps working through it
pub fn action_label(id: i32) -> &'static str;    // moved from library.rs:842, TRANSLATORS lines and msgctxts unchanged
pub fn action_sources(id: i32) -> u8;
pub const ACTION_GROUPS: [ActionGroup; 7];
pub const GROUP_ROWS_MAX: usize = 12;            // const assertion over every group (PLAN 12.6)
```

- `LearnRow`, the `K_*` tables and `K_GROUPS` move here from library.rs (727-842) with their `n_()` labels and
  TRANSLATORS comments unchanged. library.rs keeps drawing; shortcuts.rs keeps parsing (`K_ACTIONS` becomes the
  registry's name list, the same names and order of lookup).
- Groups (each at most 11 rows, so a list fits sideways; PLAN 12.5 adjusted to the code):
  - Editing (7): undo, redo, tool:eraser, hold-eraser (Buttons sheet only until phase 6), eraser-mode, white,
    clear-page;
  - Tools (11): unchanged;
  - Pages (8): previous / next page, first / last page, zoom out / in, scroll up / down, new-page, fit-page, pages,
    landscape-fit;
  - Index (7): links, index, go-to-page, go-back, add-link, name-page, link-areas (`index`, `add-link`, `name-page` move
    here from Pages);
  - Menus (9): unchanged at c2333a9 (tool picker, paper, display, pen feel, fold panel, library, close menu, the clock
    `status` and `status-corner`; brief decision: the clock rows stay here, the Screen group would pass 11);
  - Screen (10): invert, light down / up, warmth down / up, rotation-lock, pinch-zoom, theme, side-button, mirror-pdf,
    sleep-screen, sleep;
  - System (6): gestures, shortcuts (Gestures sheet only), bluetooth-controller (Gestures sheet only), language,
    sleep-after, update-channel.
- Lists: Buttons = 7 groups + Reset + Done = 9 rows; Gestures = the master row + 7 groups + Reset + Done = 10 rows.
- Sources: every action KEY | PAD | TOUCH | SHAPE, except `hold-eraser` (KEY | PAD; phase 6 adds PEN), `shortcuts` and
  `bluetooth-controller` (TOUCH | SHAPE: a key or controller binding would switch off its own source). Phase 1's five
  and feature B's two clock actions included.
- Every SHAPE-capable action has an EN and an ES label (test).

---------------------------------------------------------------------------------------------------------------

## 6. The remaining actions (PLAN 12.4, phase 3 rows)

New ids after `SHORTCUT_LAST = 53`, in this order (ids are memory only; files use the names): `zoom-out`, `zoom-in`,
`scroll-up`, `scroll-down`, `invert`, `light-down`, `light-up`, `warmth-down`, `warmth-up`, `rotation-lock`,
`pinch-zoom`, `theme`, `side-button`, `mirror-pdf`, `sleep-screen`, `sleep`, `gestures`, `shortcuts`,
`bluetooth-controller`, `language`, `sleep-after`, `update-channel` (`SHORTCUT_LAST` becomes 75). Dispatch in
`editor_shortcut_at` (and the Library's key path for the ones that make sense there: everything but zoom and scroll).

| Action | Kind | Does | Feedback |
|---|---|---|---|
| `zoom-out`, `zoom-in` | STEP | x0.8 / x1.25 around the anchor (the pen tip, the two-finger midpoint, the shape's box centre; the screen centre for keys), clamped to `zoom_min..zoom_max`, `view_clamp`, compose | `Ui` of the page area |
| `scroll-up`, `scroll-down` | STEP | 75% of the screen height; past the page's end continues onto the previous or next page at the same zoom and the matching edge (never adds a page) | `Ui`; a page change is `go_to_page`'s GC16 |
| `invert` | TOGGLE | `settings.invert` through the Display sheet's inverted path | Full; no toast |
| `light-down/up`, `warmth-down/up` | STEP | one of 24 steps through a non-blocking `light_cached()` (new in light.rs: the last level read or set, never a sysfs read on the input path); unknown level: toast 'Light not ready yet.' | toast 'Light %d of %d' / 'Warmth %d of %d' |
| `rotation-lock` | TOGGLE | settings.ini `rotation_lock` = 1 + the current rotation (0 = follow `follow_rotation`); `app_set_rotation` ignores sensor turns while locked | toasts 'Rotation locked.' / 'Rotation follows the Kindle.' |
| `pinch-zoom` | TOGGLE | `settings.pinch_zoom`; `touch_context` pushes it | toasts 'Pinch zoom on.' / 'Pinch zoom off.' |
| `theme` | CYCLE | Classic / Gothic: `ui_set_theme`, a leave/enter relayout as `app_set_rotation` does | Full |
| `side-button` | CYCLE | `settings.side_button` 0..3 | toast: the Settings row sentence of the new value without '(tap to change)' (new msgids where the sentence embeds it) |
| `mirror-pdf` | TOGGLE | `settings.mirror_pdf` | toasts 'PDF copies on.' / 'PDF copies off.' |
| `sleep-screen` | CYCLE | `settings.sleep_mode` 0..2 (not while `native_pin`) | toast: the Display row sentence (existing msgids) |
| `sleep` | ONE_SHOT | save now, then a new `app_request_sleep(a)` (app.rs: the power path's sleep, as the power button) | the sleep screen |
| `gestures` | TOGGLE | `settings.gestures`; off by a gesture or shape toasts 'Gestures off. Settings > Gestures turns them back on.'; on toasts 'Gestures on.' | toast |
| `shortcuts` | TOGGLE | Enable shortcuts, as the Controller sheet's row | toasts 'Shortcuts on.' / 'Shortcuts off.' |
| `bluetooth-controller` | TOGGLE | the Controller sheet's Bluetooth row's action | toasts 'Bluetooth controller on.' / 'Bluetooth controller off.' |
| `language` | CYCLE | the installed languages in the Language sheet's order; a leave/enter relayout as `theme` | toast: the language's own name (existing) |
| `sleep-after` | CYCLE | the Display sheet's values (dispmenu.rs `step_of`, `sleep_minutes`) | toast 'Sleep after: %s' with the sheet's value text |
| `update-channel` | CYCLE | `settings.updates`' three choices | toast 'Update channel: %s' with the Updates sheet's channel name |

- settings.ini `rotation_lock` 0..4, default 0, appended after `status_corner` (store.rs:2977, the newest key at
  c2333a9), with its `Settings` field, default, `settings_cb` arm, `put_int`, `settings.rotation_lock` probe,
  FORMAT.md row and `test_probe.rs` acceptance (PLAN 13.4's list).
- A TOGGLE with nothing visible toasts; a MENU's own binding closes it (unchanged); CYCLE toasts the new state.
- `editor_shortcut_at` with `src` Shape keeps phase 1's page-turn rule (never adds a page; G6.3).

---------------------------------------------------------------------------------------------------------------

## 7. Bindings by source (`shortcuts.rs`, `library.rs`; PLAN 12.6)

- `ShortcutBinding.gamepad: bool` becomes `src: ActionSrc` (Key or Pad); `shortcuts_lookup`, `_learn`, `_forget`,
  `_pads_for` take the source. shortcuts.ini and controller.ini bytes are unchanged (PORTING rule 1): the source is
  derived from which file a line is in.
- `SHORTCUT_MAX` 64 -> 192; `write_pads`' 4096-byte buffer becomes a computed size; library.rs's fixed row arrays
  (`[0i32; 4]` and the 16-entry group arrays) use `GROUP_ROWS_MAX`.
- The rule, both sheets: per source a binding does one action; PAD, TOUCH and SHAPE give an action at most one binding
  each (keys keep shortcuts.ini's many-to-one); built-in duplicates stay legal (the D-pad starters). Learning a binding
  another action holds moves it; the toast names the action that lost it ('That gesture is no longer for %s.' for
  touch and shapes; the Buttons sheet's existing toast for keys and pads).
- `BindKind` (library.rs): one descriptor drives both sheets, not a copy:
  `struct BindKind { modal: Modal, title, prompt, sources: u8, row_text: fn(&App, i32, &mut String), learn: fn(..), forget: fn(..), reset: fn(..) }`.
  The Buttons sheet is `BindKind` BUTTONS; the Gestures sheet is GESTURES. `v_controller*.qs` stay green at every
  commit (PLAN 18).

---------------------------------------------------------------------------------------------------------------

## 8. Settings > Gestures (`library.rs`; G9.1, PLAN 12.7)

### 8.1 The Settings bottom row (library.rs:1460-1477)

- KOReader 420 -> 300 px; Controller, Gestures and Updates share the middle: `mid = done.x - ko.x - ko.w - 4 x 30`,
  three equal buttons (the last takes the remainder). About 280 px each upright Gothic, 282 Classic, 306 sideways.
- Gestures: glyph F07CB (new `IconKind::Gestures` in ui.rs), 'Gestures' / 'Gestos', hit `H_GESTURES`, not gated by
  Enable shortcuts. The TRANSLATORS note becomes "each a word under its icon, about 240 px at 34 px".
- Raw taps that move (PLAN 15.4's "changed on purpose"): Controller 746 1930 and Updates 1152 1930 upright, the
  sideways Updates tap 1062 1695 (y_all_screens.qs), in v_controller*.qs, u_update_*.qs and y_all_screens.qs (19, 102,
  631 at c2333a9). New centres about 580, 890, 1200 at y 1930 upright; 863, 1199, 1536 at y 1695 sideways: confirm on
  the first run with a new probe `library.ui_at.X.Y` (the hit's name: `ko`, `controller`, `gestures`, `updates`, `done`)
  and keep the confirmed numbers.
- `test_i18n.rs`: 'Controller', 'Gestures', 'Updates' and their Spanish fit 248 px at 34 px caps in both styles.

### 8.2 The sheet (`Modal::Gestures = 17`, probe name `gestures` in `SHEETS`, library.rs:2700, which grows to 18)

- List (10 rows): 'Gestures: on (tap to change)' / 'Gestures: off (tap to change)'; the seven group rows; Reset gestures;
  Done. A line under the master row: 'Ideas: a two-finger tap for Undo, a finger shape for Pages.' (brief decision: the
  plan's three-finger idea is dropped, the owner's panel has two contacts).
- A group screen: one row per action of the group that has TOUCH or SHAPE in its sources (hold-eraser absent until
  phase 6). Row text 'Action: gesture' through `row_text`: the touch binding's name ('2 fingers down', '1 finger left',
  '2-finger tap', '3-finger tap'), else 'finger shape', else 'no gesture'; 'finger shape (newer version)' for a carried
  newer line. With a shape, a 96 x 96 thumbnail in a 120 px right column: `draw_shape_thumb(c, r, &FsGesture)` (ours,
  library.rs): the medoid sample (`fs_medoid`) scaled from 0-255 into the box, its 32 points joined by 3 px
  `canvas_line_aa` segments, black on white, 1-bit-safe. A pair row (zoom out / in) shows its two actions' gestures
  separated by ' / '.
- Tap a row: the record card (section 9). Hold a row `ROW_HOLD_MS` (600 ms): clear that action's touch and shape
  gestures (`gesturemap_clear_action`); a cleared built-in is written `=nothing`; toast 'No gestures for %s now.'.
- Reset gestures: with shapes recorded, a confirm card first ('Reset gestures? %d finger shape will be deleted.' with
  its `ng_` plural; buttons Reset / Cancel, existing msgids where they exist); then delete the file and reload.
- Opening the sheet reloads gestures.ini; a refused count > 0 toasts '%d line of gestures.ini could not be used.'
  (`ng_`); an unreadable file toasts 'gestures.ini could not be read: Reset makes a new one.' and the sheet shows the
  built-ins.
- The sheet works with Enable shortcuts off.
- Sideways: the same lists (at most 11 rows) under CHECK_FIT.

### 8.3 Probes

`library.gestures_group` (the open group's probe name or empty), `library.gestures.N` (row N's text as drawn),
`library.gestures.learning` (`0`, or the action name being recorded), `library.record` (the record card's state:
`0`, `wait`, `pattern NAME`, `shape K` with K samples, `checking`, `ready`), `library.record.line` (its progress or
reason line), `library.ui_at.X.Y`, `gestures.ACTION` (now also `touch.1.*` and `shape`), `gestures.shape.ACTION` (its
sample count, empty for none), `gestures.shape_count`, `gestures.refused`.

---------------------------------------------------------------------------------------------------------------

## 9. Recording (G9.2, G9.3)

### 9.1 touchgest's record mode

- The record card pushes `TgView { record: true, .. }` through `input_touch_context` while it is up (and false when it
  closes). In record mode touchgest emits no `PinchBegin/Move/End` side effects and reports every two-finger session
  as one `Gesture` event: a positive id on success, a negative code on a refusal. A session that classifies ZOOM is
  reported as "kept for zoom". Single-finger events are emitted as today (the card captures shapes itself), except a
  single finger landing in the pen window, reported as code -1.
- Codes (one table, `touchgest_reject_code(r) -> i32`, unit-tested both ways): -1 pen near, -2 not together (`Late`),
  -3 not straight, -4 swipe farther (`Short`), -5 not the same way (`Direction`, and `Spread`), -6 hand moved, -7 kept
  for zoom (a ZOOM class), -9 fingers too close or far (`Apart`), -10 too slow (`Slow`), -11 three fingers on a
  two-slot panel (`Three`). -8 ("one finger is kept for taps and links") is the card's own, never touchgest's (brief
  decision).
- The record mode uses the same `TgTune` as runtime (a unit test runs one session both ways and compares the verdict).
- `library_event` handles `InType::Gesture` before its `app_pointer_phase(ev) < 0` return (library.rs:2435 onwards):
  only while the record card is up; ignored otherwise.

### 9.2 The record card

- 1400 px wide, about 1000 px tall (fits sideways), centred; title 64 px 'Do the gesture for %s'; body 44 px 'Draw a
  shape with one finger, or swipe or tap with two fingers.'; three 260 x 260 dashed numbered slots
  (`canvas_dash_rect`); one progress/reason line (44 px; two lines at 34 px if needed), the only part refreshed while
  recording (GL16 of the line); footer 34 px 'One stroke, without lifting. Two turns or a loop.'; buttons Use / Try
  again / Cancel (Use disabled until a gesture is ready).
- A shape is drawn anywhere on the card except the button row, with the page's trail (10.3) in the card's area; a
  touch that starts on a button and reaches Drawing becomes a shape and the press is dropped.
- The card keeps its own `FsStroke` (FsMode::Record) and the previous lift for `fs_two_strokes`.

### 9.3 The flow (the first try decides the kind)

- **Two-finger (or three-finger) pattern**: the name at 64 px in the slots' place ('2 fingers down'); Use / Try again /
  Cancel. If another action holds it: 'Now for %s. Use moves it here.'; Use then toasts 'That gesture is no longer for
  %s.'. A refusal shows its line (code table, strings in section 15) and stays ready for another try.
- **One finger, `FsLift::Swipe(dir)`**: records as '1 finger left' (`touch.1.<dir>`), like a pattern.
- **One finger, `FsLift::Nothing`** (a tap or hold): 'One finger is kept for taps and links.'
- **One finger, `FsLift::Refused(reason)`**: the reason's sentence (section 15); `fs_two_strokes` true at the next
  landing turns a refused straight stroke into 'One stroke, without lifting.' (within `FS_TWO_STROKE_MS`).
- **One finger, `FsLift::Shape`** (sample in hand):
  - room: `fs_record_room(nshapes without this action's own)` else 'Grimoire keeps up to 32 shapes.';
  - sample 1: `fs_record_first(&s, work_pump)`: `Slip` gives 'Too easy to make by accident.' plus 'Give it two turns
    or a loop.' (slot stays empty); otherwise slot 1 fills, 'Good. Draw it again (2 of 3).';
  - samples 2 and 3: `fs_record_consistent(&s, &earlier)`: `Inconsistent` gives 'Not like the first one: draw it
    again.'; after 2: 'Once more makes it surer, or Use.' and Use enables; after 3, the checks run at once;
  - before Use acts (at Use, or after sample 3): `fs_prepare` the new gesture; `fs_record_separation(&new,
    &others_without_this_action)`: `SameAs(i)` shows 'That is the shape for %s. Use moves it here.' (Use then moves it
    and toasts 'That shape is no longer for %s.'); `TooClose(i)` shows 'Too close to the shape for %s.' and discards
    the samples; `Clear` goes on;
  - the slip bank: the line 'Checking that a slip cannot draw it...' is drawn and flushed (GL16 of the line), then
    `fs_record_bank(&g, work_pump)`; `Slip` gives 'Too easy to make by accident.' plus 'Give it two turns or a loop.'
    and discards the samples; otherwise ready.
- **Use**: write gestures.ini through a copy (section 4); on success swap the map, close the card, the row shows the
  new binding or thumbnail; on failure toast 'Could not save the gestures.' and keep the card. A new shape replaces the
  action's old shape; its touch binding stays. Replacing a carried newer line warns first: 'This replaces a shape from a
  newer Grimoire.' (Use again confirms).
- Timeout: 10 s only before the first attempt (the card closes silently); none once a sample exists. A second tap on
  the row stops recording. Rotation relayouts the card and keeps the samples. Cancel discards.

---------------------------------------------------------------------------------------------------------------

## 10. Finger shapes on the page (`editor.rs`; G8, G11)

### 10.1 Arming (G8.1)

Armed when all hold: `settings.gestures`; at least one `shape.*` line or `touch.1.*` binding; no keyboard open; the
editor open with no sheet in front except the Links bar (phase 2's card, Link to... and the strip are sheets: not
armed); not in the Pages view. Not armed: nothing changes from today, no trail ever.

### 10.2 Capture (EditorState gains `fs: FsStroke`, `fs_drawn: usize`, `fs_trail: Rect`, `fs_last_frame: u32`)

- A finger that becomes `Owner::Link` (the down branch where `link_down_at` is set) also calls `fs_begin(&mut s.fs,
  x, y, t, px100)` with `t = ev.k_ms` on the Kindle (`ev.t_ms` when `k_ms == 0`, the runner).
- Each TouchMove of that session: `fs_add(&mut s.fs, x, y, t, ev.value & TOUCH_F_BROAD != 0)`. A move inside the app's
  own slops does what it does today.
- When the link hold fires (phase 2's card opens): `fs_kill(&mut s.fs, FsReason::Held)` first. A finger whose capture
  reached Drawing never opens the card: `editor_tick`'s hold test also requires `s.fs.state != FsState::Drawing`.
- `editor_tick` calls `fs_tick(&mut s.fs, now)` only when `a.input_drained`; `Dead` thins the trail (10.4's reject
  drawing) and the rest of the touch is ignored until the lift.
- The off-screen cancel up (second finger, palm flag, pen), a `PinchBegin` (`pinch()` already drops the `Owner::Link`
  finger), `editor_suspend`, `editor_enter`, `editor_leave`: `fs_abort` (10.5).
- v1: exactly one contact for the whole session.

### 10.3 The trail (G11)

- New primitive `canvas_dot_poly(c, pts, r, pitch, halo, phase) -> i32` (canvas.rs, beside phase 2's
  `canvas_dash_poly`; a sibling instead of a mode flag so that function's signature stays, brief decision): round dots
  of radius `r` every `pitch` px along the polyline with a `halo` px white ring, 1-bit only, honouring `c.clip`,
  returning the phase. Trail: `r` 3.5 px (7 px dots), `pitch` 20, `halo` 2, scaled by `ui_px`.
- While Drawing, at most every 40 ms of event time (25 Hz): draw the points since `fs_drawn` (`fs_points`) into
  `a.screen`, `app_invalidate(new dots' rect, RefreshMode::Fast)` and its own `app_flush` at once (as the keyboard field
  does), so a trail frame never merges into a GL16. The first frame draws the path since landing. Grow `fs_trail`.
- `compose` does not redraw the trail (a recompose during a shape is rare; the lift cleans up). Colour panels: if
  `display.last_fast` reads `wait`, halve the rate there (device check only).

### 10.4 The lift (G8.3)

Before dispatch `s.owner = Owner::None` (editor_shortcut refuses while an owner is set). Then by `fs_finish(&mut s.fs,
t_lift, FsMode::Runtime, &mut sample)`:

- `Nothing`: today's tap handling, in today's order (the corner first, then the link follow).
- `Shape`: `fs_match(&sample, gesturemap_shapes(&a.gestures), &mut work_pump)`.
  - `fired: Some(i)`: `compose(a, fs_trail)`, `app_invalidate(a, fs_trail, RefreshMode::Ui)`, then
    `editor_shortcut_at(a, action, cx, cy, ActionSrc::Shape)` with the centre of `fs_box`. The cleanup GL16 and the
    action's refresh go out in one flush; a full-screen action carries the trail away. A toast names the action
    (`action_label`, 1200 ms) unless the action toasted itself (`a.toast_until` changed during dispatch) or opened a
    full-screen view. A dispatch that returns false is a reject.
  - otherwise a reject.
- `Swipe(dir)`: dispatched with `ActionSrc::Touch` when `touch.1.<dir>` is bound (same cleanup); otherwise a reject if
  the trail was drawn, nothing if not.
- `Refused(_)`: a reject if the trail was drawn, else nothing.
- **Reject** (G11): compose the trail rect and redraw every other dot (one `Fast`, own flush: it thins), then 250 ms
  later from `editor_tick` a `Ui` of the rect (the dots fade, no flash). No toast, except once ever (`hints` bit 7): 'That
  is not one of your finger shapes. Settings > Gestures lists them.'
- The trace's `E` line gets the shape outcome (`E <k_ms> shape fired undo 41 78 160`, `E <k_ms> shape reject
  NoMatch`), written by the editor through a new `input_touch_trace_event(&str)` (no-op off the Kindle).

### 10.5 Aborts (G8.6)

- Second finger or pinch: the trail rect joins the pinch's first refresh, or gets a GL16 at PinchEnd if the pinch
  never committed.
- Pen arrival: restore the trail rect with `Fast` and add it to the writing settle rect (never a GL16 while the pen is
  down).
- Sleep, rotation, leaving the editor: no refresh (the screen is redrawn anyway).

### 10.6 With the Links bar up (G8.7)

A finger whose capture reaches Drawing over the page is a shape, not "tap the page to stay" (the bar commits only
within its 30 px slop). Its lift dispatches as a key would: the bar's own shape (`shape.links`) cancels back to the
origin page (D2); another menu's shape swaps (committing a peek first, as phase 1's menu branch does); anything else is
refused and fades.

### 10.7 Probes

`fshape.last` (`<fired|reject|refused|cancel> <action|reason> <d1> <thr> <d2>`), `fshape.fired` (count since the last
read), `fshape.state` (`idle`, `pending`, `drawing`, `dead`), `fshape.trail` (`x y w h` or empty), `fshape.armed`
(`0`/`1`).

---------------------------------------------------------------------------------------------------------------

## 11. E-ink refresh choices (no new display call, no new wait)

| Surface | Waveform | Notes |
|---|---|---|
| Two-finger swipe or tap | nothing while moving; the action's own refresh | unchanged |
| PAN / ZOOM, a late zoom (3.3 d), a re-grab (3.3 e) | Fast at most every 180 ms of `t_ms`, Ui at PinchEnd | the grace delays PinchEnd by at most `grace_ms` |
| Trail frames | Fast, own flush, at most 25 Hz | never pen mode, never live ink |
| Match | Ui of the trail rect, flushed with the action's refresh | one GL16 per shape, counts toward the every-30th Full |
| Reject | Fast (thinning), then Ui after 250 ms | no flash |
| Abort by pen | Fast restore + the writing settle | never GL16 while the pen is down |
| Record card reason / progress line | Ui of that line only | |
| Gestures sheet open, group, close | as the Buttons sheet (Ui) | |
| `invert`, `theme`, `language` | Full | deliberate screen changes |
| other actions | per their kinds (section 6) | |

---------------------------------------------------------------------------------------------------------------

## 12. The runner and docs/SIM.md

### 12.1 New commands (arms in `script_step_in` before the unknown-command arm; all through touchgest)

- `shape NAME X Y SIZE_MM [MS] [ROT] [SEED] [REV]`: a synthetic one-finger shape from `fingershape_synth` (its names:
  `Z`, `N`, `S`, `circle`, `alpha`, `triangle`, `V`, `check`, `zigzag`, `square`, the synth's set), as one-contact frames
  every 13 ms through the same touchgest, on the monotonic frame clock.
- `fpath X1 Y1 X2 Y2 ... [MS]`: an exact polyline over MS (default 600), for edge tests.
- `trace FILE`: replays a touch-trace file's `F` lines as contact frames on their `k_ms` deltas and its `P` lines into
  the pen context (the same parser as `tgstudy`, in touchgest.rs as `touchgest_trace_parse_line`).
- Hooks: `GRIMOIRE_SIM_TOUCH_SLOTS=N`, `GRIMOIRE_TOUCH_TUNE=v1` (scenarios rule unset list, `SIM_HOOKS` in
  `test_scenarios.rs`).

### 12.2 SIM.md

The command table, the Checks table (every probe of 8.3 and 10.7, `settings.rotation_lock`, `touch.tune` (`v1` or
`new`)), the new Settings bottom-row coordinates, the record card's slot and button coordinates (checked with
`library.ui_at`), and a paragraph "Recording a gesture in a scenario". Selftest: `tests/selftest/pass-rust.qs` gains
`expect fshape.state is idle`, `expect gestures.shape_count is 0`, `expect library.record is 0`;
`tests/selftest/fail-rust.qs` gains `expect fshape.state is drawing`.

---------------------------------------------------------------------------------------------------------------

## 13. Unit tests

Run with `cargo test -- --test-threads=1`, host and `--features sim`.

- `test_touchgest.rs`: every existing case under `TG_TUNE_V1`, unchanged. New, under `TG_TUNE`: each loosened field on
  both sides of its new edge (swipe 15/17 mm, spread 0.45/0.55 with opposite along-axis motion, and a converging flick
  at 0.6 that passes by rule (a); cos 0.68/0.72; ratio 2.9/3.1; tap 390/410 ms); (b) a first finger lifting at 70 ms
  while the other swipes 20 mm fires, and one that stops fires nothing; (c) a pinch with midpoint drift zooms; (d) a
  swipe candidate whose spread grows 20% with opposite motion becomes ZOOM and emits PinchMove(1) once, then zooms; a
  PAN upgrades; (e) a lift and a landing 12 mm away within 60 ms keep the zoom (one PinchBegin(1) again, no PinchEnd);
  at 100 ms or 20 mm the zoom ends with PinchEnd stamped at the lift; (f) a pinch 400 ms after the pen left, 50 mm
  away, zooms; at 200 ms it does not; the hover rule on both sides of 60 mm and of 50 ms, and an unlearned mapping
  falling back; three fingers with slots 3 and 2; record mode: every code from the table, no PinchBegin/Move/End, the
  same verdict as runtime for one session; `touchgest_log_measure` writes no coordinates and nothing ungrabbed.
- `test_tgstudy.rs`: the trace parser (F, P, E, header, a bad line), the block splitter at 4 s, the replay of
  `tests/fixtures/touch/synthetic.txt` giving its expected outcomes under both tunes.
- `test_gesturemap.rs`: `touch.1.<dir>` read and written, `touch.1.tap` refused; shape lines round trip through
  `gesturemap_format` and the parser; `=nothing` leaves shapes; a `Newer` shape line carried verbatim; malformed shape
  lines refused and counted; carry limits 1 KiB per line and 16 KiB in all; at most 32 shapes; unreadable never
  written; Reset deletes; `gesturemap_set` returns the loser; a line whose action lacks the source is refused
  (`touch.2.tap=hold-eraser`, `shape.hold-eraser=...`).
- `test_actions.rs` (new): every action has a name, an EN and an ES label, a group, a kind and sources; groups at most
  11 rows and `GROUP_ROWS_MAX` holds; pairs symmetric; `hold-eraser` is not TOUCH or SHAPE; `shortcuts` and
  `bluetooth-controller` are not KEY or PAD; every name round-trips through `action_by_name`; `K_ACTIONS`' old names
  all resolve.
- `test_shortcuts.rs`: counts; learn-everything fits in 192; shortcuts.ini and controller.ini bytes unchanged.
- `test_fingershape.rs`: the items of 1.2 (pad runs, ALL-consistency, design gates).
- `test_i18n.rs`: the three bottom-row words fit 248 px at 34 px caps; every refusal fits 1320 px at 44 px or two lines
  at 34 px; the record card's title and lines fit sideways.
- `test_probe.rs`: `rotation_lock` undotted.
- canvas: `canvas_dot_poly` (dot spacing along a polyline with corners, two pieces with the returned phase equal the
  whole, clip, gray 0 and 255 only).
- library.rs `#[cfg(test)]`: `draw_shape_thumb` stays inside its box for every synth shape.

---------------------------------------------------------------------------------------------------------------

## 14. Scenarios

Every new file starts with `# rust-only`. Contact-level touch after pen input waits out the pen window (`wait 1100`).
Raw taps on the Settings row, the sheet and the record card are checked first with `library.ui_at.X.Y`.

### 14.1 New scenarios

- **`g_gest_10_forgiving`**: under `TG_TUNE`: a swipe whose fingers converge 40% sideways fires `2.up`; a swipe whose
  first finger lifts after 70 ms while the second goes on 25 mm fires (b); a pinch with midpoint drift zooms
  (`editor.zoom` changes); a swipe candidate that turns into a spread zooms (d); a zoom whose finger lifts and lands
  again within 60 ms keeps zooming, `display.presents` shows no `ui` between (e); a pinch 400 ms after a pen stroke,
  60 mm away, zooms (f). With `# env: GRIMOIRE_TOUCH_TUNE=v1`, `g_gest_10b_v1` shows the first two rejected (`touch.reject
  is spread`, `slow`).
- **`g_gest_11_hover`**: `pen hover` 80 mm from a two-finger swipe: it fires; at 30 mm: `touch.reject is pen` and the
  rate-limited toast.
- **`g_gest_12_three`**: with the default 10 slots a three-finger tap bound in gestures.ini fires; with `# env:
  GRIMOIRE_SIM_TOUCH_SLOTS=2` it is rejected `three` and the Gestures sheet shows no three-finger row.
- **`g_gest_13_onefinger`**: a learned `touch.1.left` (gestures.ini line) fires from the page, never from the panel or
  a sheet; unbound, the same stroke does nothing and draws no trail.
- **`v_actions_screen`**: every new action by key, through the `settings.*` and display probes: a light step with the
  level unknown toasts 'Light not ready yet.' and never reads sysfs; with rotation-lock on, `rotate 1` keeps the
  orientation; scroll-down past the end lands on the next page at the same zoom; zoom-in at a key zooms around the
  screen centre; a theme switch relayouts the panel (`editor.panel_rect` changes); invert gives one `full`.
- **`v_actions_system`**: language cycles and relayouts (a string probe changes language); Enable shortcuts and the
  Bluetooth controller toggle by a gesture bound in gestures.ini; turning gestures off by a gesture toasts how to turn
  them back on, and a two-finger swipe then does nothing; the Gestures sheet's master row turns them on.
- **`v_gestures_1`**: Settings > Gestures (the new button, `library.ui_at` check); Pages row; `swipe 2 ...` down: the
  card shows '2 fingers down'; Use: `gestures.pages is touch.2.down`, gestures.ini absent (a built-in); learn `touch.2.tap`
  for Undo: the line appears; learn it for Redo: the card says 'Now for Undo. Use moves it here.', Use toasts 'That
  gesture is no longer for Undo.'.
- **`v_gestures_2_reject`**: LANDGAP 300 gives 'Put the fingers down together.'; a diagonal gives 'Swipe in a straight
  line.'; a spread gives 'Two fingers spreading is kept for zoom.'; a one-finger tap gives 'One finger is kept for taps
  and links.'; a two-finger swipe 300 ms after the pen gives 'The pen is near: move it away.'.
- **`v_gestures_3_forget_reset`**: hold a row: 'No gestures for Undo now.' and the line goes; clear a built-in:
  `touch.2.down=nothing` is written and 2-down no longer opens Pages; Reset deletes the file and brings the built-ins
  back.
- **`v_gestures_es`**: the sheet, a group, the card and three refusals in Spanish, upright and sideways, both styles,
  `GRIMOIRE_CHECK_FIT=1`.
- **`s_shape_1_record`**: record `shape Z` for Undo three times: slots fill, lines as in 9.3, Use writes `shape.undo`,
  the row shows a thumbnail (shot).
- **`s_shape_2_fire`**: draw ink, `pen leave`, `wait 1100`, `shape Z ...`: `editor.strokes` drops by one, the toast
  names Undo, `display.presents` shows `fast` frames then one `ui`, no `full`; zoomed in (pinch) and sideways the same
  shape fires.
- **`s_shape_3_reject`**: `shape S` unrecorded: nothing changes; `fshape.last` starts `reject`; a `fast` then a `ui`
  250 ms later; the bit 7 tip once, never again.
- **`s_shape_4_guards`**, each giving nothing: a pen hovering (`pen hover`); a start on the panel; a second finger at
  200 ms (trail cleaned); a 450 ms rest before moving (late start); 3 s in all (the trail thins while down); a 600 ms
  pause. Also: a tap on a link still follows it; a 600 ms hold still opens phase 2's card; a shape that starts in the
  corner zone still fires; a tap there only toggles the clock (`settings.status_on`).
- **`s_shape_5_refusals`**: V gives 'Too easy to make by accident.'; a small Z 'Draw it bigger: at least 2 cm across.';
  an inconsistent second sample 'Not like the first one: draw it again.'; Z for Redo while Undo has Z 'That is the shape
  for Undo. Use moves it here.' and the toast; a near shape 'Too close to the shape for %s.'; a bar then a bar within
  700 ms 'One stroke, without lifting.'.
- **`s_shape_6_scroll`**: sideways with `landscape_fit=0`, pinched in: a mid-screen two-finger drag pans; the shape bound
  to `pages` opens Pages.
- **`s_shape_7_bar`**: with the Links bar up and a peek on, the bar's own shape cancels to the origin page; another
  shape fades; a finger tap on the page still commits.
- **`s_shape_8_match32`**: 32 shapes in gestures.ini (a fixture written by the scenario's `# copy:`), one fires;
  `main.longest_unpumped_ms is ok`.
- **`s_shape_9_traces`**: `trace tests/fixtures/touch/owner-*.txt` blocks J and K (cut into their own files) against
  the 8-shape stress set: `fshape.fired is 0`; and the legacy drags generated from earlier device logs (if any were
  provided) the same. Until the owner's trace exists, the synthetic palm drifts from `fingershape_synth` stand in, and
  the file says so.
- **`v_gestures_shape_clear_reset`**: hold a shape row clears it; Reset with shapes asks first, Cancel keeps them,
  Reset deletes them.
- `y_all_screens`: the Settings bottom row, the Gestures sheet and its seven groups, the record card (empty, a
  pattern, two samples, a refusal), the Buttons sheet's seven groups; EN and ES, both styles, CHECK_FIT.

### 14.2 Existing scenarios changed on purpose (each in the commit that changes the behaviour)

- The raw Settings bottom-row taps (8.1): v_controller*.qs, u_update_*.qs, y_all_screens.qs.
- The Buttons sheet's group list grows from 5 to 7 groups and its rows move (Pages loses index, add-link, name-page;
  Index gains them): the raw Buttons-sheet taps in v_controller.qs, v_controller_forget.qs, v_controller_switch.qs,
  v_controller_triggers.qs and y_all_screens.qs, each checked with `library.buttons_group` and `library.learning`.
- Two-finger scenarios whose expectations sit on an old edge (find them by running the suite after commit 7): add
  `# env: GRIMOIRE_TOUCH_TUNE=v1` only where the scenario tests the old edge itself; otherwise update the expectation and
  say why in the commit message.

### 14.3 Must stay green unchanged

Every pinch scenario (legacy `pinch` is direct injection: `k_pinch_gate` and the others never pass through touchgest);
every phase 1 and phase 2 gesture, bar, card and Find scenario apart from those listed above; `y_live_ink`,
`h_live_vs_settled`; the Display sheet's scenarios.

---------------------------------------------------------------------------------------------------------------

## 15. UI strings (English / Spanish)

Budgets in the `// TRANSLATORS:` line above each: toasts about 1700 px at 44 px bold or two lines; sheet rows about
1300 px at 44 px (1000 with a thumbnail); record card title 1300 px at 64 px; card lines 1320 px at 44 px or two lines at
34 px; bottom-row words 248 px at 34 px caps.

| Where | English | Spanish |
|---|---|---|
| Settings row | Gestures | Gestos |
| sheet master row | Gestures: on (tap to change) | Gestos: activados (toca para cambiar) |
| sheet master row | Gestures: off (tap to change) | Gestos: desactivados (toca para cambiar) |
| sheet line | Ideas: a two-finger tap for Undo, a finger shape for Pages. | Ideas: un toque con dos dedos para Deshacer, una figura con el dedo para Páginas. |
| group | Screen... / Screen | Pantalla... / Pantalla |
| group | System... / System | Sistema... / Sistema |
| sheet row | Reset gestures | Restablecer los gestos |
| gesture names | 2 fingers up, 2 fingers down, 2 fingers left, 2 fingers right, 2-finger tap | 2 dedos hacia arriba, 2 dedos hacia abajo, 2 dedos a la izquierda, 2 dedos a la derecha, toque con 2 dedos |
| gesture names | 3 fingers up ... 3-finger tap (as above) | 3 dedos hacia arriba ... toque con 3 dedos |
| gesture names | 1 finger up, 1 finger down, 1 finger left, 1 finger right | 1 dedo hacia arriba, 1 dedo hacia abajo, 1 dedo a la izquierda, 1 dedo a la derecha |
| gesture names | finger shape / finger shape (newer version) / no gesture | figura con el dedo / figura con el dedo (versión más nueva) / sin gesto |
| row format (`%s: %s`, msgctxt "gesture row") | %s: %s | %s: %s |
| toast | No gestures for %s now. | Ya no hay gestos para %s. |
| confirm (`ng_`) | Reset gestures? %d finger shape will be deleted. / Reset gestures? %d finger shapes will be deleted. | ¿Restablecer los gestos? Se borrará %d figura. / ¿Restablecer los gestos? Se borrarán %d figuras. |
| button | Reset | Restablecer (existing if present) |
| toast (`ng_`) | %d line of gestures.ini could not be used. / %d lines of gestures.ini could not be used. | No se pudo usar %d línea de gestures.ini. / No se pudieron usar %d líneas de gestures.ini. |
| toast | gestures.ini could not be read: Reset makes a new one. | No se pudo leer gestures.ini: Restablecer crea uno nuevo. |
| toast | Could not save the gestures. | No se pudieron guardar los gestos. |
| card title | Do the gesture for %s | Haz el gesto para %s |
| card body | Draw a shape with one finger, or swipe or tap with two fingers. | Dibuja una figura con un dedo, o desliza o toca con dos dedos. |
| card footer | One stroke, without lifting. Two turns or a loop. | De un trazo, sin levantar el dedo. Dos giros o un bucle. |
| card buttons | Use / Try again | Usar / Otra vez |
| card line | Now for %s. Use moves it here. | Ahora es para %s. Usar lo trae aquí. |
| toast | That gesture is no longer for %s. | Ese gesto ya no es para %s. |
| refusal -1 | The pen is near: move it away. | El lápiz está cerca: apártalo. |
| refusal -2 | Put the fingers down together. | Apoya los dedos a la vez. |
| refusal -3 | Swipe in a straight line. | Desliza en línea recta. |
| refusal -4 | Swipe farther. | Desliza más lejos. |
| refusal -5 | Move both fingers the same way. | Mueve los dos dedos igual. |
| refusal -6 | Your hand moved: try again. | La mano se movió: inténtalo otra vez. |
| refusal -7 | Two fingers spreading is kept for zoom. | Separar dos dedos es para el zoom. |
| refusal -8 | One finger is kept for taps and links. | Un dedo es para tocar y enlaces. |
| refusal -9 | Keep the fingers 1 to 8 cm apart. | Separa los dedos entre 1 y 8 cm. |
| refusal -10 | Too slow: do it in one go. | Muy lento: hazlo de una vez. |
| refusal -11 | This Kindle feels two fingers at most. | Este Kindle detecta dos dedos como máximo. |
| shape line | Good. Draw it again (2 of 3). | Bien. Dibújala otra vez (2 de 3). |
| shape line | Not like the first one: draw it again. | No se parece a la primera: otra vez. |
| shape line | Once more makes it surer, or Use. | Una vez más la afina, o Usar. |
| shape line | That is the shape for %s. Use moves it here. | Es la figura de %s. Usar la trae aquí. |
| toast | That shape is no longer for %s. | Esa figura ya no es para %s. |
| shape line | Too close to the shape for %s. | Muy parecida a la figura de %s. |
| shape line | Checking that a slip cannot draw it... | Comprobando que un roce no la trace... |
| shape line | Too easy to make by accident. | Demasiado fácil de hacer sin querer. |
| shape line | Give it two turns or a loop. | Dale dos giros o un bucle. |
| refusal TooSmall | Draw it bigger: at least 2 cm across. | Dibújala más grande: 2 cm o más. |
| refusal TooSlow | Too slow: draw it in one go. | Muy lenta: dibújala de una vez. |
| refusal LateStart | Start moving as soon as you touch. | Muévete en cuanto toques. |
| refusal Paused | You held still, so it stopped. | Te detuviste y se canceló. |
| refusal Straight | Too straight: give it two turns. | Muy recta: dale dos giros. |
| refusal Tangled | Too many turns: draw a simpler shape. | Demasiadas vueltas: hazla más simple. |
| refusal Broad | Use a fingertip, not the hand. | Usa la yema del dedo, no la mano. |
| refusal TwoStrokes | One stroke, without lifting. | De un trazo, sin levantar el dedo. |
| refusal Full | Grimoire keeps up to 32 shapes. | Grimoire guarda hasta 32 figuras. |
| warning | This replaces a shape from a newer Grimoire. | Esto sustituye una figura de un Grimoire más nuevo. |
| page tip (bit 7) | That is not one of your finger shapes. Settings > Gestures lists them. | No es una de tus figuras. Ajustes > Gestos las muestra. |
| toasts | Gestures off. Settings > Gestures turns them back on. / Gestures on. | Gestos desactivados. Ajustes > Gestos los vuelve a activar. / Gestos activados. |
| action labels | Zoom out / zoom in; Zoom out; Zoom in | Alejar / acercar; Alejar; Acercar |
| action labels | Scroll up / down; Scroll up; Scroll down | Desplazar arriba / abajo; Desplazar arriba; Desplazar abajo |
| action labels | Invert colors | Invertir los colores |
| action labels | Light down / up; Less light; More light | Menos / más luz; Menos luz; Más luz |
| action labels | Warmth down / up; Less warmth; More warmth | Menos / más calidez; Menos calidez; Más calidez |
| action labels | Lock rotation; Pinch to zoom; Switch theme; Side button; PDF copies; Sleep screen; Sleep now | Bloquear la rotación; Pellizcar para ampliar; Cambiar de tema; Botón lateral; Copias en PDF; Pantalla de reposo; Reposo ahora |
| action labels | Gestures; Enable shortcuts (existing); Bluetooth controller (existing); Language (existing); Sleep after (existing); Update channel | Gestos; (existing); (existing); (existing); (existing); Canal de actualizaciones |
| toasts | Light not ready yet. / Light %d of %d / Warmth %d of %d | La luz aún no está lista. / Luz %d de %d / Calidez %d de %d |
| toasts | Rotation locked. / Rotation follows the Kindle. | Rotación bloqueada. / La rotación sigue al Kindle. |
| toasts | Pinch zoom on. / Pinch zoom off. | Zoom con pellizco activado. / Zoom con pellizco desactivado. |
| toasts | PDF copies on. / PDF copies off. | Copias en PDF activadas. / Copias en PDF desactivadas. |
| toasts | Shortcuts on. / Shortcuts off. | Atajos activados. / Atajos desactivados. |
| toasts | Bluetooth controller on. / Bluetooth controller off. | Mando Bluetooth activado. / Mando Bluetooth desactivado. |
| toasts | Sleep after: %s / Update channel: %s | Reposo tras: %s / Canal de actualizaciones: %s |

Reuse an existing msgid wherever the English text is identical (check `po/es.po` first: 'Cancel', 'Done', 'Reset',
'Invert colors' may exist in another form; a different wording is a new msgid). po/es-glossary.md: "gesto", "figura
con el dedo" (never just "figura" in a row name: Smart shapes own "forma"), "toque".

---------------------------------------------------------------------------------------------------------------

## 16. Docs to update in phase 3

- **docs/FORMAT.md:** gestures.ini written (header, order, `touch.1.*`, `shape.<action>` format 1 byte for byte, carry
  limits, removal when empty, Reset); `rotation_lock`; `hints` bit 7; the `touch: measure` log line in the log section.
- **docs/SHORTCUTS.md:** the registry table (every action, kind, group, sources), "gestures do not need Enable
  shortcuts", the unassignable list (calibration; hold-eraser on touch and shapes; Enable shortcuts and the controller
  on keys and pads), the binding rule.
- **docs/SIM.md:** section 12.2.
- **docs/DESIGN.md:** the Settings row, the Gestures sheet, the record card, the trail, the refresh table of section 11.
- **docs/KINDLE.md:** "The two-finger study" (3.7) and the phase 3 owner checks; TESTING.md the same checks.
- **docs/index-hook/TOUCH-STUDY.md** (new): the study's numbers and the frozen values (3.7).
- **docs/index-hook/PURE-MODULES.md:** fingershape merged, its items closed.

---------------------------------------------------------------------------------------------------------------

## 17. Commit plan and the gate

### 17.1 Small logical commits, in this order

Each message ends with CLOUD.md's attribution lines. Each leaves `make rust-check` green.

1. `fingershape: from claude/rust-index-pure-p4zqq5 (<commit>), the synthetic corpus in src` (1.2).
2. `fingershape: slip bank pad, consistency against every sample, the design's corpus gates` (1.2 items 1-4).
3. `touchgest: one tuning table (TgTune, TG_TUNE_V1 = today), the measure log line` (3.2, 3.5; tests under V1 only;
   behaviour unchanged: `TG_TUNE = TG_TUNE_V1` in this commit).
4. `touchgest: the spread rule, the last lift, zoom by opposite motion, the late zoom, the lost finger, the pinch after
   writing` (3.3 rules behind their fields, still off by default; tests set them on).
5. `tools: tgstudy, a touch-trace replay; touchgest's trace parser; runner trace command` (3.6, 12.1's `trace`).
6. `touchgest, input_evdev: the hover rule; three fingers where the panel has them` (3.8, 3.9; `g_gest_11_hover`,
   `g_gest_12_three`). `TG_TUNE` takes the hover fields' values here (60 mm, 50 ms), the only fields set before commit
   7. The three-finger half may be dropped (3.9).
7. `touchgest: looser two-finger defaults from the study` (3.4, 3.7: `TG_TUNE` gets its values; TOUCH-STUDY.md;
   `g_gest_10_forgiving`, `g_gest_10b_v1`; 14.2's last item). Push here.
8. `actions.rs: the registry; library.rs reads it; Buttons sheet in seven groups` (5, 7's BindKind for BUTTONS,
   `test_actions.rs`, 14.2's Buttons taps).
9. `shortcuts: bindings by source, SHORTCUT_MAX 192` (7).
10. `editor, app, light, store: the remaining actions, rotation_lock` (6, `v_actions_screen`, `v_actions_system`,
    l10n).
11. `gesturemap: gestures.ini written, touch.1 lines, shape lines` (4, `test_gesturemap.rs`).
12. `editor: finger shapes on the page (capture, trail, match, reject), runner shape and fpath` (10, 12.1,
    `canvas_dot_poly`, `s_shape_2/3/4/6/7/8/9`, `g_gest_13_onefinger`, l10n). Only after 3.7's freeze gate passed.
13. `library: Settings > Gestures (the bottom row, the sheet, hold to clear, Reset)` (8, `v_gestures_3`, 14.2's
    Settings taps, l10n).
14. `library, touchgest: recording (record mode, the record card, patterns, swipes and shapes)` (9, `v_gestures_1`,
    `v_gestures_2_reject`, `s_shape_1_record`, `s_shape_5_refusals`, `v_gestures_shape_clear_reset`, l10n). Push here.
15. `hints: the finger-shape reject tip` (bit 7).
16. `y_all_screens and l10n-check: the Gestures sheet, the record card, the seven groups` (`v_gestures_es`);
    `docs: phase 3` (16). Push.

### 17.2 The gate: `make rust-check`

As in PHASE2-BRIEF 16.2: unit tests twice (host, `--features sim`), headless, tools (`tgstudy` builds), the armv7
build, scenarios plus selftest, l10n-check, mirror-cleanup-check, share_check, live_check, bt_check and the exporter,
with `GRIMOIRE_TEST_PORT=8850`. On this Linux machine Apple-libm float fixtures and outline-font CRCs may differ:
never edit an expectation to pass on Linux; list the differences in your report; the Mac gate is authoritative.
fingershape's scores are integers and must match exactly on every host. Count the existing scenarios this phase
converts and give the number in the report. Never run a GUI binary, never touch a device.

---------------------------------------------------------------------------------------------------------------

## 18. Owner acceptance on the Kindle (by hand, numbered)

1. **Two fingers, at your speed.** Turn pages with two fingers left and right, open Pages and the Links bar with two
   fingers down and up, quickly, slowly, lightly, with the fingers drifting together: they should work without
   hunting for the right pressure. Pinch to zoom, including straight after writing and with the fingers drifting:
   the zoom starts. Lift a finger for an instant mid-pinch: the zoom carries on.
2. **Nothing by accident.** Write a page with your palm resting, then hold the Kindle and read for a few minutes:
   no page turns, no menus, no finger shape fires.
3. **The pen nearby.** Swipe with two fingers while the pen hovers away from them: it works. With the pen right next
   to the fingers it says 'Move the pen away, then swipe.' now and then.
4. **Settings > Gestures.** The Settings bottom row reads KOReader | Controller | Gestures | Updates | Done (Mando,
   Gestos, Actualizaciones in Spanish), upright and sideways.
5. **Recording a pattern.** Record a two-finger tap for Undo and try it. Record one that is taken: the card names the
   action that loses it. Make bad tries on purpose (fingers far apart, diagonal, pen near): each is refused with its
   reason.
6. **Recording a finger shape.** Record a Z, an N or a loop for Pages, three times; then draw it anywhere on the page,
   upright, sideways and zoomed in: the Pages view opens. A V or a check mark is refused as too easy to make by
   accident. A shape you did not record fades away.
7. **The corner.** A tap in the clock corner still toggles the clock; a finger shape you start there still fires.
8. **Clearing.** Hold a row to clear it. Reset restores the built-ins and asks first when shapes exist.
9. **Every toggle.** Controller > Buttons shows seven groups. Bind light, invert, rotation lock, sleep screen, theme,
   language, sleep after and 'Whole page sideways' to keys, and some of them to gestures or shapes; Enable shortcuts
   and the Bluetooth controller to a gesture or shape. Each does what its name says, with a toast where nothing
   visible changes.
10. **Report** any two-finger try that still needs care, with what you did (fast, slow, light, after writing); if asked,
    run the two-finger study again (docs/KINDLE.md) and send the files.

Known in this build: no three-finger rows on this Kindle (two contacts); pen-button gestures come in phase 6; one-turn
shapes are refused.

---------------------------------------------------------------------------------------------------------------

## 19. Brief decisions (fold into PLAN-V2 section 19)

1. The two-finger thresholds become one runtime table `TgTune` with `TG_TUNE_V1` (c2333a9's behaviour) and `TG_TUNE`
   (phase 3's); record mode and the replay tool read the same table.
2. Four rule changes beyond numbers (3.3): the spread rule needs opposite along-axis motion; the last lift may finish a
   swipe; zoom by opposite motion and a late zoom from a swipe candidate or a pan; a zoom or pan survives a finger lost
   for up to 80 ms (re-grab re-anchors through a second `PinchBegin(1)`); a pinch may start in the pen window under the
   swipe's far rule.
3. Provisional defaults of 3.4 ship if no trace arrives; the study (3.7) then refines them with stated decision rules.
4. A new `touch: measure` log line per two-finger session (integers, no coordinates).
5. `tgstudy` is a host binary in the crate; the trace parser lives in touchgest.rs and serves the runner's `trace`.
6. The owner's panel has two slots: three-finger support is built gated on `touch_slots >= 3`, last in the touch work,
   and droppable.
7. fingershape's synthetic corpus moves into `src/` (compiled always, called only by the runner and tests).
8. The slip bank is pumped, not resumable; Cancel during the check is handled after it.
9. Record-mode codes: PLAN's -1 to -7, then -9 apart, -10 slow, -11 three; -8 is the card's own.
10. Shapes are prepared at load (no lazy preparation past 8).
11. The clock rows (`status`, `status-corner`) stay in the Menus group (9 rows); the Screen group keeps 10.
12. The Gestures sheet's ideas line drops the three-finger idea; no footer line (SHORTCUTS.md carries the unassignable
    list).
13. `canvas_dot_poly` is a sibling of `canvas_dash_poly`, not a mode of it.
14. A Drawing finger never opens phase 2's link card; the card's hold kills the capture (`fs_kill(Held)`).
15. New strings beyond PLAN-V2's: the 'off' master row, the ideas line, the 'no gesture' and '1 finger' names, the
    malformed/unreadable/save toasts, refusals -9 to -11, the action labels and toasts of section 6 not given in PLAN
    12.4.
16. The corner is live only while the page is in front (status.rs `page_in_front`), so the record card (in Settings)
    has no corner rule; PLAN G4.2's record-card row does not apply.
