# Grimoire on the Kindle Scribe

## Install

Grimoire targets the modern "hdnext" jailbreaks (Vera, WinterBreak, SpiderCat...). On those, KUAL is
obsolete; apps are **scriptlets**: `.sh` files in `documents/` that show up in the Library as
books and run when tapped (the jailbreak's SH_Integration does this).

1. Download the release zip (or run `make package` for `build/dist/Grimoire.zip`) and unzip it
   to get a `documents` folder with two files, and a `KOReader plugin (optional)` folder for a
   hand install of the KOReader plugin (see KOReader below).
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
grimoire/app/koreader/grimoire.koplugin   KOReader's Grimoire plugin, as the app carries it
grimoire/app/licenses/         the licenses of the fonts and code in the app
grimoire/notebooks/<id>/notebook.ini, p<N>.qpg, thumb.pgm   (the format: docs/FORMAT.md)
grimoire/settings.ini, folders.ini, sleep.pgm, grimoire.log
grimoire/.passcode-paused      legacy beta.1 marker, consumed for its recovery notice
grimoire/.passcode-left-off    counts the starts told that a restart left the passcode off
grimoire/.koplugin-installed   the build whose plugin went into KOReader, or "removed"
grimoire/trash/                deleted notebooks and pages
grimoire/PDFs/                 PDF mirror (USB road); kept out of documents/ so the Kindle never indexes it
```

With KOReader installed (`/mnt/us/koreader`), Grimoire also has these there:

```
koreader/plugins/grimoire.koplugin     the plugin, installed (Tools > Grimoire in KOReader)
koreader/settings/grimoire_launch.txt  the plugin's launch marker, while a hand-over is under way
```

and one key in KOReader's own settings, `grimoire_start_with`, while a hand-over borrows
KOReader's `start_with` for its next start (KOReader below).

## Leaving

- **Library > Exit** saves everything and returns to the Kindle home screen. Opened from KOReader
  (KOReader below), it reads **Back to KOReader**: it and the 4 s hold end Grimoire the same way,
  and KOReader starts again in its place.
- **Holding the power button 4 seconds** saves and leaves from any screen, asleep or awake, a
  locked Kindle included (awake, a "Saved. Leaving Grimoire." toast shows first). A Kindle that
  stays locked after a press (no `outOfScreenSaver` within 3 s, the power service still in its
  screensaver) keeps Grimoire on its sleep screen; the second such press in a row saves and leaves
  as well, for a power key that cannot report a hold.
- When the open page cannot be saved (a full userstore), leaving does not claim it was. Awake,
  the first hold stays in Grimoire and shows "Could not save this page. Hold power 4 s again to
  leave anyway." A second hold within a minute leaves, showing "Leaving Grimoire without this
  page." From the sleep screen, and for the two-press exit of a locked Kindle, it leaves at once.
  Each such try logs `leaving: the open page could not be saved`, and the first awake hold adds
  `leaving: staying this once (...)`.
- The **power button** puts the Kindle to sleep *in* Grimoire: the sleep screen is the page you were
  on (or a pinned page, or the Kindle's own screensaver: Display > Sleep screen), and waking returns
  to it. Opened from KOReader, the Kindle's own screensaver cannot show (it needs the Kindle UI,
  which KOReader keeps paused), so the page does instead, and the log says so once: `sleep: the
  Kindle's own screensaver needs the Kindle UI, which KOReader keeps paused: the page instead`.
  Grimoire listens to the power service like KOReader does: a shell loop of one-shot
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
  (with a warning, and the autosave tries again every 2 s) instead of being dropped, and a page
  file that cannot be read is renamed `p<N>.qpg.damaged-<time>` rather than overwritten. A page
  from a newer Grimoire is not damage: it opens read-only, and its file is never written.
- When Grimoire exits while the Kindle is locked (power service state `screenSaver`,
  `readyToSuspend` or `suspended`), the launcher does not write the saved screen back, because it
  would cover the screensaver or the passcode prompt. Pillow, awesome, the stopped status jobs and
  the home booklet are restored as usual. In stop-framework mode only a log line changes. Every
  exit logs `power state at exit: X`.
- The launcher's watchdog kills a Grimoire that stops answering for 15 s (so a hang never
  leaves the Kindle paused), with a grace period after the device wakes from suspend.

## The Device Passcode

A Library start now keeps `/var/local/system/userpasswdenabled` intact throughout the session
(`GRIMOIRE_PASSCODE=enabled`). It refuses to start over a locked/unknown state and overrides
`stop-framework`: the framework must remain available for the keypad.

On `goingToScreenSaver`, Grimoire saves, releases the pen/touch `EVIOCGRAB`s, resumes awesome
and enables pillow. Native PIN takes priority over a page or wallpaper. While input is yielded,
pen and touch events are discarded without logging coordinates. On `outOfScreenSaver`, which the
Kindle sends after PIN validation, it disables pillow, stops awesome, drains old input, resets
contact state and reacquires the devices before drawing. These UI boundary calls are ordered,
bounded to 3 seconds, and a failure leaves Grimoire so the launcher's restore can return the UI.
The wake timer never infers native unlock from elapsed time or a guessed power state.

Beta.1 could leave the PIN off after a restart. Its `.passcode-paused` marker is still consumed:
with no passcode flag, Grimoire leaves the unknown PIN state alone and reminds the user to enable
it in Kindle Settings at up to three starts (`.passcode-left-off`). New sessions create no pause
marker and never recreate a passcode flag.

KOReader's own `koreader.sh` normally removes the passcode flag before its plugin can run.
Hosted Grimoire inherits that policy and does not guess whether a missing flag should be restored.
Use a Library start for native PIN protection. If a host preserves the flag, Grimoire supports
native unlock using that host's paused UI. The native keypad hand-over and reacquisition came in
1.1.0-beta.2. The maintainer confirmed the native keypad and writing after unlock upright and
sideways on a Scribe 2024 with firmware 5.20.1.0.2. The host tests verify ownership, order,
failure and timeout; other firmware still needs a device check ([TESTING.md](TESTING.md)).

## The language

The screens are in English or Spanish: settings.ini `language=` is `auto` (the default), `en` or
`es` (docs/FORMAT.md). For `auto` the binary looks once at startup, before it draws anything, and
the first of these that is set decides:

