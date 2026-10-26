# Device testing

What was checked on a real Kindle, on which Kindle and firmware, and what still needs a device.
`make check` covers the logic, the package, the launchers and their failures. It cannot prove what
only a Kindle shows: the screen, the pen, the passcode keypad, KOReader and KPM on the device.

2.0.0-beta.1 includes release candidate 1.1.0-rc.3, updated documentation, reproducible release
packaging, retuned pen defaults and the gray-paper repair described below. While shortcuts are
enabled, key and controller presses now keep the Kindle awake; "Annotate this page in Grimoire"
refuses a notebook's own PDF from `grimoire/PDFs`. Those last two fixes have automated coverage
but have not been confirmed on a Kindle. The candidates 1.1.0-rc.1 to rc.3 were never published;
their results are below, newest first.

## 2.0.0-beta.4 candidate

The offscreen CPU measurements and output-equivalence checks are in
[INK-PERFORMANCE.md](INK-PERFORMANCE.md). They do not establish visible pen latency.
The input-backlog scenario verifies intermediate live refreshes, all stored samples and
unchanged rendering. Share checks exercise queued and simultaneous PDF downloads, completed
page counts, sent-byte counts, unchanged PDF bytes, failed requests and HEAD responses.

Live frames over gray paper now move each pixel once, toward its value (see "Gray paper" in
INK-PERFORMANCE.md). `tests/test_fbpack.c` checks that on every paper gray, normal and
inverted, at 8 and 32 bpp, and that black-and-white content keeps the old dither exactly.
The device check: write on a white page and on a gray PDF page; the gray page should no longer
lag behind the white one or show white specks around the stroke before it settles.

Physical pen feel and the opt-in MTK pen-mode change still need a device comparison.
No default pen-mode change has been made. The owner has not yet tested this candidate.

## 2.0.0-beta.3

A fresh SSH snapshot captured all 130 current pen settings from the Kindle on 2026-10-25.
The pen-only fixture checks new-install defaults and Reset tool independently of the compiled
values; saved version-2 profiles keep their own values. The settings/page versions and stored
stroke parameters are unchanged. Whole-stroke erasing remains the default.

Empty PDF-mirror folder cleanup is checked through real Library Remove and Rename actions,
including failed saves, nonempty directories and symlinks. The core check also covers sanitized
name collisions and directory aliases. Cleanup removes only an empty directory.

The maintainer confirmed that writing and Reset tool matched the new tuning on the first
candidate. The first TEST-and-block pad passed both eraser modes and both reset controls.
At the maintainer's request, that pad has now been replaced with their blackletter “erase me!”
from page 4 of October-2026-02. The embedded copy contains only ink and page data, with no notebook
identifier or title. The source notebook is never read or changed by the practice pad.

The sample retains all 32 original strokes, their pressure and tilt, and is fitted and centered
in portrait and landscape. Whole-stroke erasing uses the editor's swept stroke-hit test;
Precise restores the bare ruled pad. Reset pad restores the sample; Reset tool also restores
whole-stroke mode. Automated checks cover empty areas, partial and swept hits, precise erasing,
reset and both orientations.

Pen feel's fine slider gesture is checked with both pen and touch, including exact values,
fractional motion, moving away without a jump, catch-up and direct tracking within the knob
band, repaint throttling, release, bounds,
a new gesture and saved values. Tool-picker checks cover the fixed eraser, pinned pens, the
two-tap top pen button, the direct grid button and the remembered eraser fallback after restart.
On revision `4d7b593`, the maintainer confirmed the centered handwriting, both eraser modes,
Reset pad and both two-tap picker paths. Fine control worked too; the maintainer then requested
a knob-height band that catches up and tracks the pointer directly, with fine control outside
it. That refinement still needs confirmation on the Kindle.

## The Kindles

| Date | Kindle | Firmware | KOReader | KPM | Build |
|---|---|---|---|---|---|
| 2026-10-24 | Kindle Scribe 2024 | 5.19.6 | v2026.07.2 | 0.2.2 (CLI 1.0.0), `kindlehf` | 1.1.0-rc.1, 1.1.0-rc.2 |
| 2026-10-25 | the same Kindle Scribe 2024 | 5.20.1.0.2 | v2026.07.2 | 0.2.4 | 1.1.0-rc.3 |
| 2026-10-25 | the same Kindle Scribe 2024 | 5.20.1.0.2 | v2026.07.2 | 0.2.4 | 2.0.0-beta.1, 1.2.0 |

