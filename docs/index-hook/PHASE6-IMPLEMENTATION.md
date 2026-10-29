# Phase 6 implementation: Library, localization and documentation

This records the owned Library/UI work against [PHASE6-BRIEF.md](PHASE6-BRIEF.md) and the
shared ownership contract. It is not a release gate or Kindle acceptance report. The owner
ruling replaces the brief's pseudo-actions with real PEN-only `pan` / `zoom` actions 82/83.
Existing actions 78..81, folder navigation hit codes 69/70, and map/folder code are preserved.

## Delivered behavior

Settings > Gestures includes PEN in its filter. Hold eraser is pen-only there; Pan with the
pen and Zoom with the pen come from the registry's Pages rows and accept only drag. Ordinary
actions accept all five button kinds. The Settings side-button row remains the ink-choice
cycler; clearing is represented by the triple/hold bindings in Gestures.

A row shows its first available touch, shape or pen source, in that order, and `+N` additional
action/source bindings. Multiple slots for one action/source count once, including the legal
Clear page triple/hold duplicate. Paired rows summarize both actions. Three-finger bindings
are hidden unless the panel reports at least three slots; unsupported Button2 and Eraser
bindings are not live row candidates. Known and carried newer shapes count as one source.
The existing shape thumbnail column is retained.

The card owns a separate `PenButtons` and pen proximity/contact state. PenButton events and
pen hover/contact/lift/leave feed the model with event timestamps; ticks run only after
`App.input_drained`. It uses `App.pb_config`, including waits derived from current bindings.
Button and Button2 have translated click/double/triple/hold/drag labels. The recognized name
wraps in the existing pattern area at 64 px; the new body explicitly mentions the pen button.
Hold eraser rejects clicks and touch/shapes; pan/zoom reject everything except a pen drag,
with translated two-line refusals.

Use calls `gesturemap_set` on a copy and publishes it only after atomic save succeeds. The
card names an occupied slot's current action; successful reassignment names the losing
action in a toast. Save failure leaves the candidate and live map intact. Existing shape
first-Use clash detection still requires another Use to confirm a newly found conflict.
Normal nib taps operate Use, Try again and Cancel. Button drags cannot activate those buttons
or become page ink. Ui is GL16 on the Kindle: recognized drag/refusal state updates at contact,
but its card/line refresh waits for nib lift or proximity loss.

Map save/reset/load call `app_gestures_sync` after publishing. Reset starts from a map clone
so the registry's hardware capability survives default restoration. No new hit codes or
public Library APIs were needed.

## Timing and persistent format

A click is a press of at most 450 ms without nib contact. If double or triple is bound for
that button, even a single click waits 350 ms after release. Otherwise tap fires at release.
Each next press of a double/triple must be within 350 ms of the previous release; recognition
resolves 350 ms after the final release. This replaces the old three-clicks-within-1.2-seconds
rule. A hovering hold remains 1200 ms; momentary erasing starts at press without that wait.

`pen.Button.*` is live. `pen.Button2.*` is live only with BTN_STYLUS2 capability;
`pen.Eraser.*` is always carried. Slots are `tap`, `double`, `triple`, `hold`, `drag`.
`nothing` clears a built-in; `hold-eraser` is legal only for hold/drag, and registered `pan` /
`zoom` only for drag. The shared map, carry limits and copy-save-swap behavior are documented
in [FORMAT.md](../FORMAT.md). There is no separate pen.ini or page-format change.

## API dependencies

The implementation consumes the fixed shared APIs:

- `App.pb_config: PbConfig`, `app_gestures_sync(&mut App)` and `App.input_drained`.
- `PenButtons`, `PbPen`, `PbIn`, `PbOut`, `penbutton_event`, `penbutton_pen`, `penbutton_tick`.
- `gesturemap_pen_button2`, `gesturemap_key_for(..., ActionSrc::Pen)`, `gesturemap_set`,
  `gesturemap_clear_action`, load/save/reset and the existing shape APIs.
- `SHORTCUT_PEN_PAN=82`, `SHORTCUT_PEN_ZOOM=83`, source masks and registry rows.
- `InputCaps.pen_button2`, `InEvent.side_button2`; PenButton value 0 selects Button,
  value 2 selects Button2, and the corresponding boolean supplies its state.

No interface changes are requested. Editor dispatch, live capture blocking, physical input,
the pure recognizer/map and runner are their respective owners' work.

## Focused validation

Use the pinned Rust 1.97.1 toolchain and shared `build/cargo` target directory. These checks
were run for the Library/UI change, with `--test-threads=1`:

| Check | Result |
| --- | --- |
| `cargo test --manifest-path rust/Cargo.toml --lib library::tests -- --test-threads=1` | 9 tests passed |
| `cargo test --manifest-path rust/Cargo.toml --features sim --lib library::tests -- --test-threads=1` | 9 tests passed |
| `cargo test --manifest-path rust/Cargo.toml --test test_i18n -- --test-threads=1` | passed |
| `make l10n` | extraction/merge completed; only es.po/es.mo and grimoire.pot are tracked outputs |
| `msgfmt --check --no-hash --endianness=little -o po/es.mo po/es.po` | passed |
| Owned-file `git diff --check` | passed |

