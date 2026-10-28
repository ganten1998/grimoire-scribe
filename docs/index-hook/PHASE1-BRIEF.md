# Phase 1 implementation brief: gestures and the Links bar (Rust)

This brief is self-contained: with it and the repository you can build phase 1. It applies
`docs/index-hook/PLAN-V2.md` (the authority; section numbers below, such as G3 or 7.4, are its sections), the owner's
rulings in `docs/index-hook/README.md`, `docs/index-hook/CLOUD.md` and the contract `rust/PORTING.md`. Where this brief
is more precise than the plan, the brief says so in place ("brief decision"), and PLAN-V2 has been corrected to match
every such decision (its section 19 lists them), so the two agree. If you still find a disagreement, a marked brief
decision wins for phase 1 code and anything else follows the plan; raise the difference with the owner's session
rather than guessing.

All paths are relative to the repository root. Rust line numbers are at `claude/rust-index-base` (17cbfbc). They
drift, so search by the function name. Every existing name this brief cites was checked to exist there.

---------------------------------------------------------------------------------------------------------------

## 0. What phase 1 is

Phase 1 is one Kindle build that the owner installs and tests. It contains:

1. **The touch pipeline.** The multitouch classifier moves out of `input_evdev.rs` into a new pure module,
   `touchgest.rs`. Both the Kindle reader and the headless runner feed it contact frames. It emits today's
   TouchDown/Move/Up and Pinch* stream, plus a new `InType::Gesture`. The extraction carries these input fixes:
   - kernel time per frame;
   - SYN_DROPPED resync;
   - touch drained during long work;
   - off-screen cancels instead of real lifts;
   - pairing by landing order;
   - a busy flag that counts every contact.
2. **The two-finger built-ins (D6)**, the mirror rule and per-axis panning (D5):
   - two fingers down open Pages;
   - two fingers up open the Links bar;
   - two fingers left and right turn the page and never add one.
3. **Sideways, show the whole page** (`landscape_fit`, on by default) and its action.
4. **Contact-level runner input** (`finger`, `frame`, `swipe`, `tapn`, `tap panel NAME`) through the SAME
   touchgest, with `touch.*` probes.
5. **The touch log** in grimoire.log and the opt-in `touch-trace.txt`.
6. **The Links bar**:
   - the list (one row per Index entry, xN);
   - peek and commit;
   - the back list;
   - the Find sheet (Go to page or name);
   - the eye (link areas);
   - the panel's Links button.
7. **Five new action names**, bindable to keys and controller buttons at once: `links`, `go-to-page`, `go-back`,
   `link-areas` and `landscape-fit`.

### 0.1 Explicitly out of scope (later phases; do not build them now)

| Not in phase 1 | Phase |
|---|---|
| Ink-free link making, link areas stored in links.ini, migration of beta.5 links, smallest-wins hit test, link edits in undo, the long-press card on the page or in the bar, Link to..., Find PICK mode, Copy page keeping links | 2 |
| Settings > Gestures, recording, writing gestures.ini, the integer $P matcher (fingershape.rs), finger shapes and their trail, three-finger gestures, the pen-hover acceptance rule, the live T_palm size guard, `actions.rs`, `ShortcutBinding.gamepad` becoming a source, `SHORTCUT_MAX` 192, the Settings bottom row, every-toggle assignment beyond the five names above, `rotation_lock` | 3 |
| Smart shapes (shape.rs), the Shapes panel button, `canvas_dash_poly`, `canvas_slash` | 4 |
| The Index map (graph.rs, mapview.rs), the bar's Index map chip | 5 |
| Pen-button gestures (pen lines in gestures.ini become live) | 6 |
| The keyboard dock (`keyboard_set_dock`, a `dock_top` field in `modal_rect`): it has no phase 1 caller | 2 (PICK mode) |
| Window emulation (Shift / Alt / Ctrl + right button) | not applicable: the SDL window simulator is not ported (`rust/src/sim_sdl.rs:8-11`); only the headless runner exists |

Known in this build (say so in the owner notes):
- New link runs today's inked `link_start` flow (editor.rs:2258).
- Holding a link on the page still opens the Pages-view picker (`link_hold`, editor.rs:2439).
- Holding a bar row does nothing (a hold past 600 ms cancels the press silently).
- Three-finger gestures are off.
- The area boxes are the legacy ink boxes.

No pure module from `claude/rust-index-pure` is needed in phase 1 (fingershape.rs, shape.rs and graph.rs come in
phases 3, 4 and 5).

---------------------------------------------------------------------------------------------------------------

## 1. Branch, base, and what must land first

- Base: `claude/rust-port` **after** both pre-features below are merged there. Branch: `claude/rust-index-p1` (or
  the name your task gives). Never push to `main`, `claude/rust-port` or `claude/beta5-features`.
- Set up the machine with `docs/index-hook/CLOUD.md` (toolchain 1.97.1, armv7 musl target, cargo-zigbuild).

### 1.1 Feature A: Live view fast ink (must be merged)

It is described as four parts: a pen sample hook in input_evdev, a capture-blocked flag in app dispatch, the editor's
live ink style, and a separate `/live/pen` stream. Before you write code:

1. Find its names with `git log claude/rust-port --oneline -- rust/src/input_evdev.rs rust/src/app.rs rust/src/live.rs`
   and `grep -n "capture\|pen_hook\|live/pen\|live_ink_style" rust/src/*.rs`.
2. Note:
   - (a) the pen sample hook's site, expected at the end of `emit_pen` (input_evdev.rs:957) with its call deferred
     outside the `State` borrow, as `drain` (input_evdev.rs:2302) does for the hover hook;
   - (b) the capture-blocked predicate or setter, and where `app_event` (app.rs:629) reads it;
   - (c) the editor's live ink style.
3. Rules for phase 1:
   - The touch extraction touches the touch path only. Leave untouched:
     - `emit_pen`;
     - the pen sample hook and its deferred call;
     - `read_pens` and the pen-first order in `input_pump` (input_evdev.rs:2579) and `input_poll` (2616).
   - Touch drained in `input_pump` goes AFTER the pens.
   - Every pen contact that will not become ink raises the capture-blocked flag at its PenDown. In phase 1 these are:
     - the pen commit during a peek, whose stroke is swallowed (section 13.6);
     - pen taps on the Links bar;
     - pen taps on the Find sheet and its keyboard.

     **Integration.** Add `pub fn editor_pen_not_ink(a: &App) -> bool` (editor.rs). It returns true while
     `lb.open || find.open`, or while `owner == Owner::Outside` with `owner_is_pen`. OR it into feature A's predicate.
     - If feature A uses a per-PenDown setter instead, call that setter in the three places above.
     - If feature A's predicate already derives from `sheet_open` (editor.rs:2032), extending `sheet_open` with
       `lb.open || find.open` (section 11.3) covers the bar and Find, and only the swallowed commit stroke needs the
       explicit call.
   - If the flag gates screen dispatch ("input owned elsewhere"), put the new `InType::Gesture` branch of `app_event`
     after it.
   - The bar, the eye, the Find sheet and gestures are `App.screen` pixels. They reach the screen tile stream, never
     `/live/pen`. Never draw them with the editor's live ink style: that style means ink.

### 1.2 Feature B: the clock/battery corner (must be merged)

It is described as a finger TAP in a ~12 mm top-left zone that toggles a clock/battery overlay. No gesture may claim
that tap, and a shape that starts there is still a shape.

1. Find its names: `grep -n -i "clock\|corner\|battery" rust/src/*.rs`.
2. Phase 1 needs two things from it:
   - **the zone**, `{0, 0, z, z}` with `z = mm_to_px(12.0, a.dpi)` (util.rs:173, about 142 px), in the logical frame
     of the current orientation;
   - **a predicate** that is true for a one-finger, unmoved (30 px slop, `press_move`, ui.rs:817), real lift whose
     LANDING is in the zone. It decides at the lift, never at the down.
3. **Integration shim** (brief decision). Add two private helpers in editor.rs and call only these from phase 1 code,
   so a name difference is fixed in one place:

   ```rust
   fn corner_zone(a: &App) -> Rect                 // feature B's zone, or {0,0,z,z} as above
   fn corner_tap(a: &App, x0: f32, y0: f32, moved: bool, cancelled: bool) -> bool
   ```

   `corner_tap` returns false for a cancel up (`TOUCH_F_CANCEL`, x = y = -10000) and for the pen, and otherwise
   defers to feature B.
   - **B decides at the lift and toggles the clock itself:** `corner_tap` only answers whether the lift was the
     corner's.
   - **B swallows the lift in `app_event` before the screen sees it:** check that it hands the screen an off-screen
     cancel up instead, so `Owner::Link`, `Owner::Outside` and `Owner::Bar` end cleanly. If it does not, add exactly
     that (a TouchUp at -10000 with `TOUCH_F_CANCEL`) in B's branch, as a separate commit.
   - **B decides at the down:** stop. Ask the owner's session for B to decide at the lift. Phase 3's shapes from the
     corner depend on it (PLAN-V2 section 18, "Feature contracts").
4. Its probe: scenarios below write `expect clock.on is 1`. Replace `clock.on` with feature B's real probe key. If it
   has none, add one in a separate commit with a SIM.md row.

### 1.3 Pre-flight

- Run `make rust-check` on the base before any change, and record which checks fail on this Linux machine
  (section 22).

---------------------------------------------------------------------------------------------------------------

## 2. Hard rules that apply to every commit

1. No em dashes (U+2014) in code, comments, docs, UI or commit messages. Use commas, colons and parentheses. Check
   with `grep -rn $'\u2014' rust/src docs po tests` before each commit.
2. Every UI string in English and Spanish, in the same commit:
   - `tr` / `n_` / `c_` / `nc_` / `ng_` with a `// TRANSLATORS:` line directly above, carrying the width budget;
   - formats through `cformat!` / `csnprintf!`;
   - edit `po/es.po`, then `make l10n` where it works (see PORTING "Packaging" 4: do not commit its output
     wholesale), then `msgfmt --check --no-hash --endianness=little -o po/es.mo po/es.po`; commit .po and .mo together;
   - strings fit both orientations: every new surface is visited in `y_all_screens` upright and sideways, in both
     styles, under `GRIMOIRE_CHECK_FIT`, and `make l10n-check` passes;
   - a changed meaning gets a new msgid, never a changed translation.
3. Never change the `.qpg` format: no header version bump, no tool id >= 10, no ext flag.
4. Never add an `MXCFB_WAIT_FOR_UPDATE_SUBMISSION` wait or a second wait on one display marker. Add no display calls:
   - screens use `app_invalidate` (app.rs:1329) and `app_flush` (app.rs:1346);
   - `app_press_feedback` (app.rs:1419) is the one existing immediate DU;
   - separate `app_flush` calls only where this brief says so.
5. The main loop never blocks ~100 ms: the pen's evdev buffer overflows. Long work calls `work_pump()` (util.rs:452):
   page renders already pump every 64 rows and every 128 points. New loads use the pumped reader (section 13.9).
   Prefetch does one band per idle tick.
6. Pen mode tolerates only DU: no GL16 and no toast while the pen is down.
7. PORTING.md rule 1, byte-exact:
   - settings.ini keeps every existing line's bytes and order, and the new undotted keys go after `language`;
   - shortcuts.ini and controller.ini are unchanged;
   - existing qlog lines are unchanged, and new lines are `qlog!` with printf formats;
   - every existing `app_probe` key and value is unchanged;
   - existing runner commands keep their exact event sequences;
   - `display.presents` of untouched flows is unchanged;
   - an unknown command still fails the run.

   PORTING rule 2 governs everything this brief does not deliberately change. The deliberate changes are listed in
   section 20.4.
8. Timing: `now_ms` and `k_ms` are wrapping u32 compared with `wrapping_sub`. Recognizers time by the frame's time,
   never by `now_ms()` inside a batch.
9. Global state: module state is `thread_local! RefCell` with short borrows. Never call another module, `work_pump()`
   or a callback while holding a borrow, and have no global App pointer. touchgest is pure: its state lives inside
   input_evdev's and sim_sdl's `State`, and it never calls out.
10. `unsafe` only for FFI and ioctls (`EVIOCGABS`, `EVIOCGMTSLOTS`), each with `// SAFETY:`.
11. Float rules: never `mul_add` (use `fmla`). `zoom_max` keeps today's expression and order, so the ceiling is
    bit-identical. touchgest classifies in integer px rounded from `map()`'s f32 and emits `map()`'s f32 unchanged.
12. Memory: allocations sized by data use `try_reserve_exact` and keep a slower path that draws the same pixels.
13. The clock corner's one-finger TAP is never claimed by anything this phase adds (section 9.2).
14. The pen always wins: pen proximity or contact cancels every finger session, and the pen writes even on a link
    (section 9).

---------------------------------------------------------------------------------------------------------------

## 3. The event contract (`input.rs`, `mainloop.rs`, `power.rs`, `platform_none.rs`, `app.rs`)

### 3.1 input.rs

- `InType` (input.rs:32): append `Gesture = 24` after `KeyUp = 23`, with the comment "two or three fingers made a
  recognized gesture: value = id; (x, y) start midpoint, (x2, y2) end midpoint".
- New constants:

  ```rust
  pub const TOUCH_F_BROAD: i32 = 1;   // TouchDown/Move/Up value bit: TOUCH_MAJOR passed T_palm (log only in phase 1)
  pub const TOUCH_F_CANCEL: i32 = 2;  // TouchUp value bit: the off-screen cancel (x = y = -10000.0, as today)
  ```

- `InEvent` (input.rs:82) is unchanged in layout. Touch events now also carry `k_ms`: the SYN_REPORT kernel time in
  `now_ms()`'s clock (`pensync_ms`, pensync.rs:355); 0 in the runner unless set (the runner sets `k_ms = t_ms`).
  `PinchBegin.value`:
  - 0: legacy (runner `pinch`; the editor behaves as today);
  - 1: a touchgest-classified session.

  `PinchMove.value`:
  - 0: legacy;
  - 1: ZOOM;
  - 2: PAN;
  - 3: SWIPE.
- `InputCaps` (input.rs:129) gains `pub touch_slots: i32` (`EVIOCGABS(ABS_MT_SLOT).maximum + 1`; 0 unknown; the
  simulator reports 10).
- New platform-forwarded entry points (input.rs, `platform::...` like `input_hover_hook` at input.rs:212). Each needs
  a body in input_evdev.rs, sim_sdl.rs and platform_none.rs:

  ```rust
  pub fn input_touch_context(v: TgView);            // the editor's view context for touchgest (section 4.2)
  pub fn input_touch_trace(path: Option<&str>);      // phase 1 opt-in trace file (section 18.2); None stops it
  ```

### 3.2 mainloop.rs

- `event_name` (mainloop.rs:151): `K` becomes `[&str; 25]` with `"gesture"` last. The assert (mainloop.rs:178) becomes
  `assert!(K.len() == InType::Gesture as usize + 1, ...)`, and the range check uses `InType::Gesture`.
- `App.input_drained: bool` (new App field, app.rs:32). In `run()`:
  - set it true when the batch loop ended because `poll(&mut ev, 0, ...)` returned 0, or when the outer poll returned
    0;
  - set it false when the loop ended on the 64-event or 8 ms limit.

  The prefetch and the bar's hold timer read it.

### 3.3 power.rs

- `power_asleep_drops` (power.rs:187) adds `InType::Gesture` to the dropped list.

### 3.4 app.rs `app_event` (app.rs:629)

- The asleep drop (app.rs:751) now covers Gesture, through `power_asleep_drops`.
- The book-running drop (app.rs:772) adds `|| ev.type_ == InType::Gesture`.
- The presence condition (app.rs:777) adds `|| ev.type_ == InType::Gesture`, so `note_input` runs.
- The feature A flag: if it gates screen dispatch, the Gesture event reaches the screen only past it (section 1.1).
- Gestures do not depend on `settings.shortcuts_enabled`. The master switch is `settings.gestures`, applied inside
  touchgest through `TgView.gestures_on`.
- Every screen except the editor ignores Gesture with no change: `library_event` returns on
  `app_pointer_phase(ev) < 0` (library.rs:2405), and the others do the same.

---------------------------------------------------------------------------------------------------------------

## 4. `touchgest.rs` (new, pure, host-tested)

Model it on `pensync.rs`: no App, no I/O, no allocation, O(1) per frame. Register it in `rust/src/lib.rs`:
- `pub mod touchgest;` in "wave 1: foundations", beside `pensync` (lib.rs:44);
- `pub use crate::touchgest::*;` in `prelude`.

### 4.1 Types