## 1.2.0 on firmware 5.20.1.0.2

The Index/page-control candidate at `295f811` passed the full Mac and cloud checks; all three
cloud packages matched the Mac build byte for byte. It was installed through KPM 0.2.4 after a
fresh backup. All notebook, template and settings hashes and KOReader process IDs were unchanged.
The maintainer confirmed New page, Copy page with independent ink, Index jumps, naming pages and
names surviving reopening.

The maintainer then requested removal of the floating Back and reset-zoom controls. Index jumps
and pinch zoom now leave the page clear; pinch inward returns to the fitted view. The focused
interaction checks cover jumps with unsaved ink, failed saves and the former button positions.

The highlighter now has a fixed vertical tip. The maintainer explicitly accepted that old
highlights render with the new shape, tracked in issue #5. Tests check the tip's proportions and
fixed orientation, live/saved agreement and erasing/compaction. The original render-reference
tables stay in place with explicit exceptions only for highlighter-containing renders; other
tools, paper and serialized file bytes keep their old references. On revision `d57c755`, the maintainer confirmed that both floating buttons were gone and the
vertical highlighter looked right. Release 1.2.0 contains these tested changes with a new version label.

## 2.0.0-beta.1 on firmware 5.20.1.0.2

The candidate at `d4e0afd` was installed through the local KPM test feed as package 2.0.1001.
A fresh backup was taken first. Notebook-page and settings hashes, and KOReader's process IDs,
were unchanged after installation. The installed ARM executable matched the checked build.

- The maintainer opened Grimoire from Tools > Grimoire, wrote in it and returned to KOReader.
  Grimoire exited with status 0.
- Fresh SSH connections to KOReader's port 2222 and the independent development listener on
  port 2223 succeeded in all 90 samples before and during that session. Both listeners kept
  their original process IDs. Fresh connections to both also passed after the return.
- The maintainer quit KOReader and opened Grimoire from the Kindle Library. The installed
  Grimoire plugin was updated. Two short power-button sleep/wake cycles appear in the log; the
  maintainer confirmed writing resumed and the Kindle home screen returned correctly on Exit.
- With Device Passcode enabled, the maintainer confirmed the native PIN screen, successful
  unlock and writing afterwards, both upright and sideways, from a Kindle Library launch.
- Share served a PDF successfully. Preparing a 99-page notebook took a long time; visible export
  progress is requested. Returning to an annotated PDF page preserved the writing.

Writing over gray parts of a PDF page showed a temporary shadow in normal and inverted view.
The framebuffer path dithered the whole dirty rectangle, including unchanged image paper. A
candidate change keeps the previous logical frame and dithers only changed pixels. Its packing
tests cover both pixel formats, rotation, inversion, overlapping updates and grayscale settling;
removing paper preservation makes those tests fail. On the repaired build at `9c10591`, the
maintainer reported the shadow was much better in the same PDF page. Pen lag was unchanged and
was more noticeable over gray images than over black-and-white content. This is an improvement
to the paper artifact, not a claim that PDF handwriting latency is resolved.

The maintainer's current pen settings were read from the Kindle before the checkpoint. All 130
values across the ten tools are the fresh-install and Pen feel > Reset tool defaults. A pen-only
snapshot in `tests/fixtures/pen-defaults.ini` checks those values independently of the compiled
defaults, and checks that a saved version-2 profile keeps its own values. The settings version
and the frozen original handwriting-render tests are unchanged.

An unexpected restart reported after a long sleep was investigated separately. The retained
system log records Amazon's feature-flag manager requesting a restart on suspend, followed by
system shutdown. Grimoire exited during that shutdown. The kernel records a software restart;
no Grimoire crash trace or watchdog kill was found for that event. The recorded restart was at
11:19; the maintainer reported seeing a restart screen on waking later. These logs do not prove
the cause of every possible sleep failure.

## 1.1.0-rc.3 on firmware 5.20.1.0.2

**The hand-over from KOReader.** Firmware 5.20.1.0.2 adds a virtual input device,
`kindle-input-pt`, with keyboard keys and no absolute axes. KOReader keeps it open through its
exit, after it has let go of the pen and the touch screen. The 1.1.0-rc.2 launcher refused any
input device KOReader still held, so Grimoire never started from KOReader.