The new Library tests exercise 349/350 ms resolution, no premature tick on an undrained queue,
a queued release below the hold threshold, immediate-tap policy after clearing multi-click
slots, save/reset timing sync, nib button taps, reassignment and failed-save preservation,
1199/1200 ms hold recognition, slot/source refusals, contact/lift and proximity cancellation,
no Ui/GL16 submission during a recorded drag, source counting and unsupported-capability rows.
Existing Library tests still cover shape first-Use confirmation, sheet/hit geometry and
thumbnails. The i18n tests check all ten names at 64 px, longest registry labels plus `+N`
with thumbnail room, the new body and refusals, EN/ES and Gothic/Classic, upright/sideways
widths. The Spanish catalog has no untranslated entries.

## Remaining integration and owner work

The completed focused scenarios and selftests are recorded below. The parent owns the combined
release gate and retargeting of older scenarios against its final integrated UI. No full gate,
project formatting, GUI, device/SSH operation, publishing or push was performed here.

No Phase 6 Kindle result or Phase 3 owner trace is available here. `TG_TUNE`, real-hand shape
acceptance, accidental-fire rates, hover spacing, physical second-button behavior and e-ink
appearance remain unmeasured. Run the numbered [Kindle owner checks](../KINDLE.md#phase-6-owner-checks-pending-on-a-kindle)
and preserve the pending [Phase 3 study](TOUCH-STUDY.md); synthetic tests do not replace either.

## Editor, input and Live integration

The editor feeds the pure timing model before page/map pointer routing and ticks it only
after the input queue drains. A bound contact gets Owner::PenDrag, dispatches once at
the nib anchor, and swallows the remainder even when the button is released before the nib.
An unbound drag keeps the existing side-button ink. Pen page turns never add a page.
Map keyboard guards and the existing menu close/swap path remain authoritative.

Momentary pen erasing starts at button press and ends at release/proximity loss. Its hold
mask is separate from key/controller/Live holds, so releasing one source cannot release
another. Pan/zoom retain their start anchor, clamp the view, submit Fast at most every
180 ms, and finish with Ui. Bound-action toasts and non-Fast presents wait for nib lift;
sleep cancels deferred feedback. Pen arrival also aborts a pending finger capture.

Both input readers and the runner carry independent first/second button state. The new
open-time capability line is input: pen buttons %d; unsupported Button2 stays carried.
The side-click helper schedules edges without blocking the input loop. Model wall ticks
that precede a queued edge cannot expire that edge, including across u32 wrap.

The early Live path also distinguishes both button bits, before the queued pen-down has
an editor owner. Its internal capture table now has eight styles (eraser and two buttons).
The 20-byte public sample stays the same size and uses the additional LIVE_SIDE2 flag;
the browser's drawing decision remains LIVE_DRAW. Raw Live decoding/resync carries
BTN_STYLUS2, preventing provisional ink for second-button drags and selecting the correct
momentary eraser style. Focused tests cover this independently of the final page render.

## Completed focused handback

Source/runtime commit: 7553d60. The completion commit adds twelve v_gestures_4 scenarios,
five text-binding fixtures plus their README, and both Rust selftest updates. The passing
selftest's existing panel hit points were retargeted for the Shapes button. Older combined
Pages/screen-tour scenarios remain with the parent.

The scope covers recording Links on double-click; editable triple/hold Clear page defaults;
immediate momentary erasing; non-ink Pages drag; actual pan/zoom UI recording and motion;
legacy unbound side-button ink; delayed and immediate tap; hidden/carried and enabled/live
Button2; and the bilingual pen UI gallery in both orientations. The focused harness also
checks saved pen lines and both cleared built-ins, beyond the touch-first action probes.

Pinned host model/Live/i18n tests pass, including 12 standalone PenButtons boundary tests,
the existing pure tests, 959 action checks, 246 gesturemap checks, and the new second-button
early Live regression. Five editor pen tests pass on host and simulator; nine Library tests
pass on each build. Input validation passes 29 adapter/simulator tests plus three runner
checks. The current headless and ARM release builds pass in index-p6-host-arm.log.

The final focused scenario run is index-p6-focused-final.log: twelve scenario runs, both
Rust selftests (including the exact expected exit-3 failure oracle), and four EN/ES x
Gothic/Classic gallery runs. All 18 expected outcomes pass. Each gallery includes upright
and sideways states with CHECK_FIT. The binary hash is
387c6ea3c2fc99930a1d10a7b2b4d5e14c24e6614ce9154f0c8fbccdec827c03.

Actual rendering exposed a clipped Spanish Hold eraser title. The fixed title uses the
existing 64-to-34 px fitting path within its original bounds. A regression now renders
every assignable action title in both languages/themes; recognized pen names remain 64 px.
No translation was shortened to hide this issue.

No known Phase 6 source blocker remains. No final combined rust-check result or device
acceptance is claimed. Parent heads after 6f3cdd6 (including c34aaed) have deliberately
not been merged here; the parent will resolve those overlaps during integration.
