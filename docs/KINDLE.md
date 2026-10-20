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
grimoire/app/BUILD_ID, CAPABILITIES   the build unpacked there; what the launcher supports
grimoire/app/licenses/         the licenses of the fonts and code in the app
grimoire/notebooks/<id>/notebook.ini, p<N>.qpg, thumb.pgm   (the format: docs/FORMAT.md)
grimoire/settings.ini, folders.ini, sleep.pgm, grimoire.log
grimoire/.passcode-paused      there while the launcher has the Device Passcode paused
grimoire/trash/                deleted notebooks and pages
grimoire/PDFs/                 PDF mirror (USB road); kept out of documents/ so the Kindle never indexes it
```

## Leaving

- **Library > Exit** saves everything and returns to the Kindle home screen.
- **Holding the power button 4 seconds** saves and leaves from any screen, asleep or awake, a
  locked Kindle included (awake, a "Saved. Leaving Grimoire." toast shows first). A Kindle that
  stays locked after a press (no `outOfScreenSaver` within 3 s, the power service still in its
  screensaver) keeps Grimoire on its sleep screen; the second such press in a row saves and leaves
  as well, for a power key that cannot report a hold.
- The **power button** puts the Kindle to sleep *in* Grimoire: the sleep screen is the page you were
  on (or a pinned page, or the Kindle's own screensaver: Display > Sleep screen), and waking returns
  to it. Grimoire listens to the power service like KOReader does: a shell loop of one-shot
  `lipc-wait-event -s 0 com.lab126.powerd goingToScreenSaver,outOfScreenSaver` behind a TERM trap,
  so it dies with Grimoire and takes lipc-wait-event along. A run that fails is logged as
  `input: power service watcher failed, retrying (lipc-wait-event exit status N; ...)` and tried
  again 2 s later. Its own "Sleep after" timer, and a press the power service did not follow with
  a sleep within 2.5 s, ask for sleep with `lipc-set-prop -i com.lab126.powerd preventScreenSaver 0`
  and then `powerButton 1`, from one shell. For a press the power service did not answer (no
  sleep within 2.5 s, or no wake within 3 s), Grimoire first reads
  `lipc-get-prop -s com.lab126.powerd state`: it never draws over a Kindle in its screensaver,
  and never sends a power button to one.
- Ink is also autosaved 2 s after the pen lifts and at every press of the power button, so a
  crash or a forced reboot loses at most the last stroke. A page that fails to save stays open
  (with a warning) instead of being dropped, and a page file that cannot be read is renamed
  `p<N>.qpg.damaged-<time>` rather than overwritten. A page from a newer Grimoire is not damage:
  it opens read-only, and its file is never written.
- When Grimoire exits while the Kindle is locked (power service state `screenSaver`,
  `readyToSuspend` or `suspended`), the launcher does not write the saved screen back, because it
  would cover the screensaver or the passcode prompt. Pillow, awesome, the stopped status jobs and
  the home booklet are restored as usual. In stop-framework mode only a log line changes. Every
  exit logs `power state at exit: X`.
- The launcher's watchdog kills a Grimoire that stops answering for 15 s (so a hang never
  leaves the Kindle paused), with a grace period after the device wakes from suspend.

## The Device Passcode

With a Device Passcode set, the Kindle holds back `outOfScreenSaver` until the passcode is typed,
and its keypad cannot take a touch while Grimoire grabs the touch screen: a Kindle that locked
under Grimoire could not be unlocked, and on leaving, the screen saved at the start went back over
the passcode keypad. So the launcher pauses the passcode for the session, as KOReader's launcher
has done since 2016:

- At the start, if `/var/local/system/userpasswdenabled` exists, `app/grimoire.sh` writes the
  marker `grimoire/.passcode-paused` and removes the flag (`device passcode: paused while
  Grimoire runs`). At the end of `restore()` it touches the flag back and removes the marker
  (`device passcode: back on`); in stop-framework mode that happens before `start lab126_gui`,
  which reads it. `restore()` runs however Grimoire ends: Library > Exit, the 4 s hold, a crash,
  the watchdog.
- A marker found with no flag at the next start means a session ended without `restore()` (the
  Kindle restarted under Grimoire). The launcher then does not turn the passcode back on, since it
  may have been changed or turned off since, and logs `device passcode: left off by a session
  that did not end (a restart?); not turned back on here`.
- The binary learns which through `GRIMOIRE_PASSCODE`: with `paused` it shows "Your Kindle
  passcode is paused while Grimoire is open." the first time (settings.ini `passcode_told`), with
  `left-off` "The Kindle passcode was left off. Turn it back on in Kindle Settings."
- While Grimoire is open the passcode does not protect the Kindle; the README's Privacy section
  says so.

## The launcher

`documents/Grimoire.sh` takes flags for tools such as the planned KOReader plugin:

- `Grimoire.sh --unpack-only` migrates from Quire and unpacks when BUILD_ID differs, starts
  nothing, and exits 0. Run again with the same build, it does nothing.
- `Grimoire.sh --reset-calibration` runs `app/grimoire.sh --reset-calibration` in the foreground;
  its exit status is the answer.
- Exit statuses: 0 done (for a plain start, Grimoire is on its way); 1 the app could not be
  unpacked, or the calibration could not be reset; 2 `--unpack-only` or `--reset-calibration` did
  nothing because the old Quire app is still running (a plain start returns 0 then); 3 refused
  inside KOReader.
- The KOReader guard: when `KOREADER_DIR` is set or `pidof reader.lua` succeeds, the launcher
  logs a line, prints the same text on stderr and exits 3. Only outside KOReader (a Library tap
  while a KOReader lingers) does it also show "Grimoire did not start: KOReader is running." with
  eips near the top of the screen, about 2 s later. Only `--unpack-only` and `--reset-calibration`
  pass the guard; 1.1 adds `--hosted`. `app/grimoire.sh` has the same guard.
- Capabilities: line 6 of `Grimoire.sh` is `# Capabilities: guard unpack-only`, and
  `app/CAPABILITIES` carries the same words; line 2 of `app/grimoire.sh` is
  `# Capabilities: guard`, which describes that file. Tools must grep these lines and never run a
  launcher to find out, because an older launcher ignores a flag it does not know and starts
  Grimoire. Read the Capabilities line of the very file you are going to run, run
  `Grimoire.sh --unpack-only` only when its line lists `unpack-only`, and trust `app/CAPABILITIES`
  only after that unpack succeeded: an older launcher's unpack (1.0.0-beta, rc.1) leaves a newer
  CAPABILITIES behind. A missing line means no support.