The launcher now lets a held input device through only when sysfs reports an absolute-axis
bitmap of zeros. It checks every held device, so a keyboard cannot hide a held pen or touch
screen later in the list. A missing, empty, blank or malformed bitmap is still a refusal.
`tests/launcher_check.sh` runs these cases under sh and dash, next to the existing ancestry,
session, passcode, installer and recovery cases.

- The launcher fix was installed on its own first, with the original kept in
  `kindle-lab/fw520-launch-fix/` on the Kindle. The maintainer confirmed opening Grimoire from
  KOReader and returning to KOReader.
- grimoire.log records the first frame, the pen and touch grabs and a clean status 0 return on
  5.20.1.0.2. It also shows the pen moved from event5 to event6. Grimoire finds its devices by
  capability, not by number, so that changes nothing.
- 1.1.0-rc.3 was then installed through KPM 0.2.4 (package version 1.1.5003). The notebook file
  hashes, the settings and the running KOReader's process IDs were unchanged.

**KOReader's incomplete first screen.** After a restart on 5.20.1.0.2, KOReader drew its very
first screen with parts missing, with or without Grimoire. KOReader takes the virtual device for
an external keyboard. Its externalkeyboard plugin then sends PhysicalKeyboardConnected, and the
file browser sets itself up again after its first paint. This is the known upstream issue
https://github.com/koreader/koreader/issues/16283.

- A repair made on this Kindle only leaves `kindle-input-pt` out of KOReader's keyboard discovery
  and hotplug, and closes its unused descriptor. Real external keyboards keep their discovery,
  mappings and capabilities.
- With it, the maintainer confirmed a complete first screen after restarting KOReader, and a
  complete screen when coming back from Grimoire without opening and closing a menu.
- Fourteen isolated checks ran the patched plugin's discovery and hotplug paths, its native
  capability handling, real keyboards, mixed devices and the discarded descriptor. The Kindle's
  LuaJIT parsed the patch. The original and the patch are kept in the maintainer's KindleLab kit.
- Grimoire does not ship or install this repair. An earlier repaint workaround in Grimoire's
  hand-over did not help a fresh start and was removed. KOReader's page and menu refresh settings
  are unchanged.

**Build delivery.** `make dist` collects the installer ZIP, the optional patches ZIP, the KPM
package and manifests, the source revision and checksums. The 1.1.0-rc.2 build tested on the
device is kept separately. Source-only HID work from PR #2 is not in this release.

## 1.1.0-rc.2 on firmware 5.19.6

1.1.0-rc.2 removed the experimental KOReader DPI patch, its Settings control and its tests, and the
calibration startup tip. The Terminess font matching and dropdown battery spacing stayed.

On the device:

- KPM 0.2.2 upgraded the installed 1.1.0-rc.1 (1.1.5001) to 1.1.0-rc.2 (1.1.5002) from a local
  test feed (`grimoirelocalrc`). After the cache cleanup was added, installing the package ID again
  refreshed the payload and removed the old DPI script from Grimoire's app cache.
- The installed ARM binary reported `1.1.0-rc.2`. The DPI script was absent from KOReader's active
  patches folder and from the app cache, and Terminess stayed installed. The settings hash matched
  the snapshot taken before the upgrade. The deployment check opened no Grimoire screen.
- The maintainer confirmed opening Grimoire from KOReader and returning to KOReader.
- After the startup tip was removed, the package was installed again with a fresh settings
  snapshot. The installed ARM SHA256 matched the tested build, and KOReader's process IDs stayed
  unchanged.
- The Gothic spacing and session guard build was installed through KPM as well. Its ARM SHA256 is
  `e6ea6cddaa8294890022446579835b50b1d53ad8cb1cf7dd48295a24bbc85877`. The settings hashes and
  KOReader's process IDs stayed unchanged. The ZIP and the KPM package carry the same launcher and
  ARM payload.
- An isolated Bluetooth HID bridge (upstream v3.18.0) found the maintainer's 8BitDo Arcade Stick
  over Classic Bluetooth, but authentication failed before an input node was ready. Its "paired"
  answer was not taken as proof of usable input. The bridge was stopped, and the Bluetooth
  services and their off state were restored. No service, udev rule, button mapper or KOReader
  plugin was installed. Grimoire ships no Bluetooth pairing, and the Arcade Stick is unconfirmed.
