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
`/mnt/us/kindle-lab/grimoire-patch-check/backup-20261024/`. The reader was not forcibly restarted by the test; the new
files apply on its next restart. The owner subsequently reported that both the battery spacing
and DPI controls look right after the restart prompt. This is owner visual confirmation on this
Scribe, not an automated pixel measurement. Remove `1-grimoire-scribe-dpi.lua` and restore the backed-up
Terminess file, then restart, to revert these two requested changes.

## Additional KPM upgrade checks

Wrapped the already published beta.2 installer in a test-only KPM package (1.1.1002), using the
new hooks. Directly installing the RC local file over it reproduced a KPM 0.2.2 dependency graph
failure before the hooks. Cleanup removed the test package while preserving its data.

Then registered a temporary local manifest with the same v2 schema as the generated feeds.
Installed beta by package ID, updated the manifest to include the RC, and installed `grimoire`
again: KPM correctly selected 1.1.5001 and ran the upgrade hooks. The actual cached ARM binary
changed from beta.2 to RC, notebook/settings sentinels survived, and no KOReader integration was
automatically installed. Test package/repository were removed and original repository listing
and KOReader process IDs stayed unchanged. The local-file uninstall/reinstall fallback then
loaded the RC into the original beta test root and preserved its sentinels too.

This validates a generated repository feed and real upgrade hooks with local artifact URLs;
public HTTPS feed installation remains to be checked once those URLs exist. It does not imply
a beta KPM artifact was previously published. See KPM.md for the local-file limitation.

## Candidate check results

- 24 baseline ASan/UBSan unit executables: 46,779 checks, zero failures.
- Full headless scenario pass, including the new enabled/disabled shortcut scenarios.
- After adding the DPI row, reran the updated all-screen language/layout checks in both themes
  and English/Spanish: all passed. An initial layout run used the old six-row binary with the
  updated seven-row test coordinates; rebuilding and rerunning resolved those test failures.
- Final KOReader manager (40 checks), shortcuts (26), and power policy (104) tests: all passed.
- Final ARM/SDL builds, ZIP contents, launcher simulations, KPM tests (9), Lua patch tests (530)
  and workflow actionlint validation: all passed.
- Final share/web checks, QR decoding, Effects comparison, Copy behavior and host export: passed;
  two fixture notebooks exported, zero failures.
- Final KPM package repeated the successful physical lifecycle test; temporary package, registry
  entry and test root were removed. The explicitly requested patch fixes and backup remain.

## Search-bar install follow-up

The owner's real `;kpm install grimoire` attempt reached KPM and selected the RC, but failed at
`mv` from `grimoire/.kpm-install` into the actual Documents folder. KPM rolled back and retained
the beta.2 app. A probe confirmed fsp rejects moves into that folder from elsewhere, while
same-directory rename/replace succeeds. Earlier isolated roots did not exercise that policy.

The hook now stages hidden files directly beside the destination launchers. A regression test
simulates fsp's boundary and a failed second rename, checking rollback and staging cleanup.
Retried real installation through KPM after backing up the original Library launchers under
`/mnt/us/kindle-lab/grimoire-before-kpm-20261024/`: installed entry is `grimoire|1|1|5001`, actual
Library wrappers are KPM wrappers, and unpacked ARM `--version` reports `1.1.0-rc.1`. Settings
hash and the running KOReader process IDs stayed unchanged. This actual installation and its
local test source are intentionally retained for the owner, unlike earlier temporary test roots.

The first cloud run also exposed a share-test timing race: its server exited after 25 seconds
while the slower runner was still downloading. The test now holds the server until its driver
stops it; the complete local share/QR/Effects/Copy checks passed with that lifetime control.

## Remaining hardware checks

- USB/Bluetooth keyboard and actual 8BitDo enumeration, button numbers, hotplug, modifiers and
  disconnect recovery. Analog axes/hats are unsupported.
- Native PIN unlock with the physical keypad and a controller attached.
- KOReader handover/wallpaper on the tester setups where beta integration failed.
- Public HTTPS KPM feed installation after publication, and Library indexing with KOReader closed.

These remain explicit candidate checks. Host simulation and unpacking do not establish them.