1. `GRIMOIRE_LANG`, should a launcher set it (none does today);
2. the Kindle's own language, the one its Settings picked: the `LANG=` line of
   `/var/local/system/locale`, or its `LC_ALL=` line if it has no `LANG=`;
3. `LC_ALL`, `LC_MESSAGES`, then `LANG` from the environment.

A language Grimoire does not speak (also `C` and `POSIX`) means English. The Kindle's file ranks
above the environment because KOReader's `koreader.sh` exports `LC_ALL=en_US.UTF-8` to what it
starts (hosted, the launcher drops that LC_ALL too: KOReader below). grimoire.log says what was
chosen and why, a line worth asking testers for:

```
i18n: language es (auto, from the Kindle's locale es_ES.UTF-8)
i18n: language en (auto, from LANG=en_US.UTF-8; /var/local/system/locale cannot be read)
i18n: language es (set in Settings)
```

A `language=` this version does not speak (a newer one's) works as `auto` and adds `; settings.ini
says language=xx, a language this version does not speak`. A Kindle set to Spanish gets every
screen, the share pages and the studio in Spanish; grimoire.log stays English, and so do paths,
file names and folder names. The launcher's own `locale: LANG=...` line reads the same file.

## KOReader

KOReader's Grimoire plugin (`kindle/koreader/grimoire.koplugin`: `main.lua` and `_meta.lua`,
GPL-3.0-or-later) opens Grimoire inside KOReader's session and brings KOReader back after it.

**What it offers.** On a Kindle (elsewhere it adds nothing), Tools > Grimoire has Open Grimoire,
Notebook PDFs (KOReader's file browser in `grimoire/PDFs`, closing an open book first, with a
note that Grimoire rewrites those files) and Reset pen calibration (after a confirmation it runs
`grimoire/app/grimoire.sh --reset-calibration`), and KOReader's gestures get an Open Grimoire
action (`grimoire_open`). Its words are its own, since KOReader's catalogs do not have them:
English and Spanish, picked by KOReader's language setting, English for any other.

**How it gets into KOReader.** A fresh installation leaves KOReader untouched. Use Library >
Settings > KOReader > Install / update Grimoire plugin, or manually copy the ZIP's
`KOReader plugin (optional)/grimoire.koplugin` into KOReader's `plugins` directory. Restart KOReader.
The three user patches are separate, optional choices. They are never installed by the launcher.

The app carries the plugin (`grimoire/app/koreader`). Once explicitly installed, a plain Library
start keeps that copy up to date with the app, copying each file then renaming it whole.
`grimoire/.koplugin-installed` records the build. Deleting the plugin marks it `removed`; it stays
removed until explicitly installed again. A start with KOReader absent forgets the marker, so a
later KOReader reinstall needs consent again. A plugin an earlier version installed keeps
receiving updates. No top-level KOReader folder is shipped over MTP.

KPM packaging and its install/upgrade/remove hooks are documented in [KPM.md](KPM.md).

**Open Grimoire, step by step.**

1. The plugin refuses at once, with a message, without `/var/tmp/koreader.sh` (the copy
   `koreader.sh` runs from while KOReader is up: this KOReader was not started by it), and when
   none of `STOP_FRAMEWORK`, `PILLOW_HARD_DISABLED` and `AWESOME_STOPPED` is `yes` (koreader.sh
   paused nothing, as from KPVBooklet).
2. With `documents/Grimoire.sh` there, its Capabilities line (in its first 10 lines) must list
   `unpack-only` and `hosted` (else "This Grimoire cannot open from KOReader: copy a newer
   Grimoire.sh into the Kindle's documents folder."), and "Preparing Grimoire..." shows while
   `sh documents/Grimoire.sh --unpack-only` unpacks a newer build ("Grimoire could not unpack
   itself." when that fails). Without it, the unpacked app is used if it is there ("Grimoire is not
   installed" if not). Either way `grimoire/app/grimoire.sh` must list `hosted` too.
3. The hand-over: the plugin writes the launch marker `koreader/settings/grimoire_launch.txt`,
   borrows `start_with` for KOReader's next start (`last` when a book is open; `filemanager` from
   the file browser when it was `last`), keeping the old value in `grimoire_start_with`, and wraps
   `Device.exit` for that one exit. Then it asks for KOReader's restart (`exitOrRestart` with
   `UIManager:restartKOReader()`).
4. KOReader closes the book or the file browser and runs its own `Device:exit()`, the only place
   it lets go of the screen, the pen and the touch screen. The wrapper runs after it, in a pcall:
   `sh grimoire/app/grimoire.sh --hosted koreader` in the foreground, then it deletes the marker if
   that returned 0.
5. reader.lua exits with code 85, KOReader's restart, and `koreader.sh` starts KOReader again in
   the same session. It kept the Kindle UI paused all along, so none of the Kindle's own screens
   show in between, and the passcode stays paused. KOReader's own start reopens the book
   (`start_with`), and the plugin puts `start_with` back.
   KOReader's Keep alive comes back too: grimoire.sh put back the preventScreenSaver KOReader had,
   but the flag KOReader keeps in memory (`PluginShare.keepalive`) is gone with the old process.
   With preventScreenSaver 1 and that flag off, KOReader's AutoSuspend spins once its suspend time
   passes without a sleep: its Kindle timer schedules itself in the past, so KOReader never reads
   input again and asks the power service `isCharging` in a loop (the KOReader hang of 2.0.0-beta.4,
   found 2026-10-26). So at each KOReader start, on a Kindle the power service keeps awake, the
   plugin turns the flag back on (`Grimoire: the Kindle is kept awake (preventScreenSaver 1):
   KOReader's Keep alive is on again`), once per run, through KOReader's own lipc handle. Keep
   alive then shows on, as it is, and its menu item turns it off.
   KOReader's rotation is set right too. On a Scribe it draws at its rotation less a native one it
   asks the window manager for as it starts (`com.lab126.winmgr` `accelerometer`: D or R make it
   upside down), but turns touches by its rotation alone, so the two agree only while that native
   rotation is the system's: the framebuffer's rotation (1: the framebuffer upside down to the
   touch panel, the rule Grimoire's display code follows). The window manager stays stopped through
   KOReader's session and Grimoire's; with the Kindle turned meanwhile (Grimoire follows the turn),
   it answers with a turn it never applied, and KOReader came back upside down with every tap
   landing opposite (seen 2026-10-26 19:37). So at each KOReader start on a Kindle Scribe
   (KOReader's `KindleScribe`), when the native rotation and the framebuffer's disagree, the plugin
   makes the native rotation the framebuffer's and sets KOReader's rotation again over it, the one
   it closed with (`Grimoire: KOReader's native rotation 2 against the framebuffer's 3 (drawing and
   touch apart): native rotation 0 and rotation 2 again`). Once per run.
   And KOReader is turned as the Kindle is held. The stopped window manager answers with how the
   Kindle was held before Grimoire, or with nothing (KOReader then starts upright), and the
   orientation sensor (`bma4xy_feature` on the Scribe 2, `kx132` on others) reports only turns, so
   a Kindle held still, upside down say, came back upright and stayed so until its rotation was
   chosen by hand (seen 2026-10-28). So the wrapper writes a second marker,
   `koreader/settings/grimoire_back.txt`, once Grimoire has run, whatever it answered; at the
   start that finds it (and deletes it), on a Kindle Scribe (`KindleScribe`, `KindleScribe3`,
   `KindleScribeColorSoft`) whose KOReader has an accelerometer, the plugin reads the sensor's
   current code on KOReader's next tick (after the native rotation above, and after the book or the
   file browser has set its own rotation): `EVIOCGABS(ABS_PRESSURE)` from the `/dev/input/event*`
   device with ABS_PRESSURE in a 10..30 range and neither ABS_X nor ABS_MT_POSITION_X, as Grimoire
   finds it. 15, 16, 17 and 18 (upright, upside down, landscape left, landscape right) become the
   `MSC_GYRO` event KOReader's `KindleGyroTransform` makes of them, handed to KOReader's own
   accelerometer handler (`Device.input:handleGyroEv`), and a `SetRotationMode` it answers is sent
   on as KOReader's main loop sends it. So KOReader's settings hold: with the accelerometer ignored
   nothing is read, under the orientation lock only a turn within portrait or within landscape is
   made, and a Kindle held as KOReader stands changes nothing. Only the rotation is set, never the
   native rotation, so drawing and touch stay together. One line in crash.log says what was read
   and done (`Grimoire: back from Grimoire, the orientation sensor reads 16
   (DEVICE_ROTATED_UPSIDE_DOWN): rotation 0 -> 2`).
6. A launch marker still there at that start means Grimoire never ran (grimoire.sh refused, found
   another Grimoire, or was never reached): the plugin deletes it and shows "Grimoire could not
   start. See grimoire/grimoire.log." over the book or the file browser.

From the file browser, `exitOrRestart` asks for the restart at once, so a failure there comes back
to the plugin's pcall and is undone (the `Device.exit` wrapper, the marker and the borrowed
`start_with`), followed by "Grimoire could not start". From a book, KOReader closes the book and
asks on its next tick, past that pcall: a failure then ends KOReader without the hand-over, and the
marker reports it at the next start, which also gives `start_with` back. A failure inside the
wrapper is caught by its own pcall, so reader.lua still exits with 85 and KOReader comes back.
KOReader's log (`koreader/crash.log`) gets `Grimoire: running ...` and `Grimoire: back in
KOReader, grimoire.sh status N`, and on trouble `Grimoire: the hand-over failed: ...`,
`Grimoire: could not restart KOReader: ...` or `Grimoire: the last hand-over did not run
Grimoire`.

**Hosted mode** (`app/grimoire.sh --hosted koreader`) refuses with exit 3, logging the reason as
`grimoire.sh: not starting...` and printing it on stderr, unless all of these hold:

- the host is `koreader` (`not starting: --hosted <name> is no host Grimoire knows (the one host is
  koreader).`);
- `KOREADER_DIR` is set (`not starting: --hosted koreader runs only inside KOReader, from its
  Grimoire plugin.`);
- reader.lua is among the launcher's parents, by their `/proc/<pid>/stat` names and cmdlines, and
  koreader.sh is an ancestor of reader.lua (up to eight launcher processes): a `/var/tmp/koreader.sh` left by a crashed session proves
  neither (`not starting hosted: KOReader's reader.lua (pid N) was not started by koreader.sh.`, or
  `... KOReader's reader.lua is not among this launcher's parents.`);