- The launchers from before KPM stay backed up in `kindle-lab/grimoire-before-kpm-20261024/`.

On the host:

- The Gothic Library has 24 design pixels under its header ornament, and its grid position and
  row count use the same offset. Classic is unchanged. Simulator screenshots in portrait and
  landscape check the gap, and the Library scenarios check card taps.
- The ZIP, the embedded app archive and the KPM payload hold only the three supported patches. The
  launcher removes the byte-exact 1.1.0-rc.1 DPI script from its own app cache, also when the
  build is already unpacked. Changed cache files and KOReader's active patches are left alone.
- The manual scriptlets check for a running Grimoire before they unpack, and both calibration
  resets refuse while Grimoire runs. Launcher cases force a stale build with a live session and
  check that the cached binary, build marker, settings and PID stay unchanged. A stale PID that is
  not running does not block an unpack.
- KPM's installer bookkeeping lives in `/tmp`, so a first launch can still move an old `quire/`
  folder; a regression case checks it. KPM has ten host tests.
- The scenario runner's selftest now expects no startup toast. Its old expectation was the
  failure in cloud run 40109165066.
- The share test keeps its server until its driver stops it, instead of a 25-second timer that
  slower cloud runs outlasted.
- CI installs ShellCheck and Luacheck and checks the KPM hooks too. The Lua check ignores only
  framework methods' implicit self and allows translation strings up to 200 columns.

## 1.1.0-rc.1 on firmware 5.19.6

**The KPM lifecycle, isolated.** A temporary userstore root held sentinel notebook and settings
files. No Grimoire screen was opened, no pen or touch was grabbed, and no KOReader hand-over ran.
Through the real KPM registry and hooks:

- The generated local `.kpkg` installed with `file://`.
- Both Library launchers appeared only in the isolated documents folder.
- `kpm launch grimoire --unpack-only` unpacked the embedded ARM payload.
- The ARM binary's `--version` reported `1.1.0-rc.1` without opening the framebuffer or input.
- Uninstall removed the package registry entry and the unchanged test launchers.
- The notebook sentinel, the settings and the unpacked app stayed.
- The hashes of the installed Library launchers stayed the same, and so did KOReader's process
  IDs.

KPM printed a harmless `doFullScan` unavailable message during install and removal while KOReader
ran; both returned success.

**The KOReader patches.** The 530 Lua patch checks passed on the Kindle's own LuaJIT. The updated
Terminess patch went into KOReader's active patches folder after the old file was compared byte
for byte with the one the 1.1.0 betas shipped. The original is backed up in
`/mnt/us/kindle-lab/grimoire-patch-check/backup-20261024/`. After a KOReader restart the
maintainer confirmed the battery spacing looks right. This candidate also carried a DPI patch,
measured on the device and removed again in 1.1.0-rc.2.

**Upgrades.** The published 1.1.0-beta.2 installer, wrapped in a test-only KPM package (1.1.1002),
could not be upgraded by installing the 1.1.0-rc.1 file over it: KPM 0.2.2 failed in its
dependency graph before the hooks ran. From a temporary local feed with the same schema as the
generated ones, installing `grimoire` again selected 1.1.5001 and ran the upgrade hooks. The
unpacked ARM binary changed to the candidate, the sentinels survived, and no KOReader plugin was
installed. The uninstall and reinstall fallback in [KPM.md](KPM.md) also kept the sentinels.

**The first real install.** `;kpm install grimoire` from the search bar reached KPM and picked the
candidate, but failed moving the launchers from `grimoire/.kpm-install` into Documents. KPM rolled
back and kept the 1.1.0-beta.2 app. A probe showed the Kindle refuses moves into Documents from
another folder, while a rename inside it works. The hook now stages its files beside the
launchers, and a regression case covers that boundary and a failed second rename. The retried
install, with the old launchers backed up in `/mnt/us/kindle-lab/grimoire-before-kpm-20261024/`,
registered `grimoire|1|1|5001`. The Library launchers were KPM's, the unpacked binary reported
`1.1.0-rc.1`, and the settings hash and KOReader's process IDs were unchanged.

**On the host.** 24 sanitizer unit test programs (46,779 checks), every scenario, the language and
fit checks in both styles and languages, the KOReader manager (40), shortcuts (26), power (104),
KPM (9) and Lua patch (530) tests, the share and export checks and actionlint all passed.

