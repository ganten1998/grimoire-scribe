# 1.1.0-rc.1 validation

## Physical device: October 24, 2026

Kindle Scribe 2024, firmware 5.19.6, ARMv7, KOReader v2026.07.2 and KPM 0.2.2 / CLI 1.0.0
(`kindlehf`). Used a dedicated temporary userstore root with sentinel notebook/settings files.
No normal Grimoire UI launch, pen/touch grabs or KOReader handover was performed. The final
package was retested through the full isolated KPM lifecycle after including the updated patches.

Passed through the real KPM registry and hooks:

- Install the generated local `.kpkg` using `file://`.
- Both Library wrappers appear only in the isolated documents folder.
- `kpm launch grimoire --unpack-only` unpacks the actual embedded ARM payload.
- ARM binary `--version` reports `1.1.0-rc.1` without initializing framebuffer/input.
- Uninstall removes the package registry directory and unchanged test wrappers.
- Notebook sentinel, settings and cached app remain.
- Hashes of the original installed Grimoire Library launchers remain identical.
- KOReader's process IDs remain identical across the test.

KPM printed a nonfatal scanner `doFullScan` unavailable message during install/remove while
KOReader was running. Both operations returned success. A normal Library indexing test is still
needed after quitting KOReader; this isolated test intentionally did not add real Library items.

The 530 pure-Lua patch checks also passed on the Kindle's own LuaJIT. An additional isolated
probe loaded its real framebuffer API and real Screen DPI menu callbacks with in-memory
settings: a 24-unit widget measured Auto=58 px, 320=62 px, 480=92 px, 640=123 px; custom 420
selected correctly. Every selection retained KOReader's restart request. The test did not open
screen/input devices, save preferences or restart the running reader. These measurements
establish the code path and scaling, not a visual check of the restarted UI.

## Automated coverage

`make -j4 check` is the release gate: ASan/UBSan unit tests, headless scenarios, translations and
text-fit checks, web/share/export checks, ARM cross-build, ZIP/launcher checks and KPM tests.
New coverage includes parser/modifier matching, settings persistence, disabled shortcuts,
keyboard/gamepad actions, drawing/sheet/native-PIN gates, page boundaries, opt-in plugin behavior,
KPM hook staging/data preservation and preview/stable feed ordering.

The requested Terminess and DPI fixes were then placed in the active KOReader patches folder,
after byte-comparing the old font patch to the known beta version and syntax-checking both new
files with the Kindle's LuaJIT. The original font patch is backed up under
`/mnt/us/kindle-lab/grimoire-patch-check/backup-20261024/`. The reader was not restarted; the new
files apply on its next restart. Remove `1-grimoire-scribe-dpi.lua` and restore the backed-up
Terminess file, then restart, to revert these two requested changes.

## Candidate check results

- 24 baseline ASan/UBSan unit executables: 46,779 checks, zero failures.
- Full headless scenario pass, including the new enabled/disabled shortcut scenarios.
- After adding the DPI row, reran the updated all-screen language/layout checks in both themes
  and English/Spanish: all passed. An initial layout run used the old six-row binary with the
  updated seven-row test coordinates; rebuilding and rerunning resolved those test failures.
- Final KOReader manager (40 checks), shortcuts (26), and power policy (104) tests: all passed.
- Final ARM/SDL builds, ZIP contents, launcher simulations, KPM tests (8), Lua patch tests (530)
  and workflow actionlint validation: all passed.
- Final share/web checks, QR decoding, Effects comparison, Copy behavior and host export: passed;
  two fixture notebooks exported, zero failures.
- Final KPM package repeated the successful physical lifecycle test; temporary package, registry
  entry and test root were removed. The explicitly requested patch fixes and backup remain.

## Remaining hardware checks

- Visual Terminess battery spacing and DPI changes after a full KOReader restart.
- USB/Bluetooth keyboard and actual 8BitDo enumeration, button numbers, hotplug, modifiers and
  disconnect recovery. Analog axes/hats are unsupported.
- Native PIN unlock with the physical keypad and a controller attached.
- KOReader handover/wallpaper on the tester setups where beta integration failed.
- KPM upgrade from a published prior KPM package and Library indexing with KOReader closed.

These remain explicit candidate checks. Host simulation and unpacking do not establish them.
