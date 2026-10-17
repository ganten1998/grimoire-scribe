# Grimoire on the Kindle Scribe

## Install

Grimoire targets the modern "hdnext" jailbreaks (Vera, WinterBreak, SpiderCat...). On those, KUAL is
obsolete; apps are **scriptlets**: `.sh` files in `documents/` that show up in the Library as
books and run when tapped (the jailbreak's SH_Integration does this).

1. Download the release zip (or run `make package` for `build/dist/Grimoire.zip`) and unzip it
   to get a `documents` folder with two files.
2. Plug in the Kindle. Newer Kindles speak MTP, not USB storage, so on a Mac use OpenMTP
   (`brew install --cask openmtp`).
3. Copy both files into the Kindle's `documents` folder. That is the whole install:
   `Grimoire.sh` carries the app inside it and unpacks itself into `grimoire/app` the first time you
   open it (and again whenever you copy over a newer `Grimoire.sh`).
4. Eject. **Grimoire** appears in the Library with its own cover (indexing can take a moment). Tap it.

Data lives in `/mnt/us/grimoire` (the `grimoire` folder on the USB storage):

```
grimoire/app/grimoire, grimoire.sh    the program and its launcher
grimoire/notebooks/<id>/notebook.ini, p<N>.qpg, thumb.pgm
grimoire/settings.ini, folders.ini, grimoire.log
grimoire/trash/                deleted notebooks and pages
grimoire/PDFs/                 PDF mirror (USB road); kept out of documents/ so the Kindle never indexes it
```

## Leaving

- **Library > Exit** saves everything and returns to the Kindle home screen.
- The **power button** puts the Kindle to sleep *in* Grimoire: the sleep screen is the page you were
  on (or a pinned page, or the Kindle's own screensaver: Display > Sleep), and waking returns
  to it. Grimoire listens to the power service (a shell loop of one-shot `lipc-wait-event
  com.lab126.powerd goingToScreenSaver,outOfScreenSaver`, which dies with Grimoire) like KOReader does, and its own "Sleep after"
  timer asks for sleep with `lipc-set-prop -i com.lab126.powerd powerButton 1`.
- Ink is also autosaved 2 s after the pen lifts, so a crash or a forced reboot loses at most
  the last stroke. A page that fails to save stays open (with a warning) instead of being
  dropped, and a page file that cannot be read is renamed `p<N>.qpg.damaged-<time>` rather
  than overwritten.
- The launcher's watchdog kills a Grimoire that stops answering for 15 s (so a hang never
  leaves the Kindle paused), with a grace period after the device wakes from suspend.

## If something is off

- **Strokes land away from the pen tip, or mirrored**: Library > Settings > Calibrate pen and
  touch. Tap five targets with the pen, five with a finger, check, Save. If even that is hard to
  reach, tap the **Grimoire - reset pen calibration** item in the Library.
- **The Kindle UI draws over Grimoire** (clock, status bar): the launcher now also stops every
  upstart job named like `*status*` / `*chrome*` and logs the jobs it saw running; Grimoire also
  repaints its top band whenever something else draws there. If it still shows, create an empty file named
  `stop-framework` in the `grimoire` folder. Grimoire then stops the Kindle UI entirely instead of
  pausing it (it takes a few seconds longer to come back afterwards).
- **Nothing happens when tapping Grimoire**: check `grimoire/grimoire.log`; the scriptlet logs whether it
  unpacked the app, and the app logs the device it found.
- **Switches** (empty files in the `grimoire` folder): `screen-flip` / `screen-noflip` force the
  180 degree framebuffer correction on/off, `pen-mode` turns on the MTK low-latency pen mode for
  live ink (off by default: FBInk notes it skips the rotation stage at the Scribe's fb rotation),
  `stop-framework` stops the Kindle UI instead of pausing it.
- Everything Grimoire and its launcher do is logged to `grimoire/grimoire.log` (rotated at 1 MB).

## How it fits in (facts vs. assumptions)

Facts (from source):

- FBInk (git master) identifies the Scribe 2 as "Pisco": MTK, Bellatrix3, 300 dpi, eclipse
  waveforms (`fbink_device_id.c`, IDs 0xFA0, 0xFA1, 0xFE5, 0xF9D, 0xFE4, 0xFE3, 0x102E, 0x102D).
- KOReader treats Scribe 1 and 2 identically (`frontend/device/kindle/device.lua`): touch panel
  at `/dev/input/touch`, a "scaled tablet" stylus (its ABS_X/ABS_Y maxima equal the
  framebuffer resolution, per FBInk's `INPUT_SCALED_TABLET`), Wacom tool protocol for the pen
  (`input.wacom_protocol = true`), MTK "fast mode" enabled so the driver does not promote
  partial refreshes to REAGL. No touch axis swap/mirror is configured for the Scribe.
- FBInk documents `fbink_mtk_toggle_pen_mode` for Bellatrix3: the low-latency pen path; while
  on, only DU updates are allowed.
- KOReader's launcher (`platform/kindle/koreader.sh`), upstart firmware (5.x incl. 5.16+, 5.19):
  dump `/dev/fb0`, `lipc-set-prop com.lab126.pillow disableEnablePillow disable`,
  `killall -STOP awesome` (5.7.2+), `stop statusbar` when `/etc/upstart/statusbar.conf` exists
  (5.17+), SIGSTOP `volumd`, open the firewall with `iptables -A INPUT -i wlan0 ...`; on exit undo
  all of it, restore the fb dump, re-enable pillow and start `app://com.lab126.booklet.home`.
  The optional full stop is `stop lab126_gui` / `start lab126_gui`.
- Firmware 5.16.3+ userland is hard-float: Grimoire is a static musl ARMv7 VFPv4 AAPCS-VFP binary,
  so it needs no libraries from the device at all.

How Grimoire uses that:

- `src/display_fbink.c` writes the 8 bpp framebuffer directly (via `fbink_get_fb_pointer`) and
  refreshes with DU (live ink, in MTK pen mode), GL16 (UI, settling a finished stroke's grays),
  GC16 + flash (page turns, screen changes, every 30 UI refreshes).
- `src/input_evdev.c` finds devices by capability (not path), grabs pen + touch, watches the
  power key, and maps axes through a calibration (default: straight onto the upright screen,
  swapped only if the axis ranges say so).
- `kindle/Grimoire.sh` is the scriptlet (Library entry, cover embedded as base64 per the
  kindlemodding.org scriptlet docs, `# DontUseFBInk` so its output does not draw over the
  screen); it detaches `kindle/app/grimoire.sh`, which does the KOReader-style pause/restore above,
  plus `preventScreenSaver`, and opens TCP 8765 on wlan0 for the share screen.
- kindlemodding.org: on hdnext jailbreaks KUAL "is now obsolete and does not work"; scriptlets
  and KPM replace it.

Assumptions (cannot be verified without the device; all are logged at startup so `grimoire.log`
tells the story):

- Tilt: if the digitizer reports `ABS_TILT_X/Y`, resolution is units per radian; without a
  resolution a range within +-127 is read as degrees. If it reports no tilt, the tilt sliders
  simply have no effect and Pen feel says "tilt: not reported".
- Starting Grimoire with the Kindle held upside down: the framebuffer may be rotated; recalibrate.