- every `/dev/input/` device among reader.lua's `/proc/<pid>/fd` entries reports no absolute axes:
  its `/sys/class/input/eventN/device/capabilities/abs` holds only zeros. An exclusive grab belongs
  to the open device, and a pen or touch screen has absolute axes. A key-only device may stay open:
  firmware 5.20.1.0.2 adds a virtual `kindle-input-pt` device with keyboard keys and no absolute
  axes, which KOReader keeps open through its exit. A missing, empty or malformed bitmap counts as
  a pen or touch screen, and every device is checked, so a keyboard cannot hide one later in the
  list (`not starting hosted: KOReader still has /dev/input/eventN open (this is not its exit
  path), so Grimoire would get no pen or touch.`). A list that cannot be read lets this through.

A KOReader launcher that did not pause the native UI is supported: hosted Grimoire pauses it
itself, restores it after the app ends, and returns without starting the home booklet. Otherwise
it borrows KOReader's pause. The new plugin retains the frameworkStarted LIPC handle across
Device:exit and the Grimoire run on no-framework starts, then closes it before KOReader restarts.


Right after these checks, before the lock wait and even when another Grimoire holds the lock, it
stops awesome again when KOReader had it stopped (KOReader's exit has just woken it, on Special
Offers Kindles), and normally leaves it stopped (native PIN handover may temporarily resume it). It also drops KOReader's `LC_ALL` and keeps `LANG`, so
Grimoire sees the locale a start from the Library sees. Of what a Library start does, it keeps the
lock (another Grimoire: `already running (launcher pid N), not starting another` and exit 1, which
leaves KOReader's launch marker), the priority, the share port's firewall hole, the watchdog and
the log, and preventScreenSaver 1 while Grimoire starts, putting KOReader's own value back at the
end. It stops the status jobs while Grimoire runs (the KOReader that KPM installs leaves the 5.17+
status bar running) and starts them again. It never touches the screen dump, pillow, the home
booklet, volumd, lab126_gui or the passcode: a `stop-framework` file is ignored (`the
stop-framework flag does not apply inside KOReader, which paused the Kindle UI itself`). It skips
the settle wait and runs Grimoire in the foreground with `GRIMOIRE_HOST=koreader`: Exit reads Back
to KOReader, and sleep mode 2 shows the page. Exit statuses: 0 Grimoire ran, 1 another Grimoire is
running, 3 refused.

Its log lines: `starting Grimoire hosted by KOReader (reader.lua pid N; KOReader stopped the
framework: yes|no, awesome: yes|no) on <firmware>`, `awesome stopped again, as KOReader had it`,
`device passcode: left to KOReader, which pauses it for its session`, and at the end `handing the
screen back to KOReader` and `power state at exit: X`. After KUAL's start without the framework
it adds `KOReader stopped the Kindle framework: the power button may not put the Kindle to sleep
in Grimoire`: KOReader drops its stand-in for the framework, which the power service wants before
it takes the power button, at its exit.

### SSH during a hosted session

An SSH server already started in KOReader can stay available while Grimoire runs. On the Scribe
2024 with firmware 5.20.1.0.2, KOReader v2026.07.2 and Grimoire 2.0.0-beta.1, fresh connections
to KOReader's port 2222 succeeded before the handover, throughout the Grimoire session and after
returning to KOReader. Its listener process stayed the same. A separate development listener on
port 2223 passed the same test. This lets a developer read logs or stage an update while writing.

Grimoire does not start an SSH server or keep Wi-Fi awake. The app must close before an update is
installed; its installer refuses to replace a running app. The test above covers the hosted
session with networking awake, not a reboot or deep sleep.

## The launcher

`documents/Grimoire.sh` takes flags for tools such as KOReader's Grimoire plugin:

- `Grimoire.sh --unpack-only` migrates from Quire and unpacks when BUILD_ID differs, starts
  nothing, and exits 0. Run again with the same build, it does nothing.
- `Grimoire.sh --reset-calibration` runs `app/grimoire.sh --reset-calibration` in the foreground;
  its exit status is the answer.
- `Grimoire.sh --hosted koreader` unpacks like `--unpack-only`, then runs `app/grimoire.sh
  --hosted koreader` in the foreground (no detaching, no plugin install): Grimoire inside
  KOReader's session (KOReader above). Its status is then that launcher's: 0 Grimoire ran, 1
  another Grimoire is running, 3 refused.
- Exit statuses: 0 done (for a plain start, Grimoire is on its way); 1 the app could not be
  unpacked, or the calibration could not be reset; 2 `--unpack-only`, `--reset-calibration` or
  `--hosted` did nothing because Grimoire, or the old Quire app, is still running (a plain start
  returns 0 then); 3 refused inside KOReader, or `--hosted` outside it. Past the unpack, `--hosted`
  answers with `app/grimoire.sh`'s status. A running Grimoire is found by the launcher's PID before
  anything is unpacked, and logged as `Grimoire is running. Close it before updating or resetting
  calibration.`; a background PDF export does not count.
- The KOReader guard: when `KOREADER_DIR` is set or `pidof reader.lua` succeeds, the launcher
  logs a line, prints the same text on stderr and exits 3. Only outside KOReader (a Library tap
  while a KOReader lingers) does it also show "Grimoire did not start: KOReader is running." with
  eips near the top of the screen, about 2 s later. `--unpack-only` and `--reset-calibration` pass
  the guard. `--hosted` goes the other way: it passes only with `KOREADER_DIR` set (else `not
  starting: --hosted is for KOReader's Grimoire plugin, inside KOReader.`, exit 3), and
  `app/grimoire.sh` checks the rest (KOReader above). `app/grimoire.sh` has the same guard for a
  plain start.
- Capabilities: line 6 of `Grimoire.sh` is `# Capabilities: guard unpack-only hosted`, and
  `app/CAPABILITIES` carries the same words; line 2 of `app/grimoire.sh` is
  `# Capabilities: guard hosted`, which describes that file. A tool reads the first
  `# Capabilities: ` line within the first 10 lines of `documents/Grimoire.sh`:
  `head -n 10 documents/Grimoire.sh | sed -n 's/^# Capabilities: //p'`. It must never grep the
  whole file: the archive appended to the scriptlet holds `app/grimoire.sh`, whose own
  `# Capabilities: guard hosted` line describes only that file, and a tool that kept the last match
  would think `unpack-only` unsupported (package_check pins that the header's line is the file's
  first).
  Nor may a tool run a launcher to find out, because an older launcher ignores a flag it does not
  know and starts Grimoire. Read the Capabilities line of the very file you are going to run, run
  `Grimoire.sh --unpack-only` only when its line lists `unpack-only`, and trust `app/CAPABILITIES`
  only after that unpack succeeded: an older launcher's unpack (1.0.0-beta, rc.1) leaves a newer
  CAPABILITIES behind. A missing line means no support.
- SH_Integration (v4.1.0) reads Name, Author, Icon and DontUseFBInk from the first six lines
  only, which is why they stay on lines 2 to 5 (package_check pins each line). It runs a
  scriptlet as `su -c 'sh -l <path>'` with no arguments, and runs xrefresh when the script ends.

## Status display

The default is a slim bar across the top of a notebook page. Its first line shows the notebook name,
clock and battery; its second shows page and section, outgoing and incoming link counts, the current
tool, screen sharing and any available controller. The page fits below the bar in either orientation,
and the bar cannot become ink. It stays visible while the writing hand rests on the screen.

Display's title row selects Bar, Corner or Off with a pen or a finger. The `status-mode` shortcut
cycles the three choices. In Bar or Off, `status-toggle` toggles the bar. In Corner it keeps the
existing tap and toggle behavior. The clock size and corner controls still apply to Corner mode.
Text refreshes only when its field changes; clock and battery refreshes wait while writing.
Long notebook and section names are shortened to fit. Live view's status switch hides the bar there.

### The clock and battery corner

A finger tap in a corner of a notebook page (a 14 mm square, the top left unless you pick another, or
where the indicator shows, with 4 mm around it, whether it is up or not) turns on a small indicator there: the time and date as Grimoire writes them elsewhere
(`Oct 10, 14:32`), a small battery drawn in pixel art that shows the charge (and a bolt over it while
it charges), and the battery's percent. Tapping the indicator, or its corner again, turns it off.
It is on the notebook page and nowhere else: not on the Library, Settings, Share or Pen feel, and not
while a sheet, a menu, the Links bar, Link to..., a link's long-press card, Find, a keyboard or the Pages view is in front of the page, so it
never gets in the way of another screen. The panel, or a link the way a finger tap would follow it,
keeps its own tap, except that a link in the corner is not followed (the corner's tap is the clock's).

One tap is one toggle, at once: the tap is decided when the finger lifts (it went down in the corner and lifts in it,
or within 5 mm of where it landed, in under 0.8 second; the path between is not asked, so a slow, hard press
that flattens and drifts still taps), and
nothing makes the next tap wait. A key or controller button can do the same: the `status-toggle` action,
and `status-corner` for the next corner (docs/SHORTCUTS.md, Buttons > Menus); with no page in front they
change the setting and say so with a toast. Settings > Display, beside Done, has the size (small,
medium or large) and the corner (top left, top right, bottom right, bottom left, in turn). They are
settings.ini `status_on`, `status_size` and `status_corner` (docs/FORMAT.md).

It is drawn over the screen for each refresh and never saved into it: it is not in a page, a
thumbnail, an export or PDF, Live view, or the sleep screen. It refreshes by itself at each minute and
when the percent or charging changes, one small grayscale refresh of its own box (two when it moves from
one corner to another: the new place and the old), and not while the pen is near, a stroke is being
drawn or the pen mode of the driver is on: it waits for the pen to leave. A finger the touch panel lets
through is never the pen's hand, so a tap takes back a pen's "near" that nothing ended. It never waits
for the display, and the setting is written to settings.ini after the refresh has gone out.

The battery comes from the kernel: the power supply whose `type` is `Battery` in
`/sys/class/power_supply/*` (its `capacity`, and `status` reading `Charging`), read every 10 seconds
while the corner is on. Where there is none, from the power service (`lipc-get-prop com.lab126.powerd
battLevel` and `isCharging`), at most once a minute and only while the pen is away, by a shell in
the background that Grimoire never waits for (one that takes over 2 seconds is stopped): its answer
shows from the next reading on. With neither, the corner shows the time alone, with no battery drawn.
The screen guard leaves the corner's box out of its checks, so it is not taken for something else
drawing over Grimoire.

## Links on a page (2.0.0-beta.6)

A link is an invisible area of a page that a finger's tap follows; making one leaves no ink. In the
Links bar, New link (or an add-link button) shows a strip along the bottom: circle what links with the
pen (a dashed line, gone at the lift), then pick where it goes in "Where does the link go?" (a new page,
the Index, a recent page, an Index entry, Search or Choose from the pages...). Areas in the bar shows each
link's area as a dashed box; it is never printed. If you want a visible border, draw it as ordinary ink.

Hold a finger on a link for about half a second: a heavy dotted border shows its area and a small card
beside it offers Rename in the Index, Change where it goes..., Redraw the area and Delete link, over the
page. Holding a row of the Links bar opens the same card for all of that row's links on the page. Making,
deleting, retargeting and redrawing a link are undone by Undo; renaming is not.

The links you made with 2.0.0-beta.5 (inked rings) keep working: as a notebook opens, each one's ring
gives it its area, and the ring stays on the page as ink. Deleting a link never deletes ink. Turning back
to 2.0.0-beta.5 (f9d13f5) keeps those links working there through their ink; links made from beta.6 on
have no ink, so they do nothing in beta.5 until you come back. The checks to run on the Kindle are in
docs/TESTING.md (Index phase 2).

## Touch gestures and the touch trace

Two fingers are gestures on the notebook page (docs/SHORTCUTS.md, gestures.ini by hand): down opens
Pages, up the Links bar, left the next page and right the previous one, and a page turned that way never
adds a page. They need no Enable shortcuts, and `gestures=0` in settings.ini turns them off (a pinch
and a pan still work). The pen wins: with it near the screen, or just lifted (under a second), a
gesture does not fire, and a swipe refused only for that says "Move the pen away, then swipe." now
and then. A palm resting beside the writing hand fires nothing.

`grimoire.log` has a line per touch session while Grimoire has the screen: the contacts, the gaps
between them landing and lifting, and what became of it (`touch: session 2 contacts, landing gaps 0 112
ms, lift gaps 0 40 ms: fired 2.down`, or `rejected pen`, `zoom`, `pan`, `palm`), a line with the
fingertip size seen for each single finger, and for the first 300 two-finger frames of a run the raw
contacts (slot, tracking id, tool, touch major, pressure, no coordinates). At most 2000 such lines a
run.

**The touch trace** records every contact frame so the thresholds can be tuned on real hands. It is
off unless you turn it on:

1. Create the empty file `grimoire/touch-trace` (beside `notebooks/`), for example
   `touch /mnt/us/grimoire/touch-trace` over SSH or an empty file made over USB.
2. Restart Grimoire (Library > Exit, then open it again).
3. Do the touch script below. Grimoire appends to `grimoire/touch-trace.txt`: a `# grimoire touch-trace 1`
   header line at each start, `F` lines (one per touch frame: kernel time, then `slot:id:x:y:major:tool`
   for each contact), `P` lines (the pen's state changes only: prox, down, up, leave; never its
   position) and `E` lines (what the recognizer decided). It stops at 4 MB, with one `E ... cap` line.
4. Delete `grimoire/touch-trace` so the next start does not record, and send `grimoire.log` and
   `grimoire/touch-trace.txt`. The trace is never in a notebook, an export or the share server, and
   nothing is written while the Kindle's own passcode keypad has the screen.

**The touch script** (about 15 minutes: a 3-minute two-finger part, then the rest). Work on a
notebook page:

- 20 two-finger swipes each way, 10 two-finger taps, and 5 three-finger taps (those should do nothing);
- writing with the palm resting on the screen (nothing should fire);
- 10 swipes straight after lifting the pen (write, lift, swipe right at once: note how often "Move the
  pen away, then swipe." shows);
- one finger drawing a Z, an N, an S, a circle, an alpha, a triangle and a V, 10 of each (nothing
  should happen in this build);
- 10 one-finger swipes each way;
- 5 minutes of writing and 2 minutes holding the Kindle and reading, thumb on the screen as you
  usually hold it.

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
  pausing it (it takes a few seconds longer to come back afterwards). Hosted by KOReader the file
  is ignored: KOReader pauses the Kindle UI itself.
- **Nothing happens when tapping Grimoire**: check `grimoire/grimoire.log`; the scriptlet logs whether it
  unpacked the app, and the app logs the device it found. `Grimoire.sh: not starting: KOReader is
  still running (reader.lua, pid N) ...` means a KOReader was left behind (after a crash, say):
  restart the Kindle.
- **KOReader says "Grimoire could not start. See grimoire/grimoire.log."**: the hand-over did not
  run Grimoire. grimoire.log has the launcher's reason, one of the `grimoire.sh: not starting...`
  lines (KOReader above) or `already running (launcher pid N), not starting another`; with no
  such line, the hand-over failed inside KOReader, and KOReader's `crash.log` has a `Grimoire:`
  line about it.
- **A `grimoire` folder moved to a Scribe of the other size** (1860x2480 and 1986x2648): the
  notebooks keep their page size (docs/FORMAT.md, Page size), and the calibration is scaled at
  startup, logged as `calibration: the pen calibration was made on a 1860x2480 panel; scaled to
  this 1986x2648 one` (with ` (not recorded: taken as an 1860x2480 Scribe's)` after `panel` for a
  calibration from before 1.1), the same for touch. A panel record gone stale shows as `... was
  made on a WxH panel, but its scale fits a WxH one: scaled from that to this WxH one`, or `...,
  but its scale fits this WxH one: made here, kept as it is`. A page file of another size than its
  notebook's (written by a version before 1.1 on the other Scribe) logs `store: <path> is WxH:
  scaled to WxH` once per run.
- **"This page was made by a newer Grimoire. It is read-only here."**: the page came from a newer
  Grimoire; update Grimoire to change it. grimoire.log says what happened: `store: <path> opens
  read-only and stays as it is on disk` and `store: read-only page: <reason>` when such a page is
  opened, and `store: <id>: recovered p<n>.qpg from p<n>.qpg.damaged-<t>` when a page an older
  version set aside comes back. These appear once per run, not on every open or Library listing:
  `store: <id>: p<n>.qpg.damaged-<t> opens now, but p<n>.qpg exists too; both kept`,
  `store: <id>: p<n>.qpg is not in the page list; its set-aside copies stay aside (...)` and
  `store: <id>: <k> unknown notebook.ini line(s) do not fit in the 2048 bytes kept, left out`.
  The toast "A damaged page was set aside; a blank one took its place (see grimoire.log)" now
  appears only for real damage. A set-aside page that is no longer in the page list can be put
  back by hand, with Grimoire closed: rename it to `p<n>.qpg` and add `<n>` to the `pages=` line
  of its `notebook.ini`.
- **"Your Kindle passcode was left off"**: the Kindle restarted while Grimoire had the passcode
  paused (see The Device Passcode); Grimoire says so at that start and the next two while it stays
  off. Turn it back on in Kindle Settings > Device Options > Device Passcode.
- **The screen**: the screen guard reads the framebuffer back at 32 bpp too (Scribe Colorsoft).
  It turns itself off after 10 repaints in a row and logs `screen guard: off after 10 repaints in
  a row (...)`. If the first frame and the next 4 refreshes all fail (retried once a second),
  Grimoire logs it and exits with status 1, so the launcher restores the Kindle UI. FBInk never
  reports 0 dpi: for a device it does not identify (named `Unidentified!` or `Unknown!`) it
  guesses 167 dpi. Grimoire then takes the dpi from the panel's physical size in vinfo (diagonal
  pixels over diagonal mm, or one side if only the width is known) when that comes to 100..600,
  else 300, and logs `display: FBInk does not know this device (...)`. On a color panel (FBInk's
  color panel flag at 32 bpp, the Scribe Colorsoft) the driver reads an update some time after it
  was sent, in tiles, so a change made before that would show half old or striped: Grimoire logs
  `display: colour panel: fast refreshes other than live ink wait until the driver has read the
  frame`, and after each such refresh it waits with `MXCFB_WAIT_FOR_MDP_COMPLETION`, logging the
  first 20 as `display: waited N ms for the colour panel's driver to read fast refresh M`. Live
  ink never waits (a pen stroke, erasing, the pen feel pad, the calibration's check line), so it
  writes no such lines. A wait the driver refuses, or one over a second, logs
  `display: MXCFB_WAIT_FOR_MDP_COMPLETION <error> after N ms (marker M): no longer waiting for the
  colour panel's read` and turns the wait off for the session.
- **The Share screen** never puts the Kindle's IP address in grimoire.log. It logs
  `share: QR code for the wlan0 address, port 8765`, or
  `share: no Wi-Fi address, so no QR code; showing the usb0 address, port 8765` (USBNetwork
  only), and `share: laid out again (turned or woken); the server carries on` after a turn or a
  wake. The server logs each request as `httpd: GET <path>`, and a folder zip as
  `httpd: GET /folder/<name>.zip`, without the folder's name. `share: layout: ...` appears only
  when something on the Share screen does not fit:
  `share: layout: a line is cut to fit (W px wide, room for R)` leaves the line's text out, and a
  part that leaves the card or runs into another is named with both rectangles. There should be
  none; if one appears, it is worth reporting with the orientation and style. Only the simulator
  also logs `share: (simulator) the code holds http://...`, which tests/share_check.sh compares
  with the decoded code.
- **Live view** logs `live: on` and `live: off`, `live: a browser paired` or
  `live: a pairing was refused (wrong code)` / `(locked)`, and `live: a viewer started (N
  watching)`. Its requests are logged without their query (`httpd: GET /live/pair`): no code or
  token ever reaches the log. On the Kindle each viewer is a process named `grimoire-live`; none
  should be left once Grimoire has closed. The pen's hover in the browser comes from the raw
  digitizer (the pen service relays no hover): after the first strokes with Live view on,
  `input: Live view's hovering pen comes from the raw digitizer, x as is, y as is (1.2 px from the
  pen on average over 32 contact frames)` says how its axes lie on the pen's, or
  `input: the raw digitizer's place is N px from the pen's at best (...): Live view shows no
  hovering pen` that they did not match, and the hover dot then only shows where the pen lifted.
- **Drawing while watching a computer**: pair Live view, then choose Monitor mode (M). It fits the
  page and hides the controls until the pointer or keyboard focus reaches them. The browser draws
  quick pressure-sensitive ink from pen samples, in the pen's own shade (the highlighter gray and
  see-through), then replaces it with Grimoire's exact image. Nibs and textured pens can change
  appearance at that handoff; white ink and precise erasing show the paper's color until the real
  paper returns. Whole-stroke erasing shows the pointer only. After a page turn the quick ink waits
  for the new page's image. The Latency button shows how late the pen and the page image show (p50,
  p90, maximum), each from when the Kindle read the pen, plus the clocks' uncertainty. It times the
  browser's drawing callback, not when a monitor physically lights a pixel. A 240 fps video showing
  the pen and monitor together is the final check. USB mode and Wi-Fi power saving are not changed
  by Live view. Live view is sometimes choppy, an open issue not explained yet: the Latency readout's
  Playout delay stays at its 20 ms cap and the dot skips, in any browser, through restarts. While it
  is, `ping -i 0.1` from the Kindle to the computer stalls about 250 ms in every 330 ms. On
  2026-10-28 a change of the radio's power saving (`iw dev wlan0 set power_save on` or `off`) ended
  it, in one direction in the afternoon and the other in the evening: neither setting is the cure.
  At actual size, full screen and Monitor mode scroll.
- **Switches** (empty files in the `grimoire` folder): `screen-flip` / `screen-noflip` force the
  180 degree framebuffer correction on/off, `pen-mode` turns on the MTK low-latency pen mode for
  live ink (off by default; small odd-rotation refreshes are padded as described in
  [INK-PERFORMANCE.md](INK-PERFORMANCE.md)). `ink-metrics` logs aggregate framebuffer copy and
  refresh-submission timings without pointer or key data. Both flags take effect at launch.
  The remaining switches include
  `stop-framework` stops the Kindle UI instead of pausing it, `tilt-flip` reverses the pen's
  tilt direction (only needed if the calligraphy nib turns the wrong way on your Kindle: it
  should give thick strokes down to the right when a right hand leans the pen that way),
  `watcher-multi` runs one continuous `lipc-wait-event -m -s 0` (KOReader's way) instead of the
  one-shot loop, for a device test (the log line `input: watching the power service (pid N): ...`
  says which mode is running), and `touch-trace` asks for the touch trace below.
- **The touch log and trace.** While Grimoire has the panel, `grimoire.log` gets
  `input: touch slots ...` at the start (how many contacts the panel follows and how it sizes
  them), the contacts of the first 300 frames of each multi-finger touch (`touch: k ...`: slot,
  tracking id, tool, size and pressure, never where), each multi-finger touch's landing and lift
  gaps and what it became (`touch: session ...: fired 2.down`, `rejected pen`, `zoom`, `pan`,
  `palm`), and a single finger's size at its lift (`touch: fingertips ...`), at most 2000 such lines
  per 60-second window. A capped window adds one `touch: log cap reached` notice; the first
  diagnostic after 60 seconds starts a fresh budget, so later sessions survive a long day.
  The 300-frame limit still applies to each contact session. With an empty `grimoire/touch-trace`, Grimoire also appends every touch frame with its
  places, the pen coming near, touching, lifting and leaving (never where it is: no handwriting),
  and each gesture's outcome to `grimoire/touch-trace.txt` (4 MB at most; each start, and each turn
  of the screen, adds a line naming the rotation, the screen and its dpi). It stays in the data folder: no export, share page or raw ZIP
  carries it. Delete `grimoire/touch-trace` to stop, and send the two files when asked.
- **The startup log** starts with a diagnostics block. `device:` lines give the model code (the
  first 6 characters of /proc/usid, never the serial), memory, CPU part and hardware. `display:`
  lines give the FBInk device id, platform, pixel format, color panel flag, raw vinfo and finfo,
  and the readback. `input:` lines give open errors, the power keys (a WARNING when there are
  several or none; the power key line ends with `, timed when read (EVIOCSCLOCKID failed)` if hold
  timing fell back to read times) and the orientation sensor check. A `settings:` line gives
  sleep_mode, sleep_minutes and side_button, and an `i18n:` line the language and why (The language
  above). The launcher's lines after `grimoire.sh: starting Grimoire` (or `starting Grimoire hosted
  by KOReader`) give `locale: LANG=...`, `device passcode: on` or `off` (then what it did with it),
  `power state: ...`, `blanket modules: ...` (marked `(Special Offers)` when the ad screensaver is
  loaded), `KOReader: installed, version <git-rev>` or `KOReader: not installed`, and
  `user id: N`; at exit, `power state at exit: X`. None of these identify the unit or a network,
  and the Share screen's lines above name the interface and the port, never the address, so the
  log can be attached to public issues.
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
- The panels: 1860x2480 on the 2022 and 2024 Scribes, 1986x2648 on the Scribe 3 and the Scribe
  Colorsoft, 300 dpi on all of them. On the Colorsoft's color panel the driver reads the
  framebuffer for an update some time after it was sent (about 23 ms, by a measurement on a
  Colorsoft that `src/display_fbink.c` cites, not one of this project's checks), in tiles; its
  FW 5.19.6 kernels have `MXCFB_WAIT_FOR_MDP_COMPLETION` (`_IOW('F', 0x4D, uint32_t)`, not in
  FBInk's `eink/mtk-kindle.h`), which KOReader waits with too.
- KOReader, as its Grimoire plugin relies on it: it keeps the pen, the touch screen and the screen
  to itself until its own `Device:exit()` (on the Kindle an own field of the device class:
  `frontend/device/kindle/device.lua` sets `KindleScribe.exit = KindleTouch.exit`).
  `koreader.sh` runs from a copy at `/var/tmp/koreader.sh` while KOReader is up, starts KOReader
  again in the same session whenever reader.lua exits with code 85 (its Restart), and keeps the
  Kindle UI paused all the while. It exports `LC_ALL=en_US.UTF-8` and its pause
  (`STOP_FRAMEWORK`, `PILLOW_HARD_DISABLED`, `AWESOME_STOPPED`) to what KOReader runs; started
  from KPVBooklet it pauses nothing. KOReader's exit wakes awesome on Special Offers Kindles, and
  the KOReader that KPM installs leaves the 5.17+ status bar running.

How Grimoire uses that:

- `src/display_fbink.c` writes the framebuffer directly (8 or 32 bpp, via `fbink_get_fb_pointer`)
  and refreshes with DU (live ink, in MTK pen mode), GL16 (UI, settling a finished stroke's grays),
  GC16 + flash (page turns, screen changes, every 30 UI refreshes). On a color panel it waits for
  the driver's read after every fast refresh but live ink's.
- KOReader's Grimoire plugin hands over in KOReader's exit path, after `Device:exit()`, and asks
  for the restart (code 85), so Grimoire gets the input and the screen and KOReader comes back
  without the Kindle UI ever waking (KOReader above).
- `src/input_evdev.c` finds devices by capability (not path), grabs pen + touch, watches the
  power key (its press, its release and a 4 s hold, timed by the kernel's event stamps), and maps
  axes through a calibration (default: straight onto the upright screen, swapped only if the axis
  ranges say so). The kernel keeps only 120-175 ms of the pen's writing for it (the raw
  digitizer's pressure and tilt about 100 ms, hover too), so nothing on its thread may block that
  long while the pen is near: long work reads the pens as it goes (the work pump), and its scan for
  keyboards and controllers never opens a node it holds (docs/INK-PERFORMANCE.md, Reader stalls).
- `kindle/Grimoire.sh` is the scriptlet (Library entry, cover embedded as base64 per the
  kindlemodding.org scriptlet docs, `# DontUseFBInk` so its output does not draw over the
  screen); it detaches `kindle/app/grimoire.sh`, which does the KOReader-style pause/restore above,
  plus `preventScreenSaver` (the Device Passcode stays on: The Device Passcode, above), and opens
  the share port (8765, or `http_port` from settings.ini) on wlan0 only. That is why the Share
  screen's QR code holds the wlan0 address, and USBNetwork addresses are listed under it but not
  encoded.
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
- With the Device Passcode on, the Kindle holds back `outOfScreenSaver` until the passcode is
  typed on its own keypad, on 5.19 and 5.20 too, and the native hand-over (The Device Passcode)
  relies on that. The hand-over passed upright and sideways on the Scribe 2024 with firmware
  5.20.1.0.2 ([TESTING.md](TESTING.md)). The power
  service's state names (`active`, `screenSaver`, `readyToSuspend`, `suspended`) are the same on
  both.