- SH_Integration (v4.1.0) reads Name, Author, Icon and DontUseFBInk from the first six lines
  only, which is why they stay on lines 2 to 5 (package_check pins each line). It runs a
  scriptlet as `su -c 'sh -l <path>'` with no arguments, and runs xrefresh when the script ends.

## If something is off

- **Stuck**: hold the power button 4 seconds; Grimoire saves and leaves, and the launcher gives
  the Kindle its UI back. Never use the passcode reset code 111222777 to get out of anything: it
  erases the Kindle, notebooks included.
- **Strokes land away from the pen tip, or mirrored**: Library > Settings > Calibrate pen and
  touch. Tap five targets with the pen, five with a finger, check, Save. If even that is hard to
  reach, tap the **Grimoire - reset pen calibration** item in the Library.
- **The Kindle UI draws over Grimoire** (clock, status bar): the launcher now also stops every
  upstart job named like `*status*` / `*chrome*` and logs the jobs it saw running; Grimoire also
  repaints its top band whenever something else draws there. If it still shows, create an empty file named
  `stop-framework` in the `grimoire` folder. Grimoire then stops the Kindle UI entirely instead of
  pausing it (it takes a few seconds longer to come back afterwards).
- **Nothing happens when tapping Grimoire**: check `grimoire/grimoire.log`; the scriptlet logs whether it
  unpacked the app, and the app logs the device it found. `Grimoire.sh: not starting: KOReader is
  still running (reader.lua, pid N) ...` means a KOReader was left behind (after a crash, say):
  restart the Kindle.
- **"This page was made by a newer Grimoire. It is read-only here."**: the page came from a newer
  Grimoire; update Grimoire to change it. grimoire.log says what happened: `store: <path> opens
  read-only and stays as it is on disk` and `store: read-only page: <reason>` when such a page is
  opened, and `store: <id>: recovered p<n>.qpg from p<n>.qpg.damaged-<t>` when a page an older
  version set aside comes back. These appear once per run, not on every open or Library listing:
  `store: <id>: p<n>.qpg.damaged-<t> opens now, but p<n>.qpg exists too; both kept`,
  `store: <id>: p<n>.qpg is not in the page list; its set-aside copies stay aside (...)` and
  `store: <id>: <k> unknown notebook.ini line(s) do not fit in the 2048 bytes kept, left out`.
  The "damaged page was set aside" toast now appears only for real damage. A set-aside page that
  is no longer in the page list can be put back by hand, with Grimoire closed: rename it to
  `p<n>.qpg` and add `<n>` to the `pages=` line of its `notebook.ini`.
- **"The Kindle passcode was left off"**: the Kindle restarted while Grimoire had the passcode
  paused (see The Device Passcode). Turn it back on in Kindle Settings > Device Options > Device
  Passcode.
- **The screen**: the screen guard reads the framebuffer back at 32 bpp too (Scribe Colorsoft).
  It turns itself off after 10 repaints in a row and logs `screen guard: off after 10 repaints in
  a row (...)`. If the first frame and the next 4 refreshes all fail (retried once a second),
  Grimoire logs it and exits with status 1, so the launcher restores the Kindle UI. FBInk never
  reports 0 dpi: for a device it does not identify (named `Unidentified!` or `Unknown!`) it
  guesses 167 dpi. Grimoire then takes the dpi from the panel's physical size in vinfo (diagonal
  pixels over diagonal mm, or one side if only the width is known) when that comes to 100..600,
  else 300, and logs `display: FBInk does not know this device (...)`.
