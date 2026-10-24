# 1.1.0-rc.2 validation

RC2 removes the experimental KOReader DPI patch, its Settings control and runtime tests.
KOReader keeps its native sizing policy and the owner's existing DPI preference. The updated
Terminess font-family selection and dropdown battery spacing remain. The calibration startup
tip is removed; startup scenarios now expect no toast in the normal case.

## Package and migration checks

The ZIP's optional files, embedded app archive and KPM payload contain only the three supported
patches. No DPI script is shipped. The launcher removes the byte-exact RC1 DPI script from its
own app cache, where tar's merge could otherwise leave it behind. Cleanup also runs when the
current build is already unpacked. Modified cache files and KOReader's active patches are not
removed automatically. A historical text fixture supports the regression test; it is not bundled.

KPM tests cover Documents' same-directory rename requirement, staged failure, rollback, data
preservation and preview/stable version ordering. Launcher tests check retirement of the old
cache file and preservation of local changes. Terminess Lua checks cover font matching, real
bold, main/aux battery spacing, bidi wrappers, idempotency and isolation to the menu status line.

## Actual Kindle

Scribe 2024, firmware 5.19.6, KPM 0.2.2. The active DPI file was already absent before this work.
KPM upgraded the actual installed RC1 (1.1.5001) to RC2 (1.1.5002). After adding cache retirement,
reinstalling the explicit package ID refreshed the RC2 payload and pruned the known old file.
The installed ARM binary reports `1.1.0-rc.2`; DPI is absent from both the active KOReader patches
folder and Grimoire's app cache, and Terminess remains installed. The settings hash matches the
snapshot from before the upgrade. No Grimoire UI was launched by the deployment check.
The owner then confirmed both opening Grimoire from KOReader and returning to KOReader work.
After removing the startup tip, the package was refreshed again with a fresh settings snapshot;
the installed ARM SHA256 matches the tested no-tip build and KOReader's process IDs stayed
unchanged. Startup/passcode/stale-annotation cases, the scenario runner's intentional-failure
check, translation notices, and the rebuilt package/launchers passed focused verification.

The real package and `grimoirelocalrc` source are retained for the owner's testing. Earlier
launchers remain backed up under `kindle-lab/grimoire-before-kpm-20261024/`. See TESTING-rc1.md for
the initial filesystem restriction and the KPM 0.2.2 local-file upgrade limitation.

## Release gate

Run `make -j4 check`: sanitizer units, headless scenarios, both themes/languages, package and
launcher checks, KPM/Lua tests, share/QR/Effects/Copy checks and host export. PR checks record the
result for the current source. The share-test driver owns its server's lifetime; it no longer
exits on the old 25-second timer while slower cloud downloads are still running.

Physical keyboard/controller behavior and multi-firmware KOReader/native-PIN handover remain
hardware checks. Public HTTPS feed installation follows publication; the local repository path
has been exercised on the device.