```rust
pub const TG_MAX: usize = 16;                       // as MAX_SLOTS (input_evdev.rs:359)

#[derive(Clone, Copy, Default)]
pub struct TgContact {
    pub id: i32,        // tracking id (runner: the finger ID)
    pub slot: i32,      // evdev slot (runner: the finger ID)
    pub x: f32, pub y: f32,     // logical screen px after map() (input_evdev.rs:759; runner: make_ev)
    pub nx: f32, pub ny: f32,   // device-normalized 0..1 before calibration: calibrate.rs:310 reads ev.nx/ny
    pub major: i32,     // ABS_MT_TOUCH_MAJOR, 0 unknown
    pub tool: u8,       // 0 finger, 1 pen, 2 palm (MT_TOOL_PALM)
    pub pressure: i32,  // ABS_MT_PRESSURE, 0 unknown
}
#[derive(Clone, Copy, Default)]
pub struct TgFrame { pub k_ms: u32, pub n: usize, pub c: [TgContact; TG_MAX] }   // every contact down this frame

#[derive(Clone, Copy, Default)]
pub struct TgPen {
    pub prox: bool,         // pen or eraser in proximity
    pub touching: bool,
    pub left_ms: u32,       // when it last left proximity (now_ms clock; State.pen_left_ms on the Kindle)
    pub last: (f32, f32),   // its last contact point, logical px
}

#[derive(Clone, Copy, Default, PartialEq)]
pub struct TgView {
    pub scrolls_v: bool, pub scrolls_h: bool,  // the page can scroll on that axis (section 8.4)
    pub zoom_ok: bool,      // settings.pinch_zoom and no sheet that refuses pinches
    pub gestures_on: bool,  // settings.gestures
    pub record: bool,       // phase 3; always false now
    pub keyboard_open: bool,
}

pub struct TgOut { pub n: usize, pub ev: [InEvent; 8], pub outcome: TgOutcome }

#[derive(Clone, Copy, Default, PartialEq)]
pub enum TgOutcome { #[default] None, Fired(i32), Rejected(TgReject) }   // for the log, the trace and the probes

#[derive(Clone, Copy, PartialEq)]
pub enum TgReject { Pen, Apart, Straight, Short, Direction, Hand, Spread, Late, Three, Keyboard, Off, Slow }

pub const G_2_UP: i32 = 1;  pub const G_2_DOWN: i32 = 2;  pub const G_2_LEFT: i32 = 3;
pub const G_2_RIGHT: i32 = 4; pub const G_2_TAP: i32 = 5;
pub const G_3_UP: i32 = 6;  /* ... */ pub const G_3_TAP: i32 = 10;   // defined now, never fired in phase 1

#[derive(Default)]
pub struct TouchGest { /* private: tracked contacts (landing time, landing point, moved flag, broad flag,
    ever-pen flag, resting flag), the followed finger, the session (landing, fingers, class, pair by landing
    order, start spread and midpoint, pen-blocked flag), suppression, view, last outcome */ }
```

### 4.2 API

```rust
pub fn touchgest_frame(g: &mut TouchGest, f: &TgFrame, pen: &TgPen, out: &mut TgOut);
pub fn touchgest_cancel(g: &mut TouchGest, k_ms: u32, out: &mut TgOut);  // pen arrival, SYN_DROPPED: end all, suppress
pub fn touchgest_tick(g: &mut TouchGest, k_ms: u32, out: &mut TgOut);    // windows that expire with no new frame
pub fn touchgest_set_view(g: &mut TouchGest, v: TgView);
pub fn touchgest_busy(g: &TouchGest) -> bool;                            // any contact down (the key-scan gate)
pub fn touchgest_probe(g: &TouchGest, key: &str, out: &mut String) -> bool;  // "touch.*" keys
pub fn touchgest_reject_name(r: TgReject) -> &'static str;
```

Each call writes up to 8 events into `out` in order; the caller `q_push`es them as they are.

### 4.3 Behaviour (constants marked prov. are provisional; phase 3 freezes them from the traces)

1. **One finger: exactly today's stream.** TouchDown at once, TouchMove only when the position changes, TouchUp at the
   lift. No added latency, and `x, y, nx, ny` as today's `emit_touch` (input_evdev.rs:1025) computes them.
   - The followed finger is the first to land (today: the first slot found).
   - Single-finger suppression uses today's global pen window (`palm_window`, input_evdev.rs:1325):
     `pen.prox || k_ms - pen.left_ms < 1000`.
   - **Fix:** inside the window, an active finger ends with the CANCEL up, never a real-place TouchUp (today's real
     lift is at input_evdev.rs:1436-1441).
   - A contact with tool 2 cancels the active finger and suppresses until all lift (today input_evdev.rs:1361-1373).
   - A contact ever reported as tool 1 is never a finger.
2. **LANDING.**
   - Second contact within 150 ms (prov.) of the first:
     - the first finger ends with the CANCEL up (`value = TOUCH_F_CANCEL`, x = y = -10000.0);
     - `PinchBegin` goes out with `value = 1`, `(x, y)` = the earlier-landed finger and `(x2, y2)` = the later one
       (pairing by landing order, not slot order);
     - the session may become a swipe or a tap.
   - 150-400 ms: zoom or pan only, never a swipe or tap.
   - Over 400 ms: a palm, as today (input_evdev.rs:1385-1392): cancel, suppress until all lift.
   - A third contact rejects the session (`Three`; three-finger gestures come in phase 3), except during a committed
     ZOOM, which keeps its two fingers.
   - The finger count is fixed at the first of: any contact moving 3 mm, the end of the landing window, the first
     lift.