- **The Share screen** logs `share: QR code for http://<ip>:<port> (wlan0)`, or
  `share: no Wi-Fi address, so no QR code; showing ...` (USBNetwork only), and
  `share: laid out again (turned or woken); the server carries on` after a turn or a wake.
  `share: layout: ...` appears only when something on the Share screen does not fit (a line cut
  short, or two parts overlapping) and names the parts. There should be none; if one appears, it
  is worth reporting with the orientation and style.
- **Switches** (empty files in the `grimoire` folder): `screen-flip` / `screen-noflip` force the
  180 degree framebuffer correction on/off, `pen-mode` turns on the MTK low-latency pen mode for
  live ink (off by default: FBInk notes it skips the rotation stage at the Scribe's fb rotation),
  `stop-framework` stops the Kindle UI instead of pausing it, `tilt-flip` reverses the pen's
  tilt direction (only needed if the calligraphy nib turns the wrong way on your Kindle: it
  should give thick strokes down to the right when a right hand leans the pen that way),
  `watcher-multi` runs one continuous `lipc-wait-event -m -s 0` (KOReader's way) instead of the
  one-shot loop, for a device test (the log line `input: watching the power service (pid N): ...`
  says which mode is running).
- **The startup log** starts with a diagnostics block. `device:` lines give the model code (the
  first 6 characters of /proc/usid, never the serial), memory, CPU part and hardware. `display:`
  lines give the FBInk device id, platform, pixel format, color panel flag, raw vinfo and finfo,
  and the readback. `input:` lines give open errors, the power keys (a WARNING when there are
  several or none; the power key line ends with `, timed when read (EVIOCSCLOCKID failed)` if hold
  timing fell back to read times) and the orientation sensor check. A `settings:` line gives
  sleep_mode, sleep_minutes and side_button. The launcher's lines after `grimoire.sh: starting
  Grimoire` give `locale: LANG=...`, `device passcode: on` or `off` (then what it did with it),
  `power state: ...`, `blanket modules: ...` (marked `(Special Offers)` when the ad screensaver is
  loaded), `KOReader: installed, version <git-rev>` or `KOReader: not installed`, and
  `user id: N`; at exit, `power state at exit: X`. None of these identify the unit or a network,
  so the log can be attached to public issues (the Share screen's lines above do name the
  Kindle's local address).
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
  The optional full stop is `stop lab126_gui` / `start lab126_gui`. It also removes
  `/var/local/system/userpasswdenabled` for its session and puts it back at exit (PR #1861,
  2016), because the passcode holds back `outOfScreenSaver` until it is typed, which an app that
  grabs the input cannot let happen.
- Firmware 5.16.3+ userland is hard-float: Grimoire is a static musl ARMv7 VFPv4 AAPCS-VFP binary,
  so it needs no libraries from the device at all.

How Grimoire uses that:

- `src/display_fbink.c` writes the framebuffer directly (8 or 32 bpp, via `fbink_get_fb_pointer`)
  and refreshes with DU (live ink, in MTK pen mode), GL16 (UI, settling a finished stroke's grays),
  GC16 + flash (page turns, screen changes, every 30 UI refreshes).
- `src/input_evdev.c` finds devices by capability (not path), grabs pen + touch, watches the
  power key (its press, its release and a 4 s hold, timed by the kernel's event stamps), and maps
  axes through a calibration (default: straight onto the upright screen, swapped only if the axis
  ranges say so).
- `kindle/Grimoire.sh` is the scriptlet (Library entry, cover embedded as base64 per the
  kindlemodding.org scriptlet docs, `# DontUseFBInk` so its output does not draw over the
  screen); it detaches `kindle/app/grimoire.sh`, which does the KOReader-style pause/restore above,
  plus `preventScreenSaver` and the passcode pause, and opens the share port (8765, or
  `http_port` from settings.ini) on wlan0 only. That is why the Share screen's QR code holds the
  wlan0 address, and USBNetwork addresses are listed under it but not encoded.
- kindlemodding.org: on hdnext jailbreaks KUAL "is now obsolete and does not work"; scriptlets
  and KPM replace it.

Assumptions (cannot be verified without the device; all are logged at startup so `grimoire.log`
tells the story):

- Tilt: if the digitizer reports `ABS_TILT_X/Y`, resolution is units per radian; without a
  resolution a range within +-127 is read as degrees. If it reports no tilt, the tilt sliders
  simply have no effect and Pen feel says "tilt: not reported".
- Starting Grimoire with the Kindle held upside down: the framebuffer may be rotated; recalibrate.
- The power key reports its press and its release apart; the 4 s hold needs both. A key that
  reports them together makes every press look short (the `power button: up after N ms` lines
  show it); the two-press exit from a locked Kindle does not need the hold.
- Removing `/var/local/system/userpasswdenabled` pauses the passcode on 5.19 and 5.20 as it does
  for KOReader, and the power service's state names (`active`, `screenSaver`, `readyToSuspend`,
  `suspended`) are the same there.
- Accented notebook and folder names become UTF-8 file names under `grimoire/PDFs`; whether the
  Scribe shows them correctly over MTP is not checked yet.
