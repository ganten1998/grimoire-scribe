# Phase 3 implementation and remaining evidence

This work adds the shared action registry, editable gestures.ini, Settings > Gestures,
recorded finger shapes, page trails, a tunable two-finger recognizer and host replay tools.
This note records implementation contracts and deviations, not device acceptance or a full
gate result. The main owner is responsible for scenarios and the final `index-p3-check.log`.

## Contracts and documentation

- [SHORTCUTS.md](../SHORTCUTS.md#phase-3-action-registry) lists every actual registered name,
  kind, group and source mask. Buttons and Gestures share seven groups. Calibration is not
  assignable; hold-eraser needs a key/pad release; Enable shortcuts and Bluetooth controller
  can be assigned only to touch/shape so a key/pad cannot disable itself.
- [FORMAT.md](../FORMAT.md) specifies the exact three-line header, changed-slot ordering,
  format-1 shape bytes, newer-line carry budgets, empty-file removal, atomic copy-save-swap
  and Reset recovery; `rotation_lock`, `hints` bit 7 and coordinate-free `touch: measure`.
- [SIM.md](../SIM.md#phase-3-touch-runner-and-recording) specifies `shape`, `fpath`, `trace`,
  capability/tuning hooks, recording steps and probes. Settings bottom-row and card/slot
  coordinates remain omitted until the main scenario owner verifies hits with `library.ui_at`.
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
5. Unverified coordinates are not supplied as approximations. The main scenario owner must
   check upright/sideways and both languages/themes before publishing numeric hit locations.

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
The documented gestures.ini header also matches the source byte for byte, and the owned
changes pass `git diff --check`.

No full gate, scenario suite or device check was run by this docs/localization worker.
The main owner's final full-gate record will be `index-p3-check.log`. Late English strings can
be filled using the dictionary/helper in `/tmp/grimoire-p3-agents/fill_phase3_es.py`; it reports
unmapped strings rather than claiming they are translated. See that directory's
`docs-finish-result.md` for the handoff and focused log names.