## Still needs a device

- **The passcode hand-over.** Grimoire keeps the Device Passcode on for a Library start and hands
  the screen and touch input to the Kindle's keypad on sleep. This has not been confirmed on a
  Kindle on any firmware (step 2 below).
- **Keyboards and controllers.** A USB or Bluetooth keyboard, 8BitDo enumeration, button numbers,
  hotplug, modifiers and recovery after a disconnect; the passcode keypad with a controller
  attached; and key and controller presses keeping the Kindle awake. Analog sticks and hats are
  not supported.
- **The KOReader hand-over elsewhere.** The testers' setups where the 1.1.0-beta integration
  failed, other firmware versions, KUAL's start without the framework, and the KOReader wallpaper.
- **Annotate this page** refusing a notebook's own PDF from `grimoire/PDFs`, while a copy of it
  elsewhere is still annotated.
- **KPM feeds.** Installing from the public HTTPS feed once a release is published, and Library
  indexing with KOReader closed.

## Checks on a Kindle

Keep a notebook backup, and use Library > Exit or hold power for 4 seconds to leave.

1. **KOReader hand-over.** Install the new Grimoire.sh. Add the plugin with Library > Settings >
   KOReader > Install / update Grimoire plugin, or copy it in by hand; a Library start alone does
   not add it. If the plugin is already installed, open Grimoire once from the Kindle Library so
   it is refreshed. Restart KOReader. From an open PDF and an EPUB, try Tools > Grimoire > Open
   Grimoire, then Annotate this page in Grimoire. Write, return, and check you are back at the same
   book and position. Annotate the same page again and check the ink is still there. Try an
   ordinary start and KUAL's start without the framework if you have it. KOReader in
   `extensions/koreader` or `documents/koreader` is found too.
2. **Passcode.** Turn on Device Passcode in the Kindle's Settings and open Grimoire from the
   Library, with KOReader closed. Write in a notebook, sleep and wake. The Kindle's keypad should
   appear; waiting or touching it must never open the notebook. After the right passcode, the
   notebook and its ink should come back. Try portrait and landscape, a second sleep, and sleeping
   with the pen still down. The next stroke and the first finger tap after the unlock should work
   without stray ink. Leave from the lock screen with a 4-second hold and check the Kindle's own
   screens and passcode still work. A Grimoire opened from KOReader normally runs with the passcode
   off, as KOReader turns it off; it is not this test.
3. **Many pages.** Open a large notebook with heavily inked pages. Open Pages, move between grid
   screens, tick and untick pages, cancel, and use Go to page... for an early and a late page. The
   numbered grid should appear without drawing previews. Delete only disposable test pages.
4. **Browser.** Share a large notebook to a phone and a computer. The field at the top should show
   the page and the total, take a number with Enter, and load only that page. Check Previous, Next,
   the browser's Back, a full-size PNG, the PDF, the zip and Effects. Try a narrow phone screen.
5. **Patches.** In Library > Settings > KOReader, turn each patch on and restart KOReader. Check
   the screenshot gestures, the square folder corners and the menu fonts. Turn them off and
   restart: KOReader's defaults should come back, and book fonts should not change. Remove the
   Grimoire plugin from this sheet and check it stays removed after another Library start, then
   install it again from the sheet.
6. **Wallpaper.** Set a custom picture, a random image folder or the book cover in KOReader. Open
   Grimoire through its plugin and pick Display > Sleep screen: KOReader wallpaper. Sleep in
   portrait and landscape. Check the picture, and that the page comes back on wake. Turn off the
   custom picture in KOReader and hand over again: Grimoire should show the current page. The
   picture is a snapshot taken at each hand-over, not KOReader's live sleep screen.
7. **Keyboards and controllers.** Turn on Library > Settings > Shortcuts > Enable shortcuts with a
   keyboard or controller attached. Check the last key or button code on the sheet, each mapped
   action in a notebook, and that pressing keys or buttons for longer than Display > Sleep after
   keeps the Kindle awake.

For a failure, write down the Scribe model, firmware, jailbreak, KOReader version, where KOReader
is installed and how Grimoire was started. Attach `grimoire/grimoire.log`, and for a hand-over
failure `koreader/crash.log`. Read the logs before sharing them; never include a passcode, a
serial number or a book file.
