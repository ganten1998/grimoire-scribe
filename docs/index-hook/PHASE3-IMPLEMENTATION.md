# Phase 3 implementation and validation

This work adds the shared action registry, editable gestures.ini, Settings > Gestures,
recorded finger shapes, page trails, a tunable two-finger recognizer and host replay tools.
This note records implementation contracts, deviations and host evidence. Device acceptance
and the owner-trace tuning study remain separate checks.

## Contracts and documentation

- [SHORTCUTS.md](../SHORTCUTS.md#phase-3-action-registry) lists every actual registered name,
  kind, group and source mask. Buttons and Gestures share seven groups. Calibration is not
  assignable; hold-eraser needs a key/pad release; Enable shortcuts and Bluetooth controller
  can be assigned only to touch/shape so a key/pad cannot disable itself.
- [FORMAT.md](../FORMAT.md) specifies the exact three-line header, changed-slot ordering,
  format-1 shape bytes, newer-line carry budgets, empty-file removal, atomic copy-save-swap
  and Reset recovery; `rotation_lock`, `hints` bit 7 and coordinate-free `touch: measure`.
- [SIM.md](../SIM.md#phase-3-touch-runner-and-recording) specifies `shape`, `fpath`, `trace`,
  capability/tuning hooks, recording steps and probes. Settings bottom-row and record-card
  tap points were verified with `library.ui_at` in both languages, themes and orientations;
  sample slots are display-only.
- [DESIGN.md](../DESIGN.md#settings--gestures-and-page-finger-shapes-index-phase-3) records
  sheet/card/trail behavior and the refresh contract: isolated Fast trail frames, Ui settle,
  no GL16 during pen-down, Full for invert/theme/language. Trails never become notebook data.
- [KINDLE.md](../KINDLE.md#the-two-finger-study) and [TESTING.md](../TESTING.md) carry the owner
  script and acceptance checks. [TOUCH-STUDY.md](TOUCH-STUDY.md) records provisional values
  with pending results rather than invented owner measurements.

## Deliberate deviations and compatibility

1. New action ids are **56..77**, preserving existing 54 (`gesture-new-page`) and 55
   (`status-mode`). Persisted bindings use names, not these internal numbers.
2. SleepScreen already had KOReader wallpaper at value **3**. The sleep-screen action keeps
   the four-state page / pinned page / Kindle screensaver / KOReader wallpaper cycle.
3. No owner trace is available. `TG_TUNE` is the brief's provisional table, not a freeze from
   real-hand percentiles. Synthetic traces/corpus evidence is explicitly synthetic. The
   owner-data shape freeze gate remains unverified despite page-shape code being present.
4. Recording refusals without a visible trail or old pattern refresh only the reason line.
   Sample/pattern changes redraw their dynamic content and buttons below the fixed title;
   trail cleanup also restores the actual trail bounds. A second-finger cancel before a trail
   appears causes no card redraw.
5. Classic and Gothic use the same outer editor panel rectangle on the scenario's base
   layout. The theme action verifies the setting and a Full relayout rather than requiring
   that rectangle to change. Unknown-light refusal is tested before any platform read in
   `test_actions`; app startup already fills the simulator cache, so the scenario checks
   actual cached light/warmth steps instead.
6. Frame timestamps can lead a wall-clock tick during replay. Signed wrapping elapsed times
   keep the pending/drawing state alive until its actual timeout, including across u32 wrap.
7. With two samples, the first Use runs checks. A newly discovered reassignment stays visible
   with the original bindings intact; the next Use confirms. A third sample runs checks
   immediately, so its reassignment message is visible before the first Use.

## Localization and validation scope

The catalog was extracted once with `make l10n` after the library/editor completion notes
arrived. All 117 new active entries are translated using brief section 15 and the glossary;
older active translations and contexts are unchanged. There are no fuzzy or empty active
entries. `msgfmt --check --no-hash --endianness=little` compiles the Spanish catalog, and its
output matches `po/es.mo` byte for byte. A direct literal-msgid audit found no current runtime
English string missing from the extracted POT.

The focused `test_i18n` target passed on host and with `--features sim`: **2,859 checks,
zero failures in each build**. It enforces Controller/Gestures/Updates at 248 px / 34 px caps
in English/Spanish and Gothic/Classic, record-card refusals/progress/conflicts within two
34 px lines and 1292 px (including actual wrapped width), the 256-byte reason buffer, and
unwrapped footer/button budgets. No translation abbreviation was needed for Actualizaciones.
The documented gestures.ini header also matches the source byte for byte. A live font-overflow
test additionally renders all seven Gestures groups and the root sheet in both languages,
themes and orientations; long binding rows and the Ideas line use the existing two-line
layout. A recorder test checks the actual two-line slip refusal, not only its translated parts.

The reviewed phase 2 fix and shape/graph/penbutton foundations were merged at `49f7e27`
(merge `6f05bb5`). Later RC map, folders and phase 4 integration remain with the parent.
`index-p3-host-arm.log` records the passing complete host/simulator unit suites and ARM build;
later timer/UI/confirmation fixes also pass their focused tests. The final stable-tree gate
is recorded separately in `index-p3-check.log` once scenarios are complete.

All four ignored release fingershape tests pass in `index-p3-corpus-full.log`, with
**954 checks and zero failures**. The retained seven-shape harsh corpus accepts **3328/3500
(95.1%)** in use and **3500/3500** in recording, with **0 wrong fires in 6986 comparisons**.
Per 12,000 synthetic slip paths, swipes and thumb arcs fire zero times; single-turn slides
fire twice and hairpins zero times. The final pad study retains `FS_BANK_PAD = 10`;
`PURE-MODULES.md` records both pad runs. These are synthetic results, not owner measurements.

## Integration boundary

The owner requested a phase 3 handback before another complete gate because RC integration
is adding map/folders/shapes in parallel. No final `index-p3-check.log` pass is claimed.
The complete English/Gothic `y_all_screens` scenario passes; the phase 3 gallery also passes
in English/Classic and Spanish in both themes. Both Rust runner selftests pass. Nine existing
scenario files were converted for the Settings/Buttons layout, alongside 23 new scenarios.
`v_actions_screen` and `s_shape_4_guards` passed their final focused reruns after correcting
cache expectations and checking an uncommitted pinch's trail cleanup at PinchEnd.

Two focused checks remain open at this boundary:

- `g_gest_10b_v1` expects `slow` for the early lift; the unchanged V1 recognizer reports
  `short` before following the remaining finger. It still rejects the gesture. The scenario
  needs its legacy edge expectation reconciled with the brief.
- `g_gest_12_threeb_slots2` correctly rejects a three-contact gesture as `three`, but the
  Gestures action row still displays an imported `3-finger tap` binding on a two-slot panel.
  Capability filtering of that display remains to close. The saved binding is preserved.

The parent should preserve `rotation_lock` before its later `shapes` / `shape_hold_ms`
settings on integration, and retain all reviewed shape/graph/penbutton foundation modules.
The owner-trace tuning study and device acceptance remain pending; synthetic trace fixtures
are explicitly labelled and do not claim owner evidence.