- Accented notebook and folder names become UTF-8 file names under `grimoire/PDFs`; whether the
  Scribe shows them correctly over MTP is not checked yet.
- The Scribe 3 and the Scribe Colorsoft have no public jailbreak: the checks cover their
  1986x2648 screen and the color panel's wait in the simulator only (`GRIMOIRE_SIM_SIZE=1986x2648`
  and `display.last_fast`, docs/SIM.md).
- The KOReader internals the plugin uses (`Device.exit`, `exitOrRestart`,
  `UIManager:restartKOReader()`, `start_with`, `DataStorage:getSettingsDir()`) are taken as
  KOReader has them; `tests/koplugin_test.lua` plays the plugin against stand-ins for them.

## Optional KOReader controls and wallpaper (1.1.0-beta.2)

Library > Settings > KOReader manages the plugin and the three scripts under
`app/koreader/patches`. Discovery checks the hosted `KOREADER_DIR`, then `/mnt/us/koreader`,
`/mnt/us/extensions/koreader` and `/mnt/us/documents/koreader`. Installing a plugin is explicit in
this sheet; patch toggles apply/remove only matching bundled files and preserve different files.
The Terminess toggle writes the embedded regular/bold fonts into KOReader's `fonts/Grimoire`.
Restart KOReader after any change. Font files can remain after disabling the patch; without it
KOReader uses its original UI font. Since 2.0.0-beta.1 the Terminess patch matches regular and bold
fonts and spaces the dropdown battery icon from its percentage. Exact old Grimoire Terminess bytes
are carried under `app/koreader/legacy` only for explicit migration; different user edits remain
protected.