3. **COMMIT** (today's dead zone, editor.rs:3278-3281): spread change >= 10% or midpoint travel >= 70 px at 300 dpi
   (`ui_px(70.0)`). touchgest has no App, so add `pub fn touchgest_init(g: &mut TouchGest, dpi: f32)`, which converts
   every mm and px threshold once with `mm_to_px` (util.rs:173), and call it from `input_open` on both platforms.
   Classified once, at that frame:
   - **ZOOM** if `zoom_ok` and the spread change in px >= 0.6 x the travel.
   - Else **TRANSLATE** along the dominant axis:
     - if the view scrolls on that axis (`scrolls_v` for vertical, `scrolls_h` for horizontal), it is a **PAN** (D5,
       per axis, never per direction: a drag that starts at a scroll limit pans nowhere and never navigates);
     - more than 30 degrees off axis: PAN if either axis scrolls, else reject `Straight`;
     - otherwise it is a **SWIPE** candidate.
   - Nothing is emitted before the commit.
   - At the commit, the first `PinchMove` goes out with the class value (1 ZOOM, 2 PAN, 3 SWIPE), then one per frame.
   - Sessions that landed in the pen window (`pen.prox || k_ms - pen.left_ms < 1000`) never ZOOM or PAN (today no
     pinch starts in the window, input_evdev.rs:1385). They can still be a SWIPE or TAP under rule 6.
4. **FIRING** at the first lift (no multi-finger double taps, so nothing ever waits).
   - **SWIPE:**
     - axis travel >= 20 mm (236 px at 300 dpi);
     - off-axis <= tan 30 degrees of it;
     - each finger moved >= 12 mm with cosine >= 0.8 to the midpoint direction;
     - largest finger travel <= 2x the smallest;
     - spread change <= 25%;
     - <= 1000 ms from the first landing.

     Sliding back under 20 mm cancels it. The id comes from the direction (screen y grows downwards, so `G_2_DOWN`
     is a positive dy).
   - **TAP:** every contact lifts within 150 ms of the others, <= 300 ms from the first landing to the last lift, and
     no contact moves more than 5 mm.
   - Output on a fire: `PinchEnd` then `Gesture { value: id, x, y: start midpoint, x2, y2: end midpoint, t_ms, k_ms:
     the frame's }`, in the same `TgOut`.
   - Output on a reject: `PinchEnd` only, and `outcome = Rejected(reason)`.
   - After a fire or a reject, every contact stays suppressed until all have lifted, so a late finger never leaks a
     tap, a link follow or a corner toggle.
   - With `gestures_on` false, or `keyboard_open` true, SWIPE and TAP never fire (rejects `Off` / `Keyboard`). ZOOM
     and PAN still work when `zoom_ok`.
5. **RESTING CONTACTS** (grip thumb, writing palm): a contact still within 3 mm for 300 ms before the session's first
   finger, or flagged tool 2, is left out of the count and the landing rule, provided every gesture finger is >= 40
   mm from it. If it moves > 3 mm during the session, the session is rejected `Hand`.
6. **PEN NEAR** (multi-finger, phase 1's conservative rule):
   - pen touching, in proximity, or gone < 300 ms at any frame of the session: the session is pen-blocked. A running
     ZOOM or PAN ends now with `PinchEnd`, as today (input_evdev.rs:1374-1381), and the geometry is still tracked;
   - gone 300-1000 ms: a SWIPE or TAP is accepted only when every finger is >= 40 mm from `pen.last`.

   At the first lift, a pen-blocked session whose geometry passes rule 4 is rejected `Pen` (the editor shows the
   rate-limited toast, section 8.6). Other pen-blocked sessions reject on their geometry. Size guards: fingers < 8 mm
   or > 70 mm apart reject `Apart`. TOUCH_MAJOR only sets `TOUCH_F_BROAD` (log only; the T_palm guard waits for phase
   3). `pub fn touchgest_set_palm_major(g, t: i32)` sets T_palm. It defaults to `i32::MAX` in phase 1, so the bit
   never sets on the device. Unit tests set a small T_palm and check the bit at `T_palm + 1`, not at `T_palm`.
7. **`touchgest_tick`** resolves pending windows (the landing window, the first finger's 150 ms) when a still pair
   produces no frame. Call it after a touch drain that read nothing, and from the runner's `poll` when its queue is
   empty.

### 4.4 Probes (`touchgest_probe`)

| Key | Value |
|---|---|
| `touch.contacts` | contacts down now (`0`, `1`, `2`) |
| `touch.class` | `none`, `one`, `landing`, `zoom`, `pan`, `swipe`, `tap` (pending two-finger tap), `suppressed` |
| `touch.suppressed` | `1` while suppressed until all lift, else `0` |
| `touch.gesture` | the last fired gesture as gestures.ini names it (`2.down`, `2.up`, `2.left`, `2.right`, `2.tap`), empty before any |
| `touch.reject` | the last reject's name (`pen`, `apart`, `straight`, `short`, `direction`, `hand`, `spread`, `late`, `three`, `keyboard`, `off`, `slow`), empty before any; a fire clears it |

---------------------------------------------------------------------------------------------------------------

## 5. `input_evdev.rs` (Kindle only; compiled with `--features kindle`)

1. **Constants and state.**
   - Add `const ABS_MT_TOUCH_MAJOR: u16 = 0x30;`.
   - `TouchSt` (input_evdev.rs:363) gains `major: [i32; MAX_SLOTS]`, `pressure: [i32; MAX_SLOTS]`, `frame_us: u64`
     and `dropped: bool`.
   - `State` gains `tg: TouchGest`, `touch_mono: bool` and `trace: TouchTrace` (section 18.2).
   - At open, in the touch branch (input_evdev.rs:2159), set `self.touch_mono = mono` (the `mono` of input_evdev.rs:1989).
2. **`handle_touch`** (input_evdev.rs:1457):
   - read `ABS_MT_TOUCH_MAJOR` and `ABS_MT_PRESSURE` into the slot;
   - on `EV_SYN` + `SYN_DROPPED` (code 3), set `T.dropped`;
   - on `SYN_REPORT`, store `T.frame_us = ev_us(ev)` (input_evdev.rs:685), then if `T.dropped`, resync: re-read
     every slot with `eviocgmtslots` (input_evdev.rs:191, the pattern at 2833-2842 in `input_set_grab`), call
     `touchgest_cancel`, clear the flag; then call `touch_frame()`.
   - Keep `down_at` as today: the log reads it.
3. **`touch_frame`** (input_evdev.rs:1349) keeps its name. Its body becomes:
   - build `TgFrame { k_ms: pensync_ms(T.frame_us, touch_mono, now_ms()) }` from the live slots (id >= 0). Each
     contact's `nx, ny` = `norm(T.x[s], &self.touch.ax)`, `norm(T.y[s], &self.touch.ay)`, and `x, y` =
     `self.map(&self.touch_cal, nx, ny)`;
   - build `TgPen { prox: P.prox, touching: P.touching, left_ms: pen_left_ms, last }` (track `last` from the pen's
     contact samples, in `emit_pen`'s mapped coordinates: store it in State at PenDown/PenMove, which is the touch
     side reading a field, not a change to `emit_pen`'s output);
   - call `touchgest_frame`, set each output event's `t_ms = now_ms()` (today's read time) and its `k_ms` = the
     frame's, then `q_push` them in order;
   - log (section 18.1) and trace (section 18.2).

   `T.active`, `T.main`, `T.suppressed`, `T.suppressed_pinch`, `last_x/last_y` and `State.pinching` are replaced by
   touchgest's state. Keep only what the log and the busy flag need.
4. **Pen arrival** (`pen_frame`, input_evdev.rs:1062-1067): replace the real-place `emit_touch(TouchUp, ...)` with
   `touchgest_cancel(&mut self.tg, k, &mut out)` and push `out`. It ends the active finger with the CANCEL up, or a
   session with `PinchEnd`, then suppresses until all lift.
5. **`input_pump`** (input_evdev.rs:2579): after `read_pens(...)`, if the touch fd is readable (poll it in the same
   `pf` array, after the pens), call `drain(touch_fd, if grabbed { State::handle_touch } else { discard_input })`.
   Update the comment above `input_pump`: "Keys, touch, power..." becomes "Keys, power and orientation wait for
   input_poll". Touch events go into the queue behind what is there, in order.
6. **Busy flag** (input_evdev.rs:2630): `let busy = s.P.prox || s.P.touching || touchgest_busy(&s.tg);`. It counts
   every down contact, not just `T.active` / `pinching`.
7. **Tick:** in `input_poll`, after the touch drain, if no touch event was read this call and `touchgest_busy`, call
   `touchgest_tick(&mut s.tg, now_ms(), &mut out)` and push.
8. **`input_touch_context`**: `with(|s| touchgest_set_view(&mut s.tg, v))`. **`input_caps().touch_slots`**: from
   `eviocgabs(fd, ABS_MT_SLOT, ..)` at open.
9. **Open-time log.** Add a NEW line right after "input: touch %s (%s) x %d..%d y %d..%d"
   (input_evdev.rs:2169-2177):
   `qlog!("input: touch slots %d..%d, touch_major %d..%d r%d", ...)`.
   Do not add names to the existing "input: found ..." NAMES list (input_evdev.rs:2006): that would change an existing
   log line, which PORTING rule 1 keeps byte-exact (PLAN-V2 G3.2 says the same).
10. **`input_set_grab`** (input_evdev.rs:2767) resets `tg` with `touchgest_cancel` on regrab, before the slot resync.
    `T.suppressed = true` becomes the cancel's suppression.

---------------------------------------------------------------------------------------------------------------

## 6. The runner (`sim_sdl.rs`) and `docs/SIM.md`

Existing commands (`tap`, `touch`, `drag touch`, `pinch`, `pentap`) keep their exact event sequences. They inject
directly with `PinchMove.value` 0 and never pass through touchgest. The new commands feed `State.tg` (a `TouchGest`
in sim_sdl's `State`, sim_sdl.rs:79).

### 6.1 New commands (arms in `script_step_in`, sim_sdl.rs:965, before the unknown-command arm; `tap panel` and `pentap panel` go inside the existing `tap`/`pentap` arm, see below)

| Syntax | Meaning | Batch |
|---|---|---|
| `finger ID down X Y [MAJOR] [TOOL]`, `finger ID move X Y [MAJOR] [TOOL]`, `finger ID up X Y` | sets contact ID (1..16) in the pending frame: down adds it, move changes it, up removes it after this frame. MAJOR default 0, TOOL default 0 (0 finger, 1 pen, 2 palm). Coordinates are screen px, run through `make_ev` (sim_sdl.rs:322) for `x, y, nx, ny` | does not end a batch |
| `frame [MS]` | builds a `TgFrame` from the pending contacts at the frame clock, runs `touchgest_frame` with the runner's `TgPen`, queues its events, then advances the frame clock by MS (default 13) | does not end a batch |
| `swipe N X Y DX DY [STEPS] [SEP] [LANDGAP]` | N fingers (2 or 3) centred on (X, Y), SEP px apart (default 295, 25 mm) across the motion, landing LANDGAP ms apart (default 30), moving by (DX, DY) over STEPS frames (default 10) 20 ms apart, then lifting one frame apart (5 ms) | ends the batch |
| `tapn N X Y [SEP] [MS]` | N fingers in a horizontal row centred on (X, Y), SEP px apart (default 150), each clamped to the screen, land within 20 ms (one frame each, 10 ms apart), stay still for MS (default 80), lift within 20 ms. `tapn 2 100 100` puts the fingers at x 25 and 175, midpoint 100 | ends the batch |
| `tap panel NAME`, `pentap panel NAME` | a finger or pen tap at the centre of the editor panel's button NAME (section 10.3); an unknown name, or no editor, fails the line like a failed check: `expect failed (line N): no panel button NAME` | ends the batch |

**Frame clock.** The frame clock is monotonic. The first `finger`/`frame` line after a batch boundary sets it to
max(`now_ms()`, the last frame's time) (wrapping comparison), never back. Each `frame MS` adds MS, so a contact
sequence is timed by offsets as `drag` (100 ms steps, sim_sdl.rs:1162) and `pinch` (160 ms, sim_sdl.rs:1224) are,
even when an `expect` or a `wait` splits it while contacts are down. Events carry `t_ms = k_ms =` the frame's time.
There is no virtual app clock: holds and pauses still need real `wait` lines.

**Pen context.** The runner keeps `TgPen` from its own pen lines:
- `pen down/move` set prox and touching, and record `last`;
- `pen up` keeps prox (hovering);
- `pen hover` sets prox;
- `pen leave` clears prox and sets `left_ms` to that event's `t_ms`;
- `pentap` = down, up, then a leave stamped at the up (a nib tap lifted away; the pen event sequence it queues is
  unchanged, only the runner's `TgPen` records the leave), so a later gesture sees the pen gone.

Rule for scenarios: contact-level touch (`finger`, `swipe`, `tapn`) after any pen input waits out the pen window
first: `pen leave` if the pen is hovering (`pentap` leaves by itself), then `wait 1100` (rule 6 of 4.3 blocks
two-finger sessions for 300 ms after a leave and checks the distance to 1000 ms; single fingers are suppressed for
1000 ms).

The pen lines themselves are unchanged. When a pen line arrives while `tg` has contacts, call `touchgest_cancel` (the
Kindle's `pen_frame` arrival rule) before queueing the pen event.

**Tick.** In `poll` (sim_sdl.rs:1405), when the queue is empty and `touchgest_busy`, call `touchgest_tick` with the
frame clock (max(`now_ms()`, the last frame's time)), never a raw `now_ms()` that can be behind the last frame.

**`tap panel NAME` resolution.** The existing arm `else if a == b"tap" || a == b"pentap"` (sim_sdl.rs:1152) matches
first and would read `panel` as the float 0 (`scan_floats`), tapping (0, 0), the clock corner. So the test goes at the
top of that arm, before `scan_floats`: when the second word is `panel`, return the new step. `script_step_in` runs
inside the `State` borrow without the App. Add a third `Step` variant (sim_sdl.rs:953), `Step::Panel(FixedStr<32>,
bool)`, holding the name and whether it is a pen tap. On it, `script_step` (sim_sdl.rs:1302), which holds `Option<&mut
App>`, calls `editor_panel_point(a, name) -> Option<(f32, f32)>` (section 10.3) outside the borrow, then queues the
same events `tap`/`pentap` queue (sim_sdl.rs:1152-1161) and ends the batch.

**`main.longest_unpumped_ms`.** In sim_sdl's State, keep `last_read_ms` and `longest_unpumped`. At each entry to
`poll` and each `sim_pump` (sim_sdl.rs:1312):
- `gap = now_ms() - last_read_ms`; `longest_unpumped = max(...)`;
- `last_read_ms = now_ms()`, also set at `poll`'s exit.

Waits spent inside `poll` do not count. Nor do presents and file saves, which are not work that should pump: add
`pub fn work_pump_mark()` in util.rs beside `work_pump` (util.rs:452; a no-op unless a hook is set, as
`work_pump_set` does), call it at the end of `app_flush` and of `write_file_atomic` (util.rs:350), and have the runner
set `last_read_ms = now_ms()` in it. The value resets on read, like `pump.calls`. The headless binary is a debug
build (`[profile.dev] opt-level = 1`) on a shared host, so the probe's bound is 250 ms of host time: it catches a
missing pump (a whole page read or rendered unpumped), not the Kindle's ~100 ms budget, which the unit tests of the
pumped reader and renderer check by counting pumps (section 19).

### 6.2 New probes

| Key | Answered by | Value |
|---|---|---|
| `touch.contacts`, `touch.class`, `touch.suppressed`, `touch.gesture`, `touch.reject` | the runner, through `touchgest_probe` in `power_probe` (sim_sdl.rs:801) | section 4.4 |
| `main.longest_unpumped_ms` | the runner | the longest gap between two input reads or pump calls since it was last checked, presents and file saves excluded (resets on read): `ok` when under the runner's 250 ms host bound, else the gap in ms (`expect` has no numeric comparison) |
| `gestures.ACTION` | `app_probe` (app.rs:1526) | the touch gestures bound to ACTION, as gestures.ini names them, comma separated (`gestures.pages` is `touch.2.down`), empty for none |
| `settings.gestures`, `settings.landscape_fit`, `settings.hints`, `settings.link_areas` | `app_probe`'s `nums` table (app.rs:1549; it grows from 19 to 23) | as settings.ini stores them |
| `notebook.last_page` | `app_probe` | the open notebook's `last_page` + 1 (the page notebook.ini reopens on), 0 with none |
| `editor.scrolls` | `editor_probe` | `V H` (`1 0`): whether the page scrolls on each axis now |
| `editor.areas` | `editor_probe` | `0` while the eye is off, else how many area boxes it draws |
| `editor.bar` | `editor_probe` | `0`, `open` or `peek` |
| `editor.bar.entries` | `editor_probe` | the bar's entry count (0 when closed) |
| `editor.bar.N` | `editor_probe` | entry N (from 1): `p<page number>` then ` x<count>` when the count > 1 (`p3 x2`), `index`, or `missing`; empty past the last |
| `editor.bar.peek` | `editor_probe` | the page number shown by a peek, `0` when not peeking |
| `editor.bar.rect` | `editor_probe` | the bar's card `x y w h`, empty when closed |
| `editor.back` | `editor_probe` | `DEPTH TOP` (`2 3`), the back list's depth and its top as a page number, `0 0` empty |
| `editor.canvas_cache` | `editor_probe` | `HITS MISSES ENTRIES` since the bar opened |
| `editor.find` | `editor_probe` | `0` or `go` |
| `editor.find.rows`, `editor.find.N` | `editor_probe` | the Find sheet's row count, and row N's label as drawn |
| `editor.panel_at.X.Y` | `editor_probe` | the name (section 10.3) of the panel button at (X, Y), empty for none, after settling a stale panel as a tap would (10.3): used to check every raw-tap conversion |
| `editor.modal` (existing) | `editor_probe` (editor.rs:3799) | gains two values, `links` (the bar in front) and `find` (the Find sheet in front), placed first in the order. Every existing value keeps its meaning |

### 6.3 SIM.md edits

- The command table (docs/SIM.md:51-80) gets one row per command above. Add batching and timing prose after the
  batching paragraph (docs/SIM.md:89-98):
  - the frame clock and offsets;
  - the pen context;
  - "legacy `tap`, `touch`, `drag` and `pinch` inject events directly and do not pass through touchgest".
- The Checks table (docs/SIM.md:161-235) gets every key above.
- The header list gets `# rust-only`: "a scenario with a `# rust-only` line is skipped when the rule runs against the
  C build (`make c-check`)".
- The "Coordinates" paragraph gets the panel's names for `tap panel NAME`.
- The selftest's new lines go in two NEW files, so the C run never sees them (6.4): `tests/selftest/pass-rust.qs`
  and `tests/selftest/fail-rust.qs`, run by the scenarios rule exactly as pass.qs and fail.qs are (the pass one must
  exit 0, the fail one must exit 3 with the listed lines failed), and skipped under `SCEN_SKIP_RUST_ONLY`. pass.qs
  and fail.qs are unchanged.
- `tests/selftest/pass-rust.qs` gets one use of each new probe family:
  - `expect touch.contacts is 0`;
  - `expect main.longest_unpumped_ms not -1`;
  - `expect gestures.pages is touch.2.down`;
  - `expect settings.landscape_fit is 1`;
  - `expect notebook.last_page is 0`;
  - and, once a notebook is open there, `expect editor.bar is 0`, `expect editor.find is 0`,
    `expect editor.scrolls is 0 0`; then a `pen line` (so Undo is enabled), `expect editor.panel_at.1768.594 is
    undo`, `tap panel undo` and `expect editor.strokes is 0`.
- `tests/selftest/fail-rust.qs` gets a failing line per family (for example `expect touch.contacts is 9`,
  `expect editor.bar is open`) and `pentap panel nosuchbutton`, which must fail by line number. It is `pentap`, not
  `tap`: the rule's `want` list leaves out every line whose first word is `wait` or `tap` (Makefile:189), so a
  failing `tap panel` line would be reported but not wanted, and the selftest would fail.

### 6.4 Makefile: `# rust-only`

Comparing `$(BUILD)` with `$(CBUILD)` does not work: `CBUILD ?= $(BUILD)/c` is recursively expanded (Makefile:19-21),
so inside c-check's sub-make (`$(MAKE) BUILD=$(CBUILD) ...`, Makefile:581) `CBUILD` is build/c/c and never equals
`BUILD`. Use an explicit flag instead:

- c-check's two lines become `$(MAKE) BUILD=$(CBUILD) SCEN_SKIP_RUST_ONLY=1 -o $(CBUILD)/host/grimoire-headless
  scenarios` and the same for `l10n-check`.
- In the scenarios rule (Makefile:158), at the top of the loop body:

  ```
  if [ -n "$(SCEN_SKIP_RUST_ONLY)" ] && grep -q '^# rust-only' $$s; then echo "== $$n (rust-only: skipped)"; continue; fi;
  ```

- After the existing selftest pass and fail runs, the same two runs for `pass-rust.qs` and `fail-rust.qs` (their own
  `want` list from fail-rust.qs, computed as for fail.qs), inside `if [ -z "$(SCEN_SKIP_RUST_ONLY)" ]`.
- In `l10n-check`, run `$(L10N_SCENARIO)` (y_all_screens.qs) only when the flag is empty: from phase 1 it visits the
  new screens and taps the moved Buttons and Display coordinates, so it is rust-only. The msgfmt checks and
  tests/l10n/templates.qs still run under the flag.

Every new scenario carries `# rust-only`. Every existing scenario this phase changes so that it needs the Rust build
(new commands, new behaviour, moved coordinates) gets the header too, in the same commit (the cost is in section 22).
`# settings: landscape_fit=0` alone does not need it: the C build carries the unknown key.

---------------------------------------------------------------------------------------------------------------

## 7. `gesturemap.rs` (new, pure) and gestures.ini (read only in phase 1)

Register it like touchgest (foundations and prelude).

```rust
#[derive(Clone, Copy, PartialEq, Eq)] pub enum TouchKind { Up, Down, Left, Right, Tap }
#[derive(Clone, Copy, PartialEq, Eq)] pub enum PenBtn { Button, Button2, Eraser }
#[derive(Clone, Copy, PartialEq, Eq)] pub enum PenKind { Tap, Double, Triple, Hold, Drag }
#[derive(Clone, Copy, PartialEq, Eq)] pub enum GestureKey { Touch { fingers: u8, kind: TouchKind }, Pen { button: PenBtn, kind: PenKind } }
#[derive(Clone, Default)] pub struct GestureMap { /* touch slots [2..=3][5] -> action (or NOTHING), pen slots (parsed,
    unused), shape lines and unknown lines carried verbatim (each <= 1 KiB, 16 KiB in all), refused count */ }

pub fn gesturemap_defaults(m: &mut GestureMap);
pub fn gesturemap_parse(m: &mut GestureMap, text: &[u8]) -> i32;          // pure: refused line count
pub fn gesturemap_load(m: &mut GestureMap, path: &str) -> i32;            // refused count, or -1 unreadable
pub fn gesturemap_action(m: &GestureMap, k: GestureKey) -> i32;           // 0 = nothing
pub fn gesturemap_key_for(m: &GestureMap, action: i32, src: ActionSrc) -> Option<GestureKey>;
pub fn gesturemap_mirror(k: GestureKey) -> GestureKey;                     // up<->down, left<->right, tap stays
pub fn gesturemap_key_of_id(id: i32) -> Option<GestureKey>;                // touchgest G_* -> key
pub fn gesturemap_key_name(k: GestureKey, out: &mut String);               // "touch.2.down"
```

- **Built-ins**, never written to a file: `touch.2.down = pages`, `touch.2.up = links`, `touch.2.left = next-page`,
  `touch.2.right = previous-page`; `pen.Button.triple = clear-page` and `pen.Button.hold = clear-page` (parsed and
  held; the pen stays on `pen_button`, editor.rs:2038, until phase 6). Two- and three-finger taps and every
  `touch.1.*` are unbound.
- **File:** `<store_root()>/gestures.ini` (store.rs:694), shortcuts.ini's grammar:
  - '#' and ';' comments, trimmed `key=value`, a later duplicate wins, keys case-insensitive;
  - read with the bounded read of `shortcuts_load` (shortcuts.rs:280-305: stat, 64 KiB cap, NUL cut). Factor that
    read into a `pub(crate) fn ini_read_bounded(path, cap) -> Result<Option<Vec<u8>>, ()>` in shortcuts.rs with
    byte-identical behaviour for `shortcuts_load`, and use it here.
- **Keys:** `touch.<2|3>.<up|down|left|right|tap>`. Refused (counted): `touch.1.*` (reserved), bad keys, unknown
  values. Values: an action file name (`shortcut_action`, shortcuts.rs:182) or `nothing`.
- **Carried, not acted on, in phase 1:** `pen.<Button|Button2|Eraser>.<tap|double|triple|hold|drag>`, `shape.<action>`
  lines (format 1 is checked by phase 3; phase 1 checks only that the value starts `1,` and is at most 1 KiB) and
  well-formed lines with unknown actions. Nothing is written in phase 1.
- **Loading** (reading gestures.ini in phase 1 is a decision recorded in PLAN-V2 section 19: it lets a hand edit turn
  a built-in off before the phase 3 sheet exists): in `app_init` right after `shortcuts_load_all` (app.rs:471), and
  in `reload_shortcuts` (library.rs:944). Store it in a new `App.gestures: GestureMap`. An unreadable file means the built-ins only. A
  refused count > 0 logs once: `qlog!("gestures: %d line(s) of gestures.ini not used", n)` (new line).
- `ActionSrc` lives in shortcuts.rs (section 8.1). `gesturemap_key_for(.., ActionSrc::Touch)` returns the first touch
  slot bound to the action, in the order 2.down, 2.up, 2.left, 2.right, 2.tap.

---------------------------------------------------------------------------------------------------------------

## 8. Dispatch in the editor (`editor.rs`, `shortcuts.rs`)

### 8.1 ActionSrc and the new entry point

- In shortcuts.rs: `#[derive(Clone, Copy, PartialEq, Eq, Debug)] pub enum ActionSrc { Key, Pad, Touch, Shape, Pen }`.
  Phase 1 uses Key, Pad and Touch.
- In editor.rs: `pub fn editor_shortcut_at(a: &mut App, action: i32, x: f32, y: f32, src: ActionSrc) -> bool`. It
  holds today's body of `editor_shortcut` (editor.rs:2816-2947) plus the phase 1 changes below.
  `pub fn editor_shortcut(a, action)` stays as the wrapper (`src = Key`, anchored at the screen centre). Export
  `editor_shortcut_at` in app.rs's `pub use crate::editor::{...}` list (app.rs:140).
- In `app_event`'s Key branch (app.rs:651): call `editor_shortcut_at(a, action, a.w/2, a.h/2, if ev.gamepad { Pad }
  else { Key })` in place of `editor_shortcut`, with identical behaviour.

### 8.2 `editor_gesture(a, ev)` (new, private)

At the top of `editor_event` (editor.rs:3345), after the `open` check: `if ev.type_ == InType::Gesture {
editor_gesture(a, ev); return; }`. This runs before the pinch branch, so the Pages view also gets its closes.

```text
editor_gesture(a, ev):
  if !s.open || a keyboard is open (s.pv.kb.open || s.find.open) -> return
  key = gesturemap_key_of_id(ev.value)?
  front = front_menu(s)                                  // extended in 11.3: SHORTCUT_LINKS, SHORTCUT_GO_TO_PAGE
  if front == SHORTCUT_LINKS && key == touch.2.down:      // the bar's fixed cancel (D2), whatever is bound
     action = SHORTCUT_LINKS
  else if front != 0:
     owns = the touch keys bound to front's action (for the Pages view: those of SHORTCUT_PAGES and SHORTCUT_INDEX)
     if key is in owns, or gesturemap_mirror(key) is in owns -> action = front    // the mirror rule (G6.2)
     else action = gesturemap_action(&a.gestures, key)
  else action = gesturemap_action(&a.gestures, key)
  if action == 0 -> return
  editor_shortcut_at(a, action, (ev.x2), (ev.y2), ActionSrc::Touch)
```

`editor_shortcut`'s menu branch (editor.rs:2832-2852) already treats "the front menu's own action" as "close it", so
this gives:
- two fingers up close the Pages view;
- two fingers down close the bar back to its origin (D2), even when gestures.ini has `touch.2.up=nothing` or binds
  `touch.2.down` to something else: without the fixed rule, 2-down would fall through to `pages`, and 13.10's
  "another menu: commit, then swap" would commit the peeked page, the opposite of D2 (scenario `g_gest_1b_unbound`);
- another menu's gesture swaps menus.

### 8.3 Page turns never add a page (G6.3)

In `editor_shortcut_at`, for `SHORTCUT_NEXT_PAGE` / `SHORTCUT_PREV_PAGE` with `src` Touch (later Shape and Pen):
- next on the last page: `app_toast(a, tr("This is the last page."), 1500)` and return true;
- previous on the first page: `app_toast(a, tr("This is the first page."), 1500)` and return true;
- otherwise `panel_act` as today (editor.rs:2897-2899).

Key and Pad keep today's `panel_act(P_NEXT)`, which adds a page on the last one.

### 8.4 Two fingers: classes, per-axis pan, pinch zoom off (D5)

Changes in `pinch` (editor.rs:3230):

- `PinchBegin`:
  - Clearing `Owner::Link` (editor.rs:3236-3238) also clears `Owner::Bar` and `Owner::Outside` (section 13). A pending
    chip press is un-inverted with `RefreshMode::Fast`, and a pending commit is dropped.
  - The cancelled panel press (editor.rs:3239-3250) is restored with `RefreshMode::Fast`, not `Ui` (brief section
    20.4 lists this deliberate change).
  - Gate: for `ev.value == 1` (classified), keep `owner != None || paper_open || mb.open || ts.open` and drop
    `!pinch_zoom`: a PAN pans with pinch zoom off. For `ev.value == 0` keep today's line exactly (k_pinch_gate and
    every runner `pinch` scenario stay unchanged).
  - While the bar or Find is open, a classified PinchBegin does not start `pinching` (the bar does not pan the page).
- `PinchMove`:
  - `value 0`: today's code, line for line.
  - `value 1` (ZOOM): skip the 10% / 70 px dead zone (touchgest already committed), re-anchor at this first move
    (`pinch_d0`, `pinch_z0`, `pinch_px/py` as the commit branch at editor.rs:3282-3289), then zoom as today, clamped
    to `zoom_max` (section 9).
  - `value 2` (PAN): re-anchor on the first move. Then `zoom = pinch_z0` (held), `ox = pinch_px - mx / zoom`,
    `oy = pinch_py - my / zoom`, `view_clamp`, paint `Fast` every 180 ms of `t_ms`.
  - `value 3` (SWIPE): ignore.
  - Set `pinch_committed` only for 1 and 2.
- `PinchEnd`: as today (`Ui` if committed).
- **The pan hint** (`hints` bit 6). At a PinchEnd that ended a PAN of a classified session, while `touch.2.up` or
  `touch.2.down` is bound and the bit is clear: toast for 4000 ms "Here two fingers move the page. Open Pages and
  Links from the panel." and set the bit (`settings_save`).

**View context push.** Add `fn touch_context(a: &mut App)`:

```rust
let pans = s.open && !sheet_open(s);   // sheet_open covers the Pages view, tools, Paper, music, Display, templates,
                                       // and (11.3) the bar and Find: none of them pans the page
TgView {
    scrolls_v: pans && ah / zoom < pgh - 0.5,
    scrolls_h: pans && aw / zoom < pgw - 0.5,
    zoom_ok: a.settings.pinch_zoom && !(s.paper_open || s.mb.open || s.ts.open || s.lb.open || s.find.open),
    gestures_on: a.settings.gestures,
    record: false,
    keyboard_open: s.pv.kb.open || s.find.open,
}
```

With a surface in front that does not pan the page, a two-finger translate is therefore always a SWIPE candidate:
pinches are not routed to the Pages view (editor.rs:3357-3360) and the bar does not pan, so a PAN there would move
nothing and the mirror rule and the bar's fixed cancel would never fire zoomed in (`g_gest_2_zoomed` checks both).

It calls `input_touch_context`. Call it at the end of:
- `view_clamp` (editor.rs:848);
- `editor_event`;
- `editor_shortcut_at`;
- `editor_tick` (cheap: it stores a small struct);
- `editor_enter` (editor.rs:3166).

`editor_leave` (editor.rs:3309) pushes `TgView { gestures_on, ..all false }`. The probe `editor.scrolls` prints the two
booleans.

### 8.5 Feedback

Nothing is drawn while two fingers move, except PAN/ZOOM's `Fast` repaint (at most every 180 ms of `t_ms`, as
today) and `Ui` at PinchEnd. The fired action's own refresh is the feedback:
- Pages view: GC16 (`pages_show`, editor.rs:2096);
- the bar: GL16;
- a page turn: GC16 through `go_to_page` (editor.rs:1302).

### 8.6 The pen toast

When a `PinchEnd` arrives with touchgest's last outcome `Rejected(Pen)`, the editor toasts "Move the pen away, then
swipe." for 2000 ms, at most once per 10 minutes (memory `EditorState.pen_toast_at`).
- Phase 1 delivery (brief decision): touchgest sets `PinchEnd.value = 1` for a pen reject (0 otherwise; existing
  `PinchEnd` readers ignore `value`).
- Write it with `t_ms`, and never while the pen is down.

---------------------------------------------------------------------------------------------------------------

## 9. One finger on the page, the corner, and the pen

### 9.1 `Owner::Link` (editor.rs:3436-3441 down, 3471-3480 up)

The owner and its name stay. At the lift, decide in this order:

1. `ev.value & TOUCH_F_CANCEL != 0`: end silently. No follow, no corner. The off-screen point already marks it moved
   through `press_move`.
2. `corner_tap(a, x0, y0, link_press.moved, false)`: the corner only (feature B toggles the clock). Nothing else
   happens.
3. Unmoved and not held: `link_follow` (editor.rs:2419) as today. A follow that changes page pushes the origin on the
   back list (section 13.8).
4. The hold stays today's `link_hold` from `editor_tick` (editor.rs:3651).
5. Anything else: nothing.

**The pen wins** (G3.3). A `PenDown` that arrives while `s.owner == Owner::Link` ends the finger session silently
(`owner = None`, no follow) and starts the stroke. Today this case is dropped by "one gesture at a time"
(editor.rs:3376-3379). Change only that case (`pen && s.owner == Owner::Link`), and run `make scenarios` to confirm
that no scenario relied on the drop.

### 9.2 Every finger-tap consumer added in phase 1 checks the corner first

| Consumer | Where |
|---|---|
| `Owner::Link` lift, before `link_follow` | 9.1 |
| The Links bar's "tap the page to stay" commit and its swallowed tap outside when not peeking (`Owner::Outside` lift) | 13.6 |
| The Find sheet's dismissal (tap outside) | 14 |

Further rules:
- The corner is one finger only: a two-finger tap whose midpoint is in the zone is `touch.2.tap`.
- The first finger of a two-finger swipe ends with the cancel up, so it never toggles the clock.
- A gesture fire suppresses all contacts until they lift, so a late finger never toggles it either.
- The pen is never a corner tap.
- Layout: the bar is at the bottom and never meets the zone. The eye's tags move right of the zone (section 12). The
  Find sheet never reaches above y = z + 8 (section 14).

---------------------------------------------------------------------------------------------------------------

## 10. Sideways, show the whole page (`landscape_fit`, D5) and the panel names

### 10.1 The view (editor.rs)

- New pure helper:

  ```rust
  fn view_rest(aw: i32, ah: i32, pgw: i32, pgh: i32, fit: bool) -> (f32 /*zoom*/, f32 /*zoom_min*/, f32 /*zoom_max*/)
  ```

  - `zoom_min = (aw/pgw).min(ah/pgh)` (today's expression, editor.rs:878).
  - `zoom = if fit && aw > ah { zoom_min } else { aw as f32 / pgw as f32 }`.
  - `zoom_max = (aw as f32 / pgw as f32) * 4.0`: the same expression and order as today's `zoom_default * 4.0`
    (editor.rs:851, 3291), so it is bit-identical when off or upright.
- `EditorState` (editor.rs:99) gains `zoom_max: f32` and `rest_ox: f32`.
- `view_reset` (editor.rs:875) uses `view_rest` with `a.settings.landscape_fit`, sets `zoom_default = zoom`, and at
  its end (after `view_clamp`) sets `s.rest_ox = s.ox`.
- `view_clamp` (editor.rs:851) and `pinch` (editor.rs:3291) clamp to `s.zoom_max` instead of `s.zoom_default * 4.0`.
  `editor_init` (editor.rs:3117) sets `zoom_max = 4.0` and `rest_ox = 0.0`.
- `zoomed` (editor.rs:911): `(s.zoom - s.zoom_default).abs() > 0.01 || s.ox != s.rest_ox || s.oy > 0.5`. This is
  today's test exactly whenever `rest_ox == 0.0`. With the whole page centred sideways, `view_clamp` sets
  `ox = -(vw - pgw) / 2` (negative), and today's test would call the resting view zoomed.
- The `editor.zoom` probe (`zoom / zoom_default`, editor.rs:3768) reads 1.00 at the whole page. SIM.md's
  `editor.zoom` row says "1.00 is the usual view: the page's width upright, the whole page sideways while
  `landscape_fit` is on".
- Rotation: `editor_enter` already calls `view_reset` on a size change (editor.rs:3175-3177), so a turn lands on the
  whole page with the rotation's usual GC16.
- Toggling (action or Display row):
  - in the editor, sideways: `view_reset` + `compose_all(a, Full)`, even if pinched in;
  - upright: only the toast;
  - from the Library-hosted Display sheet: add `pub fn editor_view_stale(a: &mut App)` (sets `s.view_w = 0` when
    `a.ed` exists), so the next `editor_enter` resets the view.

### 10.2 Display sheet row (`dispmenu.rs`)

- New hit `const D_FIT: i32 = 8;` and `DmResult::Refit = 4` (dispmenu.rs:10).
- Draw a `ui_checkbox` row "Sideways, show the whole page" after the Invert row (dispmenu.rs:162-165). `row_h`
  (dispmenu.rs:74) and `card_rect` (dispmenu.rs:81) count 7 rows (8 with a page): `rows = 7 + page`. It stays within
  the 100-140 px clamp (sideways: (1860 - 462) / 8 = 174, clamped to 140).
- `dispmenu_pointer` on `D_FIT`: flip `a.settings.landscape_fit`, `settings_save`, redraw the sheet, return `Refit`.
  - Editor host (editor.rs:3544 branch): `Refit` gives the toggle behaviour above, then the toast.
  - Library host (library.rs:2443 branch): `editor_view_stale(a)`, redraw `Ui`, toast.
- Toasts (2500 ms):
  - on: "Sideways shows the whole page.";
  - off: "Sideways fills the width.";
  - upright: "Applies when the Kindle is sideways.".
- Hint (`hints` bit 8): the first sideways turn with the setting on, for an upgrading owner only (section 11.2):
  "Sideways now shows the whole page, so two-finger swipes work. Pinch to zoom in; Display changes this." (6000 ms).

### 10.3 Panel button names (for `tap panel NAME` and `editor.panel_at`)

New `pub fn editor_panel_point(a: &mut App, name: &str) -> Option<(f32, f32)>` and a private `fn panel_name(h: &Hit)
-> String`. Both read `s.hits` (filled by `draw_panel`, editor.rs:512). The hits go stale after a stroke until the
settle (`s.panel_stale = true`, editor.rs:1781), and a human tap settles first (editor.rs:3425-3429), so
`editor_panel_point` and the `editor.panel_at` probe (`editor_probe` takes `&mut App`) call `settle_now(a)` first when
`s.panel_stale`; otherwise Undo right after a `pen line` would have no hit:

| Name | Hit |
|---|---|
| `grip`, `collapse`, `expand` | `P_GRIP`, `P_COLLAPSE`, `P_EXPAND` |
| `tool-picker` | `P_TOOLPICK` (the tool button) |
| `eraser` | `P_TOOL` with arg `TOOL_ERASER` |
| `tool:NAME` | `P_TOOL` with that pen's arg (a pinned pen), NAME as `editor.tool` names tools |
| `undo`, `redo`, `clear` | `P_UNDO`, `P_REDO`, `P_CLEAR` |
| `previous`, `pages`, `next` | `P_PREV`, `P_PAGES` (the page chip), `P_NEXT` |
| `links` | `P_LINKS` (new, section 15) |
| `paper`, `display`, `pen-feel`, `library` | `P_PAPER`, `P_DISPLAY`, `P_PENSET`, `P_LIBRARY` |

The point is the hit rect's centre. A disabled button has no hit, so `undo` with nothing to undo fails the line (as a
human tap would do nothing); a check that expects `undo` draws a stroke first.

---------------------------------------------------------------------------------------------------------------

## 11. Settings keys, hints, actions

### 11.1 settings.ini (store.rs)

| Key | Type | Default | Field | Phase 1 use |
|---|---|---|---|---|
| `gestures` | 0/1 | 1 | `Settings.gestures: bool` | master switch for two-finger gestures (`TgView.gestures_on`); off: pinch and pan still work |
| `landscape_fit` | 0/1 | 1 | `Settings.landscape_fit: bool` | section 10 |
| `hints` | int bitmask, 0..2147483647 | 0 | `Settings.hints: i32` | one-time hints, section 11.2 |
| `link_areas` | 0/1 | 0 | `Settings.link_areas: bool` | the eye, section 12 |

- Fields go in `Settings` (store.rs:58). Defaults go in `settings_default` (store.rs:2451): `gestures = true`,
  `landscape_fit = true`, `hints = 0`, `link_areas = false`.
- Arms in `settings_cb` (store.rs:2645): `keep_bool` (store.rs:2639) for the three booleans, and
  `keep_int("hints", v, 0, i32::MAX)`.
- In `settings_save` (store.rs:2848), after the `language` line (store.rs:2902), in this order: `put_int` for
  `gestures`, `landscape_fit`, `hints`, `link_areas`. Move the comment "(Last of the known keys: the newest...)" to
  apply to the block. Every existing line keeps its bytes and order. Older builds carry the four lines as unknown.
- All four keys are undotted (dotted heads are calibration or tool keys, store.rs:2646-2683, and test_probe.rs checks
  this).
- FORMAT.md's `settings.ini` section (docs/FORMAT.md:763) gets the four rows and a "Moving between versions" note
  (older versions keep them as unknown lines).

### 11.2 Hints (bit numbers are PLAN-V2 13.4's)

- bit 0: the first-open tip;
- bit 2: the eye tip;
- bits 3-4: a counter 0..3 of bar header hints shown;
- bit 6: the pan hint;
- bit 8: the sideways hint.

Bits 1, 5 and 7 belong to later phases: never touch them.

**Who sees the one-time toasts (bits 0 and 8).** Brief decision, needed because every scenario starts from a fresh or
header-made settings.ini. They show only for an **upgrading owner**: `settings_load` (store.rs:2777) read an existing
file whose `settings_version` was exactly 3 (`SETTINGS_VERSION`, store.rs:53) and which had no `gestures` key.
- Record this in a memory-only `Settings.upgraded: bool` (never saved), set in `settings_load` before
  `settings_upgrade`.
- A fresh install has no file, so no toast. A scenario's `# settings:` file has no `settings_version`, so no toast. A
  `# root:` copy saved by this build has `gestures`, so no toast.
- Scenarios that test the tips write `# settings: settings_version=3`.
- Do NOT bump `SETTINGS_VERSION`: it would change the `settings_version=` line in every file.

Bits 2, 3-4 and 6 are triggered by new actions only (the eye, the bar, a classified pan), so they need no gate.

- **First-open tip (bit 0):** the first time a notebook opens with `upgraded` and the bit clear, after
  `editor_open_notebook`'s first compose, a two-line toast for 6000 ms: "New: two fingers up show this page's links.
  Two fingers down show all pages."

### 11.3 Actions (`shortcuts.rs`, `library.rs`, `editor.rs`)

- New ids in shortcuts.rs, after `SHORTCUT_NAME_PAGE = 46` (shortcuts.rs:46):
  - `SHORTCUT_LINKS = 47`;
  - `SHORTCUT_GO_TO_PAGE = 48`;
  - `SHORTCUT_GO_BACK = 49`;
  - `SHORTCUT_LINK_AREAS = 50`;
  - `SHORTCUT_LANDSCAPE_FIT = 51`;
  - `SHORTCUT_LAST = 51`.
- `K_ACTIONS` (shortcuts.rs:162) grows from 27 to 32: `("links", ..)`, `("go-to-page", ..)`, `("go-back", ..)`,
  `("link-areas", ..)`, `("landscape-fit", ..)`.
- Kinds:
  - `links`, `go-to-page`: MENU;
  - `go-back`: one shot;
  - `link-areas`, `landscape-fit`: toggles.
- `menu_action` (editor.rs:2735) adds `SHORTCUT_LINKS` and `SHORTCUT_GO_TO_PAGE`.
- `front_menu` (editor.rs:2718) returns them while the bar or Find is in front. Order: Find first, then the bar, then
  the existing order.
- `close_front` (editor.rs:2748) closes Find (GL16 of its rect) or the bar (cancel to the origin, section 13.7).
- `open_menu` (editor.rs:2782) opens them (`linkbar_open` / `findsheet_open`).
- `sheet_open` (editor.rs:2032) adds `|| s.lb.open || s.find.open`, so every non-menu shortcut is refused while either
  is up.
- In `editor_shortcut_at`'s non-menu branch:
  - `SHORTCUT_GO_BACK`: while the bar is up, it is the bar's Cancel (section 13.7). Handle it BEFORE the
    `sheet_open` refusal (editor.rs:2874), which would otherwise refuse it. Otherwise pop the back list with
    `go_to_page`. Empty: toast "Nothing to go back to." for 2000 ms.
  - `SHORTCUT_LINK_AREAS`: flip, save, redraw (section 12).
  - `SHORTCUT_LANDSCAPE_FIT`: section 10.1.
- library.rs Buttons tables:
  - `K_PAGES` (library.rs:762) gets `lr(None, [SHORTCUT_LANDSCAPE_FIT, 0], IconKind::Expand)` and grows from 8 to 9
    rows;
  - a new `static K_INDEX: [LearnRow; 4]` with `links` (IconKind::Link), `go-to-page` (IconKind::Search, new),
    `go-back` (IconKind::Back) and `link-areas` (IconKind::Eye, new);
  - `N_GROUPS` (library.rs:793) goes from 4 to 5, with the group
    `Group { label: n_("Index..."), title: n_("Index"), icon: IconKind::Link, probe: "index", rows: &K_INDEX }`
    after Pages.
  - `action_label` (library.rs:829) gets the five labels (section 16). `library.buttons_group` (SIM.md) gains the
    value `index`.

  The Buttons list grows from 6 to 7 rows, and its `draw_menu` card is centred at (ah - h) / 3 (library.rs:583), so
  the rows move. Update the raw taps of the Buttons sheet in v_controller.qs, v_controller_forget.qs,
  v_controller_switch.qs, v_controller_triggers.qs and y_all_screens.qs. Check each with `library.buttons_group` and
  `library.learning`, and add `# rust-only` (section 6.4).

---------------------------------------------------------------------------------------------------------------

## 12. The eye: link areas over the page (`canvas.rs`, `editor.rs`)

- **New primitive** in canvas.rs: `pub fn canvas_dash_rect(c: &mut Canvas, r: Rect, thick: i32, on: i32, off: i32,
  halo: i32)`.
  - It draws a white halo band of `halo` px on both sides of each edge, then black dashes of `thick` px along the four
    edges.
  - Each edge starts at a corner with an "on" run, so the corners are solid.
  - It respects `c.clip` (as `canvas_fill`, canvas.rs:42) and uses only the gray values 0 and 255 (1-bit safe).
- **Box cache** (memory only):

  ```rust
  struct LinkBoxes {
      n: usize,
      link: [NavLink; 64],
      area: [Rect; 64],   // page px, padded
      entry: [u8; 64],    // its bar entry number, 1..
      stale: bool,
  }
  ```

  - Rust derives `Default` for arrays only up to 32 elements, and `EditorState` derives `Default` (editor.rs:97-98),
    so write `impl Default for LinkBoxes` by hand (the same for `LinkBar`'s 64 entries, 13.1).
  - It lives in `EditorState.boxes` and is filled lazily by `boxes_get(a)` with `nav_link_areas` (nav.rs:663) at
    `min_px = NAV_LINK_MIN_MM * dpi / 25.4` (exactly `link_under`'s, editor.rs:2410). The eye promises what
    `nav_link_at` hits.
  - Mark it stale on: page change (`go_to_page`, the bar commit), `draw_end` (editor.rs:1720), `erase_end`
    (editor.rs:1975), `undo_redo` (editor.rs:2071), `clear_page` (editor.rs:2010), `link_to` (editor.rs:2293), the
    Unlink branch of `pages_event`, and notebook open/close.
  - Rebuild only when the eye composes or the bar opens: `nav_link_areas` walks every stroke per link. A tap still
    uses `nav_link_at` as today.
  - Entry numbers come from the bar's grouping (section 13.2), computed by the same function, even when the bar is
    closed.
- **Drawing.** In `compose` (editor.rs:1109), after the page blit and before the panel. When `a.settings.link_areas`
  and the page is not peeked, for each cached box whose screen rect (`page_to_screen`, editor.rs:893) meets `r`:
  - `canvas_dash_rect(&mut a.screen, sr, ui_px(4.0), ui_px(16.0), ui_px(12.0), ui_px(2.0))`;
  - then a 52 x 52 white tag with a 3 px black frame at the box's top-left (kept inside the screen; if its rect meets
    `corner_zone(a)`, moved right to x = z + 8) holding the entry number, centred, 34 px bold
    (`ui_small_size()`, `FontFace::Bold`).

  At most 64 rect tests per compose. Never drawn into `page_canvas`, so never in thumbnails, PDF, share page or the
  sleep image. The live-ink path recomposes through `compose` and reuses the cache.
- **Toggle** (the bar's Areas button or the `link-areas` action): flip `settings.link_areas`, `settings_save`, recompose
  the button and the union of the boxes.
  - Refresh: `app_invalidate(button, Ui)`; `app_flush(a)`; then `app_invalidate(union, Ui)`. When the button and the
    union are more than 600 px apart, flush the union separately too.
  - No toast. The first time, with bit 2 clear: toast "Dotted boxes show where a finger tap follows a link. They never
    print." (4000 ms) and set bit 2.
- **Probe** `editor.areas`.

---------------------------------------------------------------------------------------------------------------

## 13. The Links bar (`linkbar.rs`, new, editor-hosted like `dispmenu.rs`)

### 13.1 Types and API

```rust
#[derive(Clone, Copy, Default, PartialEq)]
pub enum LbResult { #[default] None, Peek(i32), Commit, Cancel, Back, NewLink, GoTo, Eye, Index, Pager, Changed }

#[derive(Clone, Copy, Default)]
pub struct LbEntry {
    pub target: FixedStr<NAV_TARGET_LEN>,   // "p<page id>" or "index" or another version's
    pub page: i32,                           // page index, NAV_TO_INDEX (-2) or -1 missing (nav_link_page)
    pub count: i32,                          // links on the page to it (D1: shown once, xN)
    pub first_box: Rect,                     // page px, for reading order
}

#[derive(Default)]
pub struct LinkBar {
    pub open: bool,
    pub origin: i32,                // page index the bar lists (D2: kept while hopping)
    pub entries: [LbEntry; 64], pub n: usize,
    pub screen: i32,                // pager screen (6 entries per screen upright, 6 sideways)
    pub selected: i32,              // the peeked entry, -1
    pub rect: Rect,                 // the card
    pub hits: HitList, pub press: Press, pub pressed: Hit, pub has_pressed: bool,
    pub readonly: bool, pub links_ok: bool,     // nav.links_writable, links.ini readable
    pub header_hint: bool,          // this open shows the 'Tap a link to look...' line
    pub peek_label: FixedStr<128>,  // 'Showing %s.' text
    pub back_label: i32,            // origin page number for '‹ Page %d', 0 none
}

pub fn linkbar_build(lb: &mut LinkBar, nb: &NotebookInfo, nav: &Nav, page: &Page, origin: i32, dpi: f32);
pub fn linkbar_layout(lb: &mut LinkBar, aw: i32, ah: i32);
pub fn linkbar_draw(lb: &mut LinkBar, a: &mut App, nb: &NotebookInfo, nav: &Nav, peek: i32) -> Rect;
pub fn linkbar_pointer(lb: &mut LinkBar, a: &mut App, ph: i32, ev: &InEvent, dirty: &mut Rect) -> LbResult;
pub fn linkbar_probe(lb: &LinkBar, key: &str, out: &mut String, cap: usize) -> bool;
```

The editor takes `lb` out of its state for each call that also needs the App, as `with_modal` (editor.rs:337) does.
`LinkBar` must be `Default`: write `impl Default for LinkBar` by hand, since `[LbEntry; 64]` has no derived
`Default` (arrays derive it only up to 32). The bar's functions never reach into `EditorState` (PORTING "Sheets and keyboards").

### 13.2 Content (D1)

- Entries come from `nav_links_of(nav, pid, &mut first)` (nav.rs:568) and `nav_link_areas` for the boxes. Links whose
  ink is gone are not listed (dormant, as today). Each target is resolved with `nav_link_page` (nav.rs:589).
  Entries are deduplicated by target string and counted.
- **Order:** reading order of each entry's first box. Boxes whose tops are within 10 mm count as one line, read left
  to right. Entries are numbered 1..n, and the eye tags every box with its entry's number.
- **Labels:**
  - a named target: its Index name (`nav_name`, nav.rs:85), bold 44 px, cut with `font_draw_left_name` (font.rs:802);
  - an unnamed page: "Page %d" (existing msgid), regular;
  - a link to the Index: the Notebook icon and "The Index" (existing);
  - a missing target: "Page not here", 50% dithered. A tap on it toasts "That link's page is not in this notebook."
    (existing msgid, editor.rs:2433).
- **Right column:** "p. %d" at 34 px. "×%d" on a second line when the count > 1.

### 13.3 Layout (logical px at 300 dpi; scale with `ui_px`)

**Upright (1860 x 2480).** The card is at x 20, 1820 wide, its bottom 20 px above the screen edge. From the top:
- 16 px padding;
- a 36 px grip strip with a 160 x 8 bar;
- a 100 px header:
  - origin state: "Links on this page (%d)" at the left, an X of 118 x 100 at the right, and a pager "‹ 1/3 ›" (two
    118 x 100 buttons) left of the X when there are more than 6 entries;
  - peek state: a back chip "‹ Page %d" of at most 380 px, then "Showing %s. Tap the page to stay." cut to fit, then
    the X;
- R rows of chips (R = 1..3): 2 columns of 878 x 124, a 24 px column gap and a 16 px row gap;
- a 20 px gap;
- a 140 px tool row of four 430 x 140 buttons with 20 px gaps, each an icon over a 34 px caps word: Go to / New link /
  Areas / Index;
- 20 px padding.

Height = 316 + 140 R (456, 596 or 736 px). A chip is a 64 px black disc with the white number in 34 px bold, the name
from x + 96, and a 150 px right column. Nothing wraps.

**Sideways (2480 x 1860).** 2440 wide, 3 columns of 784 x 124, at most 2 rows (6 per screen), tool buttons 585 x 140,
height 456 or 596 px.

**States:**
- Empty page: "No links on this page yet." in the chip area, with New link as the Primary (inverted) button.
- First 3 opens (bits 3-4 < 3): the header's second line "Tap a link to look. Tap the page to stay." (the card grows
  by 60 px; increment the counter at open).
- Read-only page (`open_page().readonly`): New link is drawn `ButtonStyle::Disabled`. A tap on it toasts "This page is
  read-only.".
- `!nav.links_writable` (links.ini unreadable): the chip area says "Links could not be read." New link and Areas are
  disabled, and New link toasts "Links could not be read." when tapped.

**Areas button:**
- off: the eye glyph drawn through a 50% ordered dither (as `sim_dither`'s 8x8 Bayer, or `ui_icon_hollow`,
  ui.rs:318) inside a Plain frame, so it reads as dimmed, not `ButtonStyle::Disabled` gray;
- on: a solid eye on `ButtonStyle::Selected`.

**New glyphs** (`IconKind`, ui.rs:22; codepoints in `K_ICON_CP`, ui.rs:230; all four are present in
assets/TerminessNerdFontMono-Regular.ttf's cmap):
- `Search = 53` (0xF0349, md-magnify);
- `Eye = 54` (0xF0208, md-eye);
- `Bubbles = 55` (0xF05E3, md-chart_bubble);
- `LinkPlus = 56` (0xF0C94, md-link_plus).

The existing `Link` (0xF0339, md-link_variant) stays the panel's chain. `IconKind::COUNT` (ui.rs:80) goes from 53 to
57. Add each glyph with its md name in the comment.

`compose` sets `a.toast_floor = lb.rect.y` after drawing the bar (compose resets it at editor.rs:1115).

### 13.4 Opening and closing

Openers:
- `touch.2.up` (through `editor_gesture`);
- the panel's Links button (`P_LINKS`, section 15);
- the `links` action from a key or a controller.

`open_menu(SHORTCUT_LINKS)`:
- `s.link_mode = false` (opening disarms link mode, silently: `link_stop`, editor.rs:2269, would toast);
- mark the boxes stale and rebuild them, `linkbar_build`, `linkbar_layout`;
- `compose(a, lb.rect)`, `app_invalidate(a, lb.rect, Ui)`: one GL16, no flash.

Close without a peek: `lb.open = false`, free the canvas cache, `compose(a, lb.rect)`, `app_invalidate(lb.rect, Ui)`.

Opening the bar shows only the bar. The eye's boxes stay as `settings.link_areas` says.

### 13.5 Pointer routing while the bar is up

In `editor_event`'s down dispatch (editor.rs:3375 onwards), the bar's branch comes first, before the Pages view
(editor.rs:3381), the sheets and the panel:

| Down | Becomes |
|---|---|
| finger or pen inside `lb.rect` | `Owner::Bar` (new): `linkbar_pointer` gets phases 0/1/2. A chip or button inverts at the down by `app_press_feedback` (DU) and acts at the up if `press_is_tap`. A 15 mm downward drag that starts on the grip or the header is a Cancel |
| finger outside (the panel included) | `Owner::Outside` (new): nothing at the down (no panel press feedback); decided at the lift (13.6): a lift on the panel's Links button cancels, any other lift outside commits and presses nothing |
| pen outside | an immediate commit if peeking, else close; then `Owner::Outside` with `owner_is_pen` swallows the stroke until PenUp/PenLeave; it raises feature A's capture-blocked flag (1.1) |

New `Owner` variants: `Bar = 11` and `Outside = 12` (editor.rs:56).

- `editor_busy` (editor.rs:3212) is unchanged: a non-None owner is busy.
- A hold past 600 ms on a chip cancels the press silently (no peek). In phase 1 the card is not wired.

### 13.6 Peek and commit (D2)

**Peek state** (memory only, in `EditorState`):

```rust
struct Peek { active: bool, index: i32, pid: u32, zoom: f32, ox: f32, oy: f32, slot: usize, count: i32 }
```

The open page's `s.index`, `s.page`, its history, `s.zoom/ox/oy`, the box cache and link hit tests stay on the origin
until a commit, so autosave, undo or a link tap can never act on the peeked page. A peek is a display state only.
`page_canvas` keeps the origin's pixels throughout: it is the cache's pinned origin entry.

**Tap entry i** (`LbResult::Peek(i)`; target page t >= 0):
1. The press already inverted the chip (DU, immediate).
2. Get the target's canvas from the cache (13.9). On a miss, load and render it pumped.
3. Set `peek = { index: t, view: view_rest(aw, ah, pgw, pgh, landscape_fit) }`, centred as `view_clamp` would.
4. Set `lb.selected = i`, and switch the header to the peek state.
5. `compose(a, area)`, where `area = {0, 0, aw, lb.rect.y}`. `compose` blits the peek canvas at the peek view:
   generalize `blit_view` (editor.rs:919) to take the source canvas and the view. The panel draws its page chip with
   the peeked number.
6. Recompose the bar (the selected chip keeps an 8 px frame, the header shows the peek state).
7. One `app_invalidate(rect_union(area, lb.rect), Ui)` and no flush in between: the accumulator would union the two
   anyway (app.rs:1329-1339), so the peek is one GL16 present, `ui 0 0 1860 <bar bottom>` upright (no separate flush
   is stated for it, rule 2.4).

There is no GC16. Peeks never set `nb.last_page` or `info_dirty`, never write thumbnails, and never call `save_page`.

Other entries:
- tapping the selected entry again is a Commit;
- an entry for the Index commits (if peeking), closes the bar and opens the Pages view's Index tab (`pages_show` with
  `index_tab`, as `link_follow` does at editor.rs:2427-2430);
- a missing entry toasts (13.2).

**Commit** (stay), from an `Owner::Outside` finger lift within the 30 px slop, a pen down outside, or the selected
entry tapped again. The order at the finger's lift:
1. `TOUCH_F_CANCEL` set: nothing (the touch became a gesture).
2. `corner_tap(...)` true: nothing else (the clock toggles).
3. The lift is on the panel's Links button: Cancel (13.7), not a commit.
4. Otherwise: commit if peeking, else close the bar.

A second finger landing first clears `Owner::Outside` (`PinchBegin`, 8.4), so the first finger of a two-finger close
never commits.

**`bar_commit(a)`** follows `go_to_page` (editor.rs:1302) without a render and without a second read:
1. `save_page(a)` for the origin, then `write_thumb_if_needed(a)`.
2. Make t the open page without reading it again. If the session cache (`s.hist`) has t's page, the cache entry was
   rendered from it (`from_hist`). Otherwise move the entry's `page` (read by `store_load_page_pumped`, the editor's
   own reader at the notebook's page size, 13.9) into the session cache: `let e = s.entry_for(pid)?;
   s.hist[e].page = entry.page.take();`, which is `page_at`'s work (editor.rs:451-461) with the read already done.
3. Swap the entry's canvas into `page_canvas` with `std::mem::swap` (never a copy) and set `s.canvas_page =
   s.open_page_ref()`: it was rendered from that very Page, so they match by construction. Re-render
   (`render_page_full(a)`) only if the entry's bands are not all done or the peek's render drew a missing picture
   white (`bgimage_take_missing`).
4. Set `s.page`, `s.index = t`, `nb.last_page = t`, `info_dirty = true`.
5. Push the origin on the back list (13.8). If the shown page was reached through Back, pop instead.
6. `view_reset`; close the bar and free the cache; mark the boxes stale.
7. `compose_all(a, Full)` (one GC16: it clears the peeks' ghosting and resets the every-30th counter), then
   `arrive_readonly(a)`.

### 13.7 Cancel and Back

**Cancel** (back to the origin), from:
- X;
- the `links` action or the Links button again;
- two fingers down (the bar's fixed cancel, 8.2, whatever is bound);
- the grip or header drag;
- Close menu;
- a key bound to `go-back` or `close-menu` while the bar is up;
- leaving the editor or a sleep (13.10).

If peeking: clear the peek (`page_canvas` and the view were never touched), close the bar and free the cache.
`compose_all(a, Ui)`, or `Full` when `peek.count >= 3` (GL16 ghosting builds up). Otherwise, the close of 13.4.

**Back chip** "‹ Page %d" (peek state): clear the peek, keep the bar, `compose(area)` + `Ui` (GL16 of the page area).
Not peeking with a non-empty back list, the header shows "Back to %s" (the top's name, or "Page %d"). A tap peeks the
top of the back list.

### 13.8 The back list

`s.back: Vec<u32>` of page ids, at most 32 (drop the oldest), memory only, cleared at notebook close (`close_notebook`,
editor.rs:1400).
- **Pushed by:** link follows (`link_follow`), bar commits, Find results, and pages opened from the Pages view
  (`PvResult::Open` in `pages_event`, editor.rs:2386).
- **Not pushed by** next or previous page.
- **Popped by:** the `go-back` action, through `go_to_page`.
- Pages deleted since are skipped.

### 13.9 Canvas cache, pumped reader, prefetch

- `s.ccache: [CacheEntry; 2]` plus the origin's `page_canvas` (3 page canvases in all, about 3 x 4.6 MB):
  `CacheEntry { pid: u32, w: i32, h: i32, pic_gen: u32, canvas: Canvas, page: Option<Box<Page>>, rows_done: i32,
  from_hist: bool }`.
  - Key: page id + page size + `bgimage_generation()`.
  - Victim: least recently used, never the entry being shown.
  - Freed when the bar closes.
  - `canvas_init` (canvas.rs:20) already uses `try_reserve_exact`. When no entry can be allocated, the tap does what
    a link follow does: close the bar and `go_to_page(t)` (the same pixels, slower, no peek; the origin goes on the
    back list). Count it as a miss in `canvas_cache`.
- **Pumped reader** (store.rs): `pub fn store_load_page_pumped(nb, index, w, h, out) -> i32`. It is the editor's own
  reader, `store_load_page` (store.rs:1729, which every editor read goes through via `load_page`, editor.rs:445),
  with `work_pump()` every 256 strokes while deserializing, so a peek shows a page exactly as opening it would (a
  damaged file is set aside and side-restored paper written back, as at an open). The editor calls it at its page
  size (`s.pgw`, `s.pgh`).
  - Add an internal `page_deserialize_with(buf, out, pump: bool)` behind `page_deserialize` (store.rs:2246);
    `page_deserialize` keeps its behaviour and calls.
  - `store_load_page` becomes the unpumped wrapper, byte-identical for its callers; the exporter's
    `store_read_page` (store.rs:1776) is untouched.
  - The page in the session cache (`s.hist`) is used when present (`from_hist`), so nothing is read.
  - At a commit the entry's `page` moves into the session cache (13.6), so the page is read once.
- **Render:** `page_render(&mut entry.canvas, page, band, dpi)` (page.rs:236; it pumps every 128 points) in 160-row
  bands. A peek renders all remaining bands at once (a miss: 0.1-0.3 s, up to 1.2 s cold, pumped).
- **Prefetch** in `editor_tick` (editor.rs:3627), only when `a.input_drained && s.owner == Owner::None && lb.open`:
  - order: the visible entries' targets, then the top of the back list;
  - one 160-row band per tick;
  - a page not in memory is read by the pumped reader first (one page per tick);
  - stop at the first contact (a non-None owner).
- `editor.canvas_cache` counts hits and misses since open.

### 13.10 Acts on the shown page, so it commits first

- **Go to:** commit, then the Find sheet.
- **New link:** commit, close the bar, then today's `link_start` (editor.rs:2258).
- **Index:** commit, close the bar, then the Pages view's Index tab (the Index map is phase 5).
- **Another menu's button or key:** commit, then swap. In `editor_shortcut_at`'s menu branch (editor.rs:2832), when
  `front == SHORTCUT_LINKS`, a peek is active, and `action` is neither `SHORTCUT_LINKS` nor `SHORTCUT_CLOSE_MENU`:
  call `bar_commit(a)`, then `open_menu(a, action)`, and return true. Its own action and Close menu go through
  `close_front`, which cancels.
- **Pager ‹ ›:** recompose the chip grid only, GL16 of its rect.
- **Areas:** section 12 (this does not commit).
- **Leaving the editor, the power button, sleep or a crash during a peek:** `editor_leave`, `editor_suspend`
  (editor.rs:3322) and `close_notebook` cancel the peek first. Peeks never write notebook.ini, so a reopen lands on
  the origin.

---------------------------------------------------------------------------------------------------------------

## 14. The Find sheet (`findsheet.rs`, new, editor-hosted) and name search

- **Types:**

  ```rust
  #[derive(Default)]
  pub struct FindSheet {
      pub open: bool,
      pub kb: Keyboard,
      pub rows: [FindRow; 5],
      pub n: usize,
      pub rect: Rect,
      pub hits: HitList,
      pub press: Press,
  }

  #[derive(Clone, Copy, Default)]
  pub struct FindRow { pub page: i32, pub label: FixedStr<128> }
  ```

  API: `findsheet_open(f, a)`, `findsheet_update(f, nb, nav)` (recomputes rows from `kb.text`), `findsheet_draw(f, a)
  -> Rect` and `findsheet_pointer(f, a, ph, ev, dirty) -> FsResult { None, Go(i32), Close, Changed }`. `EditorState`
  gains `find: FindSheet`.
- **Keyboard:** `keyboard_open(&mut f.kb, tr("Page number or name"), b"")` (keyboard.rs:328), letters page (digits are
  row 0 there, keyboard.rs:82).
  - Every key goes through `app_keyboard_pointer` (app.rs:1434): field then key as separate Fast flushes.
  - `editor_tick` calls `app_keyboard_tick` (app.rs:1465) for the 900 ms GL16 settle, which also invalidates `f.rect`
    with `Ui`.
  - While open, `a.toast_floor = kb.area.y`.
- **Rows** sit above the keyboard modal (the field is inside the modal, with the keys right under it): up to 5 rows
  upright, 3 sideways, each 124 px with 16 px gaps, in a card whose top is never above `corner_zone(a).h + 8`.
- **The split keyboard host** (app.rs, new; PLAN-V2 8.1). Today `app_keyboard_pointer` has already invalidated the
  key's rect, unflushed, when it returns `Changed` (app.rs:1450-1455), so an `app_flush` of the rows would present
  rows and key as one union from the rows down to the key, the overlapping refresh the host avoids
  (app.rs:1444-1448). Add `pub fn app_keyboard_pointer_split(a: &mut App, kb: &mut Keyboard, phase: i32, ev:
  &InEvent) -> (KbResult, Rect)`: today's body up to and including the field flush and the `settle_at` update,
  returning the key's rect `d` without invalidating it. `app_keyboard_pointer` becomes the split call plus
  `app_invalidate(a, d, RefreshMode::Fast)`, byte-identical for its callers (j_pages_keyboard and the Index naming
  keyboard keep their `display.presents`). Find calls the split host, and on `KbResult::Changed`:
  - `findsheet_update` recomputes the rows;
  - redraw the rows card, `app_invalidate(rect, Fast)` and `app_flush(a)` (a stated separate flush);
  - then `app_invalidate(a, d, Fast)` for the key.

  So typing presents field, rows, key, each a Fast refresh of its own.
- **Matching:**
  - Digits only (`strtol10`-style): "Go to page %d" when 1 <= n <= npages, else "There is no page %d (%d pages).",
    which is not tappable.
  - Letters: every named page (`nav_index`, nav.rs:454, then `nav_name`) matched with `i18n_name_match(name, query)`:
    - 3 = prefix, 2 = a word start, 1 = substring, 0 = none;
    - accent-folded and case-insensitive;
    - sorted by rank, then by page order.
  - No match: one untappable row "No page names match.".
- **Name search helpers** (13.6 of the plan):
  - make store.rs's `fold_next` (store.rs:1444) `pub(crate)`, unchanged;
  - add `pub fn i18n_fold_next(s: &[u8], out: &mut [u8; 5]) -> usize` and `pub fn i18n_name_match(name: &str, query:
    &str) -> i32` in i18n.rs, built on it;
  - `cmp_name` (store.rs:1487) and its callers are unchanged.
- **Results:**
  - a row tap or Done on the first tappable row: close Find, push the back list, `go_to_page` (GC16);
  - Cancel, a tap outside (the corner excepted: `corner_tap` first; the tap is swallowed, `Owner::Outside`), Close menu
    or `go-to-page` again: close, `compose(rect union kb.area)` + `Ui`.
- **Openers:**
  - the bar's Go to (commit first);
  - the `go-to-page` action;
  - the Pages view's "Go to page..." (`PG_JUMP`, pagesview.rs:673). It now returns a new `PvResult::GoTo`
    (pagesview.rs:34), and `pages_event` closes the Pages view and opens Find over the page in one `compose_all(Full)`.

  The Pages view's number pad (`v.kb.page = 2`) is no longer opened from there. Keep its code for New/Copy if they use
  it (they do not today).
- `j_pages_keyboard.qs` is updated deliberately: its "Go to page" part now drives Find. Keep its `keyboard.draws`
  bound and the `display.presents` discipline (field then key).
- The keyboard dock is phase 2 (PICK mode is its first caller): do not add it now (0.1).

---------------------------------------------------------------------------------------------------------------

## 15. The panel's Links button (`panel.rs`, `editor.rs`)

- `const P_LINKS: i32 = 24;` (editor.rs:51 ends at `P_PICK_PHOTOS = 23`).
- In `draw_panel` (editor.rs:512), after the Next button (editor.rs:631-635) and before the divider: draw `ui_button(c,
  r, IconKind::Link, "", Plain)` and `hits_add(.., P_LINKS, 0)`.
  - When the open page has links (`nav_links_of(..) > 0`), add a 7 px solid black dot inside the button's top-right
    corner, 10 px in.
  - Advance `y` by `btn_h() + gap()`.
- `PANEL_BUTTONS` (panel.rs:10) goes from 11 to 12. Update the comment "Tool, eraser, undo, redo, clear, previous,
  next, links, paper, display, pen feel, library". `panel_height` (panel.rs:56) and `panel_fit` (panel.rs:69) follow
  through the constant.
- `panel_act` (editor.rs:2620): `P_LINKS` calls `editor_shortcut_at(a, SHORTCUT_LINKS, ..)` semantics (open, or close
  if open).
- **Order of work** (it moves every button below Next by `btn_h() + gap()`, 108 px upright): commit the symbolic panel
  taps first (section 20.3), with the suite green before and after, then add the button.

---------------------------------------------------------------------------------------------------------------

## 16. UI strings (English / Spanish)

Every row is a new msgid unless marked "existing". Add a `// TRANSLATORS:` line with the budget above each:
- toasts: "about 1700 px at 44 px in bold, or two lines";
- bar chips: "about 700 px at 44 px in bold" upright, 600 sideways;
- tool words: "about 400 px at 34 px caps";
- checkbox rows: "about 1300 px at 44 px".

Check widths with `test_i18n.rs` and CHECK_FIT. Add the glossary words to po/es-glossary.md: enlace, zona, gesto.

| Where | English | Spanish |
|---|---|---|
| bar header | Links on this page (%d) | Enlaces de esta página (%d) |
| bar empty | No links on this page yet. | Esta página aún no tiene enlaces. |
| bar hint line | Tap a link to look. Tap the page to stay. | Toca un enlace para mirarlo. Toca la página para quedarte. |
| bar peek header | Showing %s. Tap the page to stay. | Mostrando %s. Toca la página para quedarte. |
| bar back chip | ‹ Page %d | ‹ Página %d |
| bar back to | Back to %s | Volver a %s |
| bar right column | p. %d | pág. %d |
| bar count | ×%d | ×%d |
| bar missing | Page not here | Página no disponible |
| tool row | Go to | Ir a |
| tool row | New link | Nuevo enlace |
| tool row | Areas | Áreas |
| tool row | Index (existing) | Índice |
| bar read-only | This page is read-only. | Esta página es de solo lectura. |
| bar links unreadable | Links could not be read. | No se pudieron leer los enlaces. |
| eye tip | Dotted boxes show where a finger tap follows a link. They never print. | Los recuadros punteados muestran dónde un toque con el dedo sigue un enlace. Nunca se imprimen. |
| go-back empty | Nothing to go back to. | No hay a dónde volver. |
| first-open tip | New: two fingers up show this page's links. Two fingers down show all pages. | Novedad: dos dedos hacia arriba muestran los enlaces de esta página; hacia abajo, todas las páginas. |
| pan hint | Here two fingers move the page. Open Pages and Links from the panel. | Aquí dos dedos mueven la página. Abre Páginas y Enlaces desde el panel. |
| pen toast | Move the pen away, then swipe. | Aparta el lápiz y desliza. |
| last page | This is the last page. | Es la última página. |
| first page | This is the first page. | Es la primera página. |
| Display row | Sideways, show the whole page | De lado, ver la página entera |
| fit on | Sideways shows the whole page. | De lado se ve la página entera. |
| fit off | Sideways fills the width. | De lado ocupa todo el ancho. |
| fit upright | Applies when the Kindle is sideways. | Se aplica con el Kindle de lado. |
| sideways hint | Sideways now shows the whole page, so two-finger swipes work. Pinch to zoom in; Display changes this. | De lado ahora se ve la página entera y los gestos de dos dedos funcionan. Pellizca para ampliar; se cambia en Pantalla. |
| Find title | Page number or name | Número o nombre de página |
| Find digits | Go to page %d | Ir a la página %d |
| Find too big | There is no page %d (%d pages). | No hay página %d (%d páginas). |
| Find none | No page names match. | Ningún nombre coincide. |
| action label | Links on this page | Enlaces de esta página |
| action label | Go to page (existing msgid) | Ir a la página (existing) |
| action label | Back to the page before | Volver a la página anterior |
| action label | Show link areas | Mostrar zonas de enlace |
| action label | Whole page sideways | Página entera de lado |
| Buttons group | Index... | Índice... |
| Buttons group title | Index (existing) | Índice |

Notes on these strings:
- "Find none" is a brief addition (an empty result needs a line).
- "Go to page" reuses the existing msgid and translation (po/es.po:2019) rather than the plan's "Ir a una página".
- "Page %d" (po/es.po:943) and "The Index" (po/es.po:1963) are existing.
- The Display sheet calls itself "Pantalla" in Spanish: check po/es.po's "Display" before shipping the hint.

---------------------------------------------------------------------------------------------------------------

## 17. E-ink refresh choices (no new display call, no new wait)

Modes are `display.rs:16-24`: `Fast` = DU, `Ui` = GL16, `Full` = GC16. Measured: DU 188 ms (294 cold), GL16 271 ms,
GC16 341 ms. Screens only `app_invalidate`; `app_flush` presents once per loop pass with the strongest mode over one
union.

| Surface | Waveform | Rect | Notes |
|---|---|---|---|
| Two-finger gesture while moving | none | - | the action's own refresh is the feedback |
| PAN / ZOOM | Fast at most every 180 ms of t_ms, then Ui at PinchEnd if it moved | whole screen (`compose_all`) | as today's pinch |
| Panel press cancelled by a second finger | Fast | the button | was Ui (deliberate change) |
| Chip or bar button press | DU immediate (`app_press_feedback`) | the chip | outside the accumulator |
| Bar open / close (no peek) | Ui | the card rect | about 0.35 s, no flash |
| Peek | Ui | one present: the page area `{0, 0, aw, card.y}` (panel included) united with the card | never GC16 |
| Commit | Full | whole screen | resets the every-30th counter |
| Cancel | Ui, or Full after 3 or more peeks | whole screen | |
| Back chip | Ui | page area | |
| Pager | Ui | the chip grid | |
| Eye toggle | Ui of the button, `app_flush`, then Ui of the boxes' union | separate flushes; a third flush when far apart | no toast except the first-time tip |
| Find open from the bar or an action | Ui | rows card union keyboard | |
| Find open from the Pages view | Full | whole screen | the view closes in the same refresh |
| Find typing | Fast field, flush; Fast rows card, flush; Fast key; one Ui of keyboard and rows 900 ms after typing pauses | | the split keyboard host (14) |
| Find result | Full (`go_to_page`) | whole screen | |
| Page-turn end toast | toast only | toast rect | no page refresh |
| `landscape_fit` toggle sideways | Full (`view_reset` + `compose_all`) | whole screen | upright: toast only |
| Rotation | Full (existing) | | |

- Never a GL16 or a toast while the pen is down for ink. The pen commit during a peek happens at PenDown (plan 7.5
  and 5 (d)). Its stroke is swallowed: `draw_begin` never runs, so `display_pen_mode` is never turned on for it, and
  the commit's GC16 does not meet pen mode.
- The every-30th-Ui promotion to Full (app.rs:1360-1368) stays.

---------------------------------------------------------------------------------------------------------------

## 18. The touch log and the trace

### 18.1 grimoire.log (new `qlog!` lines only; existing lines unchanged)

Written only while input is grabbed. touchgest is never fed at the PIN (`discard_input`), so nothing is logged there.

```
input: touch slots %d..%d, touch_major %d..%d r%d
touch: k %u n %d%s                              <- the first 300 multi-contact frames per session; %s is, per contact,
                                                   " s%d:id%d:t%d:m%d:p%d" (slot, tracking id, tool, touch_major,
                                                   pressure); no coordinates
touch: session %d contacts, landing gaps %s ms, lift gaps %s ms: %s
                                                <- at the last lift of a multi-contact session; gaps as "0 112" (from
                                                   the first contact); the last %s is the outcome: "fired 2.down",
                                                   "rejected pen", "zoom", "pan", "palm"
touch: fingertips touch_major median %d max %d over %d frames
                                                <- at the lift of each single-finger session with major > 0
```

A session is from the first contact down to all lifted. Keep the per-session frame counter in input_evdev's State. Cap
the log's touch lines at 2000 per run (then one line "touch: log cap reached").

### 18.2 The opt-in trace (`touch-trace.txt`)

- **Switch:** in `app_init`, if `<store_root()>/touch-trace` exists (an empty file the owner creates), call
  `input_touch_trace(Some("<root>/touch-trace.txt"))`. The sim and none platforms accept and ignore it (the formatter
  is unit-tested in touchgest).
- **Appended** while grabbed only (never at the PIN), as text lines:

  ```
  # grimoire touch-trace 1, rotation R, WxH, dpi D        <- at each open of the file
  F <k_ms> <slot>:<id>:<x>:<y>:<major>:<tool> ...        <- one per touch frame; x, y logical px rounded
  P <k_ms> prox|down|up|leave                            <- pen state changes only, never pen coordinates
  E <k_ms> <outcome>                                     <- touchgest outcomes: "fired 2.down", "rejected pen"
  ```

- **Pure formatters** in touchgest.rs: `pub fn touchgest_trace_frame(f: &TgFrame, out: &mut String)` and
  `pub fn touchgest_trace_outcome(k_ms: u32, o: TgOutcome, out: &mut String)`.
- **Writing:** a 4 KiB buffer in input_evdev's State, written with `libc::write` to an `O_APPEND` fd at 4 KiB or at
  each session end. No file I/O per frame beyond that.
- **Cap:** stop at 4 MB (one "E <k_ms> cap" line).
- **Never exported:** it is in the root, not in a notebook; the share server does not serve it. Document it in
  FORMAT.md's file list and docs/KINDLE.md.

---------------------------------------------------------------------------------------------------------------

## 19. Unit tests

Run with `cargo test -- --test-threads=1`, host and `--features sim`. New files are picked up (`autotests = true`).
Follow `rust/tests/test_pensync.rs`: `mod testlib;`, `use grimoire::prelude::*;`, plain fns, one `#[test]` with
`run!(..)` lines.

**`rust/tests/test_touchgest.rs`** (new):
- **Parity:**
  - one finger: down, moves and up are emitted bit-identical (x, y, nx, ny) with no delay;
  - tool 2 cancels and suppresses until all lift;
  - a second finger at 120 ms gives cancel + `PinchBegin(1)`;
  - at 450 ms it is a palm;
  - the pen window suppresses one finger, with a cancel, not a lift;
  - value 0 is never emitted by touchgest (legacy belongs to the runner).
- **Fixes:**
  - pen arrival gives a cancel, never a tap;
  - `touchgest_cancel` after a simulated SYN_DROPPED leaves no phantom contact;
  - the pair is by landing order even when slots are reversed;
  - `touchgest_busy` counts a second contact with no first;
  - a 1.2 s gap between frames with kernel times 13 ms apart is classified by `k_ms`.
- **Thresholds, each on both sides:**
  - travel 19 / 21 mm;
  - angle 29 / 31 degrees;
  - landing 140 / 160 ms (swipe vs zoom/pan only);
  - spread ratio 0.55 / 0.65;
  - pen distance 35 / 45 mm (gone 500 ms);
  - finger ratio 1.9 / 2.1;
  - tap 290 / 310 ms;
  - tap move 4 / 6 mm.
- **Per-axis pan:**
  - `scrolls_v` with a vertical drag is PAN;
  - `scrolls_v` only with a horizontal drag is SWIPE `G_2_LEFT`;
  - a drag at a scroll limit is still PAN, never a swipe.
- **Other cases:**
  - a diagonal of 45 degrees with nothing scrolling rejects `straight`;
  - reversal under 20 mm cancels;
  - a resting thumb 50 mm away is ignored, and a moving one rejects `hand`;
  - a third contact rejects `three`;
  - `gestures_on` false never fires, and `keyboard_open` never fires.
- **Output:** order `PinchEnd` then `Gesture` in one `TgOut`.
- **Flags:** `TOUCH_F_BROAD` at `T_palm + 1`, not at `T_palm` (with a test-only T_palm); `TOUCH_F_CANCEL` only on
  cancels.
- **Trace:** the formatter's exact lines.

**`rust/tests/test_gesturemap.rs`** (new):
- built-ins;
- an overlay line overrides;
- `=nothing` switches one off;
- `touch.1.tap` and bad keys are refused and counted;
- pen and shape lines are carried and not acted on;
- an unreadable file gives the built-ins and -1;
- the mirror table;
- `key_for` order;
- the `key_name` strings.

**Other tests:**
- `editor.rs` `#[cfg(test)] mod tests` (editor.rs:4012): `view_rest` and `zoomed`:
  - with `landscape_fit` off, `zoom_max` and `zoomed()` are bit-identical to today's for upright 1860x2480, sideways
    2480x1860 and the 1986x2648 screen;
  - with it on sideways, `zoom == zoom_min`, `zoomed()` is false at rest and true after a pan.

  Extend `the_view_fits_the_pages_width` (editor.rs:4029) without changing its existing asserts.
- `test_probe.rs`: the four new settings keys are undotted, and `settings_save` writes them after `language`, in order,
  leaving every earlier line byte-identical (save a default `Settings`, compare its lines up to `language` with
  today's).
- `test_shortcuts.rs`: `SHORTCUT_LAST` 51 and each new name round-trips through `shortcut_action` /
  `shortcut_action_name`. The loops at test_shortcuts.rs:132, 171 and 185 cover them.
- `test_panel.rs`: `PANEL_BUTTONS` 12; `panel_fit` with 0-4 pins upright and sideways (the expected scale sideways
  with 4 pins is about 0.99; record the computed values).
- `test_i18n.rs`: the new strings fit their budgets in EN and ES.
- canvas: a `canvas_dash_rect` test checks the dash and gap lengths along each edge, solid corners, the halo pixels
  white, and clipping.
- `test_store.rs`: `store_load_page_pumped` returns the same page as `store_load_page` (stroke for stroke) and pumps
  (a counting hook set with `work_pump_set`, as test_pump.rs does) at least once per 256 strokes. This, with
  page.rs's existing pump every 128 points, is the Kindle's ~100 ms rule for peeks (the runner probe only catches a
  missing pump).
- `rust/tests/test_linkbar.rs` (new):
  - `linkbar_build` groups by target and counts (two links to p3 give one entry with count 2);
  - reading order (tops within 10 mm read left to right);
  - a link to `index` and a missing target;
  - links with no ink are not listed;
  - a read-only page gives `readonly`, and `links_writable == false` gives the "Links could not be read." state with 0
    entries;
  - `linkbar_layout` gives the heights of section 13.3 upright and sideways for 1, 2, 3 rows and the pager at 7
    entries.
- `i18n` tests: `i18n_name_match("Árbol", "arb") == 3`, `("La reunión", "reu") == 2`, `("Arena", "ren") == 1`, and
  `("Arena", "x") == 0`.
- `sim_sdl.rs` tests (sim_sdl.rs:1876, `--features sim`):
  - each new command's events and batching (`finger`/`frame` do not end a batch; `swipe`, `tapn` and `tap panel` do);
  - the frame clock offsets, and an `expect` between two `frame` lines while contacts are down: the second batch's
    frames never carry an earlier `k_ms` than the first's, and the tick uses the same clock;
  - `tap panel undo` is parsed by the tap arm's `panel` test, never as `tap 0 0`;
  - `pen leave` sets `TgPen.left_ms`, and `pentap` leaves the runner's `TgPen` out of proximity;
  - the legacy `tap`, `touch`, `drag` and `pinch` sequences are unchanged, with the existing tests kept as they are
    (for example `a_tap_is_one_batch_and_a_line_its_samples`).
- touch-log formatter: a test that `touchgest_trace_frame` and the log line formatter write nothing when told input is
  not grabbed (the formatter takes a `grabbed: bool`).

---------------------------------------------------------------------------------------------------------------

## 20. Scenarios

Every new file starts with `# rust-only`. Names sort as listed. A `# root: NAME` must sort before its user and
differ by a letter or digit.

Notes on the steps below:
- Panel taps use `tap panel` / `pentap panel`.
- Keyboard keys use SIM.md's coordinates (docs/SIM.md:100-110). After typing, assert `editor.text` / `editor.keyboard`
  rather than trusting coordinates.
- `clock.on` stands for feature B's probe (1.2).
- Contact-level touch after pen input waits out the pen window (6.1): `pen leave` if hovering, then `wait 1100`.

### 20.1 New scenarios

**`g_gest_1`** (upright, fresh root)
- New notebook (`tap 965 95`, `wait 100`, `tap 1564 2254`, `wait 200`), then `pentap panel next` twice: 3 pages.
- Go back to page 1 with `pentap panel previous` twice, then `wait 1100` (the pen window, 6.1).
- `swipe 2 930 900 0 400`: `expect editor.modal is pages`, `expect touch.gesture is 2.down`, and one Full in
  `display.presents`.
- `swipe 2 930 1400 0 -400`: `expect editor.modal is none`.
- `swipe 2 930 1400 0 -400`: `expect editor.bar is open`.
- `swipe 2 930 900 0 400`: `expect editor.bar is 0`, `expect editor.page is 1`.
- `swipe 2 1300 1200 -400 0`: `expect editor.page is 2`.
- `swipe 2 600 1200 400 0`: `expect editor.page is 1`.
- Swipe left twice more to page 3, then `swipe 2 1300 1200 -400 0` again: `expect toast is This is the last page.`,
  `expect editor.pages is 3`, `expect editor.page is 3`.
- Swipe right back to page 1, then once more: `expect toast is This is the first page.`
- `expect gestures.pages is touch.2.down`, `expect gestures.links is touch.2.up`.

**`g_gest_1b_unbound`** (`# copy: tests/fixtures/gestures-unbound .`, a new fixture whose gestures.ini is the one
line `touch.2.up=nothing`)
- New notebook with 3 pages as in `g_gest_1` (then `wait 1100`).
- `swipe 2 930 1400 0 -400`: `expect editor.bar is 0` (2-up is unbound), `expect gestures.links is` empty.
- `tap panel links`: `expect editor.bar is open`; `swipe 2 930 900 0 400`: `expect editor.bar is 0`, `expect
  editor.modal is none` (the fixed cancel, not Pages).

**`g_gest_2_zoomed`**
- `pinch 930 1200 300 900 6` (legacy) zooms: `expect editor.zoomed is 1`, `expect editor.scrolls is 1 1`.
- A contact-level vertical two-finger drag (`swipe 2 930 1200 0 400`): `expect editor.modal is none`,
  `expect touch.gesture` unchanged, the zoom unchanged (`editor.zoom` before and after), and `editor.zoomed` still 1
  (it panned).
- A drag starting 40 px from the top edge also pans.
- At the scroll limit, a drag opens nothing.
- The pan hint toast appears once (and not a second time), when the root's settings have `hints` bit 6 clear.
- `tap panel pages`: `expect editor.modal is pages`; still zoomed, `swipe 2 930 1400 0 -400`: `expect editor.modal is
  none` (the Pages view does not pan, so 2-up is a swipe and the mirror rule closes it).
- `tap panel links`: `expect editor.bar is open`, `expect editor.scrolls is 0 0`; `swipe 2 930 900 0 400`: `expect
  editor.bar is 0`, `expect editor.modal is none`, `expect editor.zoomed is 1` (back to the origin as left).

**`g_gest_3_sideways`**
- `rotate 1`, open a notebook: `expect editor.zoom is 1.00`, `expect editor.zoomed is 0`, `expect editor.scrolls is
  0 0`.
- `swipe 2 1240 700 0 400`: Pages opens.
- A second part in **`g_gest_3b_sideways_width`** with `# settings: landscape_fit=0`: rotate 1, an upward vertical
  drag `swipe 2 1240 1200 0 -400` pans (the width-fit view rests at the top, oy = 0, editor.rs:875-886, so a downward
  drag would pan nowhere; `editor.zoomed is 1` after); `swipe 2 1500 900 -400 0` turns the page.

**`g_gest_4_pen`**
- `pen line ...` then `pen leave`, `wait 350`; then `swipe 2` 60 mm away from the pen's last point: fires.
- `pen hover 900 1200`, then a `swipe 2` with a finger 20 mm away: `expect touch.reject is pen`,
  `expect toast is Move the pen away, then swipe.`
- Repeat at once: the toast does not reappear (`expect toast is` empty after `wait 2100`, then a second rejected
  swipe leaves it empty).

**`g_gest_5_pinch_off`**
- `# settings: pinch_zoom=0`.
- Swipes work (Pages opens); a spreading `finger`/`frame` sequence leaves `editor.zoom is 1.00`.
- Sideways with `landscape_fit=0`, an upward two-finger drag (`swipe 2 1240 1200 0 -400`) pans (`editor.zoomed is
  1`).

**`g_gest_6_keyboard`**
- With the link-keys fixture (`# copy: tests/fixtures/link-keys .`, `# settings: shortcuts_enabled=1`), `key 49 0`
  opens Page name: `swipe 2 930 900 0 400`: `expect editor.keyboard is Page name`, `expect editor.modal is pages`
  (nothing changed), `expect touch.reject is keyboard`.

**`g_gest_7_asleep`**
- `sleep`, `swipe 2 930 900 0 400`: `expect asleep is 1`; `wake`: `expect editor.modal is none`.

**`g_gest_8_corner`** (no links: the link, bar and peek cases are in `z_bar_9_corner`, which can start from
`z_bar_0_make`)
- In a new notebook, `tap 60 60`: `expect clock.on is 1`, `editor.page` and `editor.modal` unchanged.
- `tap 60 60` again: `expect clock.on is 0`.
- With the bar up (`swipe 2 930 1400 0 -400`, an empty bar): `tap 60 60` gives `expect editor.bar is open` and the
  clock toggled.
- Close the bar (`swipe 2 930 900 0 400`, its fixed cancel): `expect editor.bar is 0`. (With the bar up, a page turn
  would be refused by `sheet_open` and 2-down would close the bar instead of opening Pages.)
- Right after a fired gesture (`swipe 2 1300 1200 -400 0`, the last-page toast in a 1-page notebook), `tap 60 60`
  toggles the clock.
- A two-finger swipe whose first finger lands at 60 60:
  - `finger 1 down 60 60`, `frame 30`, `finger 2 down 360 60`, `frame 20`;
  - then ten frames moving both down 40 px each (`frame 20`);
  - then `finger 1 up ...`, `frame 5`, `finger 2 up ...`, `frame`.

  It fires `2.down` (`expect editor.modal is pages`), and `clock.on` is unchanged.
- `tapn 2 100 100` (fingers at x 25 and 175, the midpoint in the zone): `expect touch.gesture is 2.tap`, and the
  clock is unchanged.

**`s_fit_1`** (`# settings: shortcuts_enabled=1`, `# copy: tests/fixtures/index-keys .`)
- `rotate 1`, open a notebook with 2 pages: `expect editor.zoom is 1.00` and `expect editor.scale is 0.75` (1860x2480
  pages on 2480x1860: 1860 / 2480).
- `swipe 2 1240 700 0 400`: Pages opens; close it.
- `pinch 1240 930 300 900 6`: `editor.zoomed is 1`; `pentap panel next`: `editor.zoomed is 0`.
- Pinch in, then `key 33 0` (fit-page): `editor.zoomed is 0`.
- `key 17 0` (landscape-fit) toggles it, like the Display row below (then toggle it back).
- The Display sheet (`pentap panel display`): tap the new row: `expect settings.landscape_fit is 0`, `expect
  editor.zoom is 1.00`, `expect editor.scale is 1.33`, toast "Sideways fills the width.".
- Restart (`pentap panel library`, `waitbg`, reopen): persists.
- `rotate 0` and toggle: the toast "Applies when the Kindle is sideways." and no Full in `display.presents`.
- With `# settings: settings_version=3` (an upgrading owner): the first `rotate 1` shows the sideways hint once.

**`t_touch_1_contacts`** (the "finger resting on a link" case is in `z_bar_2_peek`, which has links)
- A contact-level spread zooms like the legacy pinch:
  - `finger 1 down 800 1200`, `finger 2 down 1100 1200`, `frame 20`;
  - then 8 frames spreading to 500 / 1400, `frame 160` each;
  - then the ups.

  Expect `editor.zoomed is 1` and `editor.zoom not 1.00`.
- `finger 1 down 900 1200 0 2`, `frame`: `expect touch.suppressed is 1`. It stays 1 while a second contact lands and
  lifts, and is 0 after every contact lifts.
- A pen arrival during a finger: `finger 1 down 900 1500`, `frame`, `pen hover 300 300`, `finger 1 up 900 1500`,
  `frame`: `expect touch.class is none` and no follow or press happened (`editor.modal is none`).

**New fixture `tests/fixtures/index-keys/shortcuts.ini`** (keys for these scenarios; a shortcuts.ini replaces
the defaults, shortcuts.rs:280):

```
# Grimoire: shortcuts for the phase 1 Index scenarios (tests/scenarios/z_bar_*, z_find_*, s_fit_1).
key.l=add-link
key.n=name-page
key.f=fit-page
key.b=go-back
key.g=go-to-page
key.k=links
key.a=link-areas
key.w=landscape-fit
key.x=close-menu
```

The evdev codes for `key CODE 0` are l 38, n 49, f 33, b 48, g 34, k 37, a 30, w 17 and x 45.

**`z_bar_0_make`** (the root that other scenarios start from)
- `# settings: shortcuts_enabled=1`, `# copy: tests/fixtures/index-keys .`
- New notebook (`tap 965 95`, `wait 100`, `tap 1564 2254`, `wait 200`); `pentap panel next` 4 times (5 pages).
- Name page 3 'Árbol': go to page 3 (`pentap panel previous` twice), `key 49 0`, then type with SIM.md's keyboard
  coordinates (shift, the accents page key, a acute, the letters page key, then r b o l), then Done (1643 2365).
  Check `expect editor.page_name is Árbol`.
- Name page 5 'Arena' the same way.
- Back on page 1, make three links with `key 38 0` (add-link) each time and a Pages-view pick:
  - `pen line 300 1000 900 1200 30 0.6 0.6`, then pick page 3 (`tap 1155 340`);
  - `pen line 300 1400 900 1500 30 0.6 0.6`, then pick page 2 (`tap 703 340`);
  - `pen line 300 1800 900 1900 30 0.6 0.6`, then pick page 3.

  Use the tile coordinates of z_index_2_links.qs. Check them with `expect editor.picking is new` before each pick.
- `expect editor.links is 3 3`.
- Close the notebook (`pentap panel library`, `waitbg`).

**`z_bar_1_list`** (`# root: z_bar_0_make`, `# env: GRIMOIRE_CHECK_FIT=1`)
- `tap 200 400` reopens the notebook on page 1.
- `swipe 2 930 1300 0 -400`:
  - `expect editor.bar is open`;
  - `expect editor.bar.entries is 2`;
  - `expect editor.bar.1 is p3 x2`;
  - `expect editor.bar.2 is p2`.
- `expect editor.bar.rect is 20 1944 1820 516`. By section 13.3 that is one chip row (456 px) plus the hint line's
  60 px, bottom 20 px above the edge. Confirm the value on the first run and keep it.
- `shot 01-bar-upright`.
- Closing:
  - X closes it (`tap` the X at the header's right): `expect editor.bar is 0`;
  - `swipe 2 930 900 0 400` closes it;
  - `key 45 0` (close-menu) closes it;
  - `key 37 0` (links) opens it and `key 37 0` again closes it.
- `rotate 1`: open it again, `shot 02-bar-sideways`, and record `expect editor.bar.rect` sideways.

**`z_bar_1b_list_es`** (`# root: z_bar_0_make`, `# env: GRIMOIRE_LANG=es GRIMOIRE_CHECK_FIT=1`): the same opens and
shots, in Spanish.

The read-only page and unreadable links.ini states are covered by `rust/tests/test_linkbar.rs` (section 19), not by
a scenario: no fixture can make links.ini unreadable portably.

**`z_bar_2_peek`** (`# root: z_bar_0_make`)
- Open the bar, tap entry 1: `expect editor.bar is peek`, `expect editor.bar.peek is 3`, `expect editor.page is 1`,
  `expect notebook.last_page is 1`, and `display.presents` shows one `ui 0 0 1860 <bar bottom>` (the page area and
  the card in one present, 13.6), no `full`.
- Tap entry 2: `expect editor.bar.peek is 2`.
- A finger tap above the bar on the link area (600 1100): `expect editor.bar is 0`, `expect editor.page is 2`, one
  `full`, and the link was not followed (the page is 2, not 3).
- `expect editor.back is 1 1`.
- `key 48 0` (go-back) returns to page 1: `expect editor.page is 1`, `expect editor.back is 0 0`.
- **Resting finger on a link when the pen arrives** (contact level): `finger 1 down 600 1100`, `frame`, `pen hover 900
  2000`, `finger 1 up 600 1100`, `frame`: `expect editor.page is 1` (no follow).
- **Legacy hold unchanged:** `touch down 600 1100`, `wait 800`: `expect editor.picking is had`; `touch up 600 1100`;
  Back.

**`z_bar_3_twofinger`** (`# root: z_bar_0_make`)
- During a peek, `swipe 2 930 900 0 400`: `expect editor.bar is 0`, `expect editor.page is 1`,
  `expect editor.modal is none` (Pages did not open).

**`z_bar_4_pen`** (`# root: z_bar_0_make`)
- During a peek: `pen line 300 600 900 620 30 0.5 0.9` outside the bar: `expect editor.bar is 0`,
  `expect editor.page is 3`, `expect editor.strokes is 0` (the stroke was swallowed), `expect display.ink is 0`.

**`z_bar_5_cancel`** (`# root: z_bar_0_make`)
- Three peeks then X: `expect editor.page is 1`, a `full` in `display.presents`.
- One peek then X: a `ui` (no full).
- During a peek, the back chip: `expect editor.bar is open` and `editor.bar.peek is 0`.

**`z_bar_6_leave`** (`# root: z_bar_0_make`)
- During a peek: `sleep`, then `wake`: `expect editor.bar is 0`, `expect editor.page is 1` (the peek was cancelled).
- Peek again, then `powerhold`: `expect running is 0`.

**`z_bar_6b_reopen`** (`# root: z_bar_6_leave`): `tap 200 400`: `expect editor.page is 1` (notebook.ini kept the
origin).

**`z_bar_7_prefetch`** (`# root: z_bar_0_make`)
- Open the bar; `expect main.longest_unpumped_ms not -1` (starts the count); `wait 1500`.
- Tap entry 1:
  - `expect editor.canvas_cache is 1 0 2` (one hit, no miss, two entries prefetched);
  - `expect main.longest_unpumped_ms is ok`.

  The probe prints `ok` for a longest gap under the runner's 250 ms host bound, else the gap in ms (brief decision:
  `expect` has no numeric comparison; presents and saves are not counted, 6.1).
- Commit (tap the page); then `key 48 0` (go-back): `expect editor.page is 1`.

**`z_bar_8_pager`** (`# settings: shortcuts_enabled=1`, `# copy: tests/fixtures/index-keys .`)
- A new notebook with 14 pages where page 1 links to pages 2..14: 13 lassos with `key 38 0`, each picked from the
  Pages view, using its `›` to reach tiles past the first screen.
- Open the bar: the header shows the pager; `expect editor.bar.entries is 13`.
- `›` twice reaches screen 3, showing entry 13; `‹` comes back.

**`z_bar_9_corner`** (`# root: z_bar_0_make`)
- On page 1, draw a link in the corner: `key 38 0`, `pen line 40 40 300 300 30 0.6 0.6`, pick page 2.
- `tap 60 60`: `expect clock.on is 1`, `expect editor.page is 1` (the link was not followed).
- With the bar up: `tap 60 60` toggles the clock, and `expect editor.bar is open`.
- While peeking (tap entry 1 first): `tap 60 60` toggles the clock, `expect editor.bar is peek`,
  `expect editor.page is 1`.
- `tap 600 1100` (an outside tap, not the corner) then commits: `expect editor.bar is 0`.
- The Links button again: open the bar, peek entry 1, `tap panel links`: `expect editor.bar is 0`, `expect
  editor.page is 1` (a cancel to the origin, not a commit).

**`z_find_1`** (`# root: z_bar_0_make`)
- Open the bar, tap Go to: `expect editor.find is go`, `expect keyboard.draws` within its bound.
- Type `3`: `expect editor.find.1 is Go to page 3`. Tap the row: `expect editor.page is 3`, `expect editor.back is 1
  1`.
- `go-back`: page 1.
- From the Pages view: `pentap panel pages`, then `tap 930 2395` (Go to page..., as in j_pages_keyboard.qs:104):
  `expect editor.find is go`, `expect editor.modal is find`, and one `full` in `display.presents`.
- Type `9`: `expect editor.find.1 is There is no page 9 (5 pages).`
- `key 34 0` (go-to-page) opens it from the page too, and again closes it.

**`z_find_2_names`** (`# root: z_bar_0_make`)
- Open Find (`key 34 0`) and type `arb`: `expect editor.find.rows is 1`, `expect editor.find.1 is Árbol`.
- Delete back and type `ar`: `expect editor.find.1 is Árbol`, `expect editor.find.2 is Arena`. Both are prefixes, so
  page order decides: Árbol is page 3.
- Type `x` (`arx`): `expect editor.find.1 is No page names match.`
- Row labels are the name alone. The page number is drawn at the right and is not in the probe.

**`z_find_3_es`** (`# root: z_bar_0_make`, `# env: GRIMOIRE_LANG=es GRIMOIRE_CHECK_FIT=1`)
- Find upright and sideways, shots, `keyboard.draws` within its bound.

**`z_link_areas_1`** (`# root: z_bar_0_make`)
- Open the bar, tap Areas: `expect settings.link_areas is 1`, `expect editor.areas is 3`, `shot 01-areas`.
- `display.presents`: two `ui` presents, the button then the union, the union within the boxes' bounding rect.
- Close the bar; `pen line` across an area: `expect editor.live_matches_render is 1`.
- Export the notebook's PDF as other scenarios do (`waitbg`): the PDF is unchanged (the overlay is not in it).
- Restart: still 1.
- Sideways and zoomed shots.
- The first-time tip toast (`expect toast is Dotted boxes show where...`).

**`y_touch_selftest`** (`# rust-only`; the short form is in tests/selftest/pass-rust.qs, 6.3)
- `tap panel undo` hits `P_UNDO` upright and sideways with 0-4 pins: after a stroke, `tap panel undo` gives
  `editor.strokes is 0`.
- Sideways: `rotate 1`, the same.
- After a stroke (Undo is disabled with nothing to undo, so it has no hit), `expect editor.panel_at.1768.594 is undo`
  upright with no pins.

### 20.2 Existing scenarios changed on purpose (each in its own commit, suite green before and after)

1. **Rotate scenarios that depend on the width fit** get `# settings: landscape_fit=0`: at least f_batch2,
   y_all_screens, y_pagesize_1_ksc, y_pagesize_2_back and z_4_erase_zoom_rotate. Run the 25 files that `rotate` (`grep
   -l '^rotate [13]' tests/scenarios/*.qs`) and add the header to each that fails only because of the fit. This keeps
   them C-compatible.
2. **`j_pages_keyboard`:** Go to page... now opens Find (keep the `keyboard.draws` and `display.presents` discipline;
   `# rust-only`).
3. **Buttons-sheet coordinates** (11.3): v_controller, v_controller_forget, v_controller_switch,
   v_controller_triggers and y_all_screens.
3b. **Display-sheet coordinates** (10.2). The new row makes the card one row taller, and `card_rect` re-centres it at
   `(a.h - h) / 3` (dispmenu.rs:81-87), so every existing row (sliders, Invert, the sleep rows) and Done move, in both
   hosts (the editor's panel and Settings > Display) and both orientations. Convert every raw tap and rect check on
   the Display sheet with the same probe-checked method as 20.3 (assert the row under the old coordinate first,
   with the Display probes, then retarget): at least i_display (for example its `display.last_fast_rect is 220 542
   1420 140` and `tap 400 878`), m_sleep, v_controller_actions, y_all_screens, y_koreader_hosted,
   y_koreader_not_hosted, y_pagesize_1_ksc, z_tool_5_sheets_turn and z_tool_b_button_over_sheets (`grep -l "modal is
   display\|display.last_fast_rect" tests/scenarios/*.qs`, then run the suite for the rest), plus SIM.md's documented
   coordinates ("the Display sheet's sleep screen row is 1240 1005 and its Done 1770 1325"). Each converted file gets
   `# rust-only`.
4. **Symbolic panel taps** (20.3).
5. **`y_all_screens`:** new shots after 65:
   - the Display sheet's new row (upright and sideways);
   - the bar (empty and with entries) upright and sideways;
   - the Find sheet upright and sideways;
   - EN and ES, both styles, CHECK_FIT (through l10n-check).

### 20.3 Converting raw panel taps (commit before `P_LINKS` lands)

About 65 files tap a panel button below Next (Paper, Display, Pen feel, Library) with raw coordinates upright, for
example `pentap 1768 1566` (Library). `grep -lE '^(tap|pentap) 17[0-9][0-9] ' tests/scenarios/*.qs` lists 84 files
with taps in the panel's column. To convert them:
1. Add `editor.panel_at.X.Y` and `tap panel NAME` first.
2. For each raw tap in the panel's column, insert `expect editor.panel_at.X.Y is NAME` before it, using a temporary
   script run (suite green: this proves which button each coordinate hits now).
3. Replace the tap with `tap panel NAME` / `pentap panel NAME` and drop the temporary expect.

Sideways and pinned-panel taps convert the same way. Taps that do not hit the panel stay raw. Each converted file
gets `# rust-only`. Then add `P_LINKS` in a separate commit. PLAN-V2 15.2 uses the same figures (84 in the column,
about 65 below Next).

### 20.4 Must stay green unchanged

- Every pinch scenario: f_batch2 (with its header), k_pinch_gate, p_power, v_controller_actions, y_all_screens,
  z_4_erase_zoom_rotate, y_pagesize_2_back, z_a_precise_other_size.
- z_index_1, z_index_2_links (inked links, `touch down`/`wait 800`/`touch up` hold), v_shortcuts_links, y_live_ink,
  h_live_vs_settled, live_1_card.

**Deliberate behaviour changes** (list them in the commit messages and the owner notes):
- the Fast restore of a panel press cancelled by a second finger;
- a PenDown during a finger link session draws;
- the off-screen cancel replaces the two real lifts (pen arrival, palm window);
- the sideways default view;
- Go to page... opens Find;
- the Buttons list has an Index group;
- the Display sheet has a seventh row;
- two fingers down over the bar always cancel to the origin;
- the panel has 12 buttons.

---------------------------------------------------------------------------------------------------------------

## 21. Docs to update in phase 1

- **docs/SIM.md:** commands, probes, `# rust-only`, the panel names, batching and the frame clock (6.3).
- **docs/FORMAT.md:**
  - the four settings keys;
  - `gestures.ini` (grammar, keys, built-ins, phase 1 reads only);
  - `touch-trace` / `touch-trace.txt`;
  - the file list rows;
  - "Moving between versions" (older versions ignore gestures.ini and carry the settings keys).
- **docs/SHORTCUTS.md:** the five actions; gestures do not need Enable shortcuts; the built-ins; gestures.ini by hand.
- **docs/DESIGN.md:** the bar, Find, the eye, the refresh table of section 17. Correct DU from about 120 to 188 ms.
- **docs/KINDLE.md:** the touch script (about 15 minutes in all, section 23, item 11) and how to create
  `grimoire/touch-trace`.
- **docs/TESTING.md:** the owner checks of section 23.
- **po/es-glossary.md:** enlace, zona, gesto.

---------------------------------------------------------------------------------------------------------------

## 22. Commit plan and the gate

### 22.1 Small logical commits, in this order

Each commit message ends with CLOUD.md's attribution lines (the Co-Authored-By and Claude-Session lines your
session's system reminder gives).

1. `Makefile, SIM.md: a # rust-only scenario header, rust-only selftest files, SCEN_SKIP_RUST_ONLY for c-check` (6.4).
2. `input: InType::Gesture, touch flag bits, input_touch_context, touch_slots` (3.1-3.4, platform stubs).
3. `touchgest.rs: the pure touch recognizer, with today's one-finger and pinch parity` (+ test_touchgest parity
   cases).
4. `touchgest.rs: two-finger swipes and taps, per-axis pan, pen-near and resting rules` (+ threshold tests).
5. `gesturemap.rs: built-in gestures and gestures.ini (read only)` (+ test_gesturemap, `App.gestures`).
6. `input_evdev: touch frames through touchgest; kernel time, SYN_DROPPED, pumped touch, cancels, busy flag`.
7. `input_evdev: the touch log and the opt-in touch trace`.
8. `runner: finger, frame, swipe, tapn and touch.* probes` (+ sim tests, SIM.md, selftest).
9. `settings: gestures, landscape_fit, hints, link_areas` (+ probes, test_probe, FORMAT.md).
10. `runner: tap panel NAME and editor.panel_at; scenarios tap the panel by name` (20.3).
11. `editor: gestures dispatch, the mirror rule, page turns that never add a page, classified pinches` (+ g_gest_1..7,
    t_touch_1).
12. `editor: a pen down ends a finger link session; corner taps are never claimed` (+ g_gest_8).
13. `editor, dispmenu: sideways shows the whole page` (+ s_fit_1, rotate headers, Display-sheet coordinates (20.2
    item 3b), l10n).
14. `shortcuts, library: links, go-to-page, go-back, link-areas, landscape-fit` (+ Buttons coordinates, l10n).
15. `canvas: canvas_dash_rect`; `editor: the link box cache and the eye` (+ z_link_areas_1).
16. `panel: the Links button` (PANEL_BUTTONS 12, test_panel).
17. `linkbar.rs: the Links bar list and layout` (+ z_bar_0, 1, 1b, 8, glyphs, l10n).
18. `store: a pumped store_load_page`; `editor: peek, commit, cancel, back list, canvas cache, prefetch` (+ z_bar_2..7,
    9, g_gest_1b).
19. `app: a split keyboard host`; `findsheet.rs, i18n: Go to page or name` (+ pagesview PvResult::GoTo,
    j_pages_keyboard, z_find_*).
20. `hints: the first-open, pan, eye and sideways tips` (+ upgrade gate).
21. `y_all_screens and l10n-check: the new screens`; `docs: phase 1`.

Push the branch at checkpoints (after commits 8, 14 and 21).

### 22.2 The gate: `make rust-check`

It runs unit tests twice (host, `--features sim`), headless, tools, kindle (cargo zigbuild armv7 musl), scenarios plus
selftest, l10n-check, mirror-cleanup-check, share_check, live_check, bt_check and the exporter (Makefile:556-564).
- Use `GRIMOIRE_TEST_PORT=8850` (any free port) to avoid clashes.
- **Linux x86_64 vs the owner's Mac (arm64):** recorded float fixtures (the brush probe fixture recorded with Apple's
  libm) and outline-font CRCs (compared on Apple arm64 only) may differ here. Never edit an expectation to pass on
  Linux. List which ones differ and why in your final report. The Mac gate is authoritative.
- **`make c-check`** runs the same scenarios rule, the selftest and l10n-check against the C binaries. With
  `SCEN_SKIP_RUST_ONLY=1` (6.4) it skips `# rust-only` files, the rust-only selftest files and `y_all_screens`, so it
  keeps passing, but it no longer guards the existing scenarios this phase converted (about 70: the panel taps, the
  Buttons and Display sheets, j_pages_keyboard). Count them with `grep -l '^# rust-only' tests/scenarios/*.qs` and
  say so in the report. That is the owner's open question 4 in PLAN-V2.
- **Never run a GUI binary, never touch a device.**

---------------------------------------------------------------------------------------------------------------

## 23. Owner acceptance on the Kindle (by hand, numbered)

Install the phase 1 build. Unless said otherwise, test in a notebook that has your beta.5 links.

1. **Upright.**
   - Two fingers down anywhere on a page open Pages with one flash; two fingers up close them.
   - Two fingers left turn to the next page, and right to the previous.
   - On the last page, two fingers left show "This is the last page." and no page is added. On the first page, two
     fingers right show "This is the first page."
   - The panel's + still adds a page.
2. **The Links bar.**
   - Two fingers up (or the new chain button after Next on the panel; it has a small dot when the page has links) open
     the bar along the bottom with no flash.
   - It shows one row per Index entry the page links to, numbered in reading order, with "×2" where two links go to
     the same page.
   - Two fingers down, X, or the chain button again close it.
3. **Hopping.**
   - Tap an entry: the page above the bar becomes that page with no flash, the panel's chip shows its number, and the
     bar keeps the starting page's links.
   - Tap another entry: it shows that one.
   - Tap the page above the bar with a finger: you stay there (one flash), and that tap does nothing else (no link
     followed, no panel button pressed).
   - Or press X or "‹ Page 3": you are back where you started, zoom included.
   - A pen stroke on the page while peeking also stays and is not drawn.
4. **Areas.** In the bar, Areas shows dashed, numbered boxes where each link's ink is, matching the row numbers. They
   are not in the PDF. They are remembered after a restart. The first time, a tip explains them.
5. **Go to.**
   - The bar's Go to (and the Pages view's "Go to page...") opens a keyboard over the page.
   - "12" offers "Go to page 12"; a name finds the page with accents and case ignored ("arb" finds "Árbol").
   - The result opens with one flash.
   - A key bound to Back to the page before returns.
6. **Sideways.**
   - Turn the Kindle: the whole page shows (on by default). Two-finger swipes work anywhere, and pinch zooms in from
     the whole page.
   - Display > "Sideways, show the whole page" off: the width fills again, vertical two-finger drags pan, and left and
     right still turn pages.
   - Zoomed in (either orientation): two fingers pan with no zoom drift, the panel's page chip and chain button open
     Pages and Links, and the hint about it shows once.
7. **The clock corner.**
   - A finger tap in the top-left corner toggles the clock: with nothing open, with the bar up, while peeking, and
     over a link there (the link is not followed).
   - A two-finger swipe that starts there fires its gesture and does not toggle the clock.
8. **Feel.**
   - Pinch zoom feels as before.
   - A finger resting on a link no longer follows it when the pen comes near.
   - After a long page render (a dense page or a PDF page), taps and swipes still register.
   - Writing with your palm resting fires nothing.
9. **Live view.** With Live view on, committing a peek with the pen shows no ink in the browser.
10. **Report:**
    - write, lift, and swipe right at once: how often "Move the pen away, then swipe." appears;
    - rest your palm and thumb while writing (nothing should fire).
11. **Traces.** Create the empty file `grimoire/touch-trace`, restart Grimoire, and run the touch script in
    docs/KINDLE.md (about 15 minutes in all: a 3-minute two-finger part, then the phase 3 set):
    - 20 two-finger swipes each way, 10 two-finger taps, 5 three-finger taps (nothing should happen);
    - writing with the palm resting;
    - 10 swipes straight after lifting the pen;
    - one-finger Z, N, S, circle, alpha, triangle and V shapes (10 each; nothing should happen in this build);
    - 10 one-finger swipes each way;
    - 5 minutes of writing and 2 minutes holding and reading.

    Then delete `grimoire/touch-trace` and send `grimoire.log` and `grimoire/touch-trace.txt`.
12. **Settings.** `settings.ini` gains `gestures=1`, `landscape_fit=1`, `hints=...` and `link_areas=...` at the end;
    your other settings are unchanged. Turning Grimoire back to an older build keeps working.

Known in this build:
- New link still uses the inked flow;
- holding a link on the page still opens the Pages picker;
- holding a bar row does nothing;
- three-finger gestures are off.