Before each handover the plugin removes an old `grimoire/koreader-sleep.png`, asks KOReader's
screensaver module to select its configured cover/custom image, paints it offscreen and commits
a complete PNG with a temporary file and rename. It never shows the sleep UI. Errors cannot stop
the main handover. Display's fourth sleep choice loads this bounded image once per Grimoire run;
missing/unsupported image modes use the page instead. Random folders select once per handover.
The last snapshot can be used on a subsequent Library start. Native PIN always takes priority.

## Smart shapes

Hold the pen still at the end of a drawn shape to preview a straightened line, circle, ellipse or polygon.
Move the nib on to keep the freehand drawing instead. Lift to keep the preview as one stroke; Undo removes it.
The Shapes button below the eraser turns this off. The tool picker also offers Smart shapes and a Quick,
Normal or Slow hold time. Ordinary writing, white ink and erasing keep their existing behavior.

An empty `grimoire/shape-log` file enables recognition diagnostics at startup. The log records measurements,
recognition reasons, timing, and whether a preview was kept or cancelled, without stroke coordinates.

When rotating into landscape with **Sideways, show the whole page** enabled, the toolbar uses the blank
margin just outside the page, with a 24 px gap at 300 dpi. A toolbar on the left stays on the left;
a toolbar on the right stays on the right. Its saved portrait position is retained. When the setting
is off, rotation keeps the previous toolbar placement behavior.

The Links bar's Index button opens the Index map. Tap a dot to see its page card, and tap it again
to open the page. The List switch returns to named pages; Graph on that list reopens the map.
Drag to pan, pinch to zoom, and choose Around for nearby links. Rename on a page card edits its Index
name. A Links-bar incoming-pages chip opens Around for the current page.

Folders may contain subfolders. Open a folder and use Folder to create one inside it; Back goes up
one level. Renaming a folder also renames its descendants. Move to folder lists full destinations,
with More folders when needed. Folder ZIP exports include descendants and keep their hierarchy.
PDF mirrors retain their existing filename convention (a slash in a logical folder becomes an
underscore). Links between pages of one notebook can be made with New link and Choose from the pages.
The selection stroke disappears; a visible border is ordinary ink drawn separately.
