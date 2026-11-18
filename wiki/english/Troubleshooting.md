**English** | [日本語](Troubleshooting-ja) | [Polski](Troubleshooting-pl)

# Troubleshooting

## Known issues

- **KOReader's first screen can be incomplete on firmware 5.20.1.** KOReader can draw its first
  screen with parts missing, also when it comes back from Grimoire. This is a KOReader issue
  ([koreader issue 16283](https://github.com/koreader/koreader/issues/16283)), and Grimoire does
  not fix it.

## Quick fixes

- **Stuck?** Hold the power button 4 seconds. Grimoire saves your page and closes, and the Kindle
  comes back. If even that does nothing, hold it until the Kindle restarts (about 40 seconds); a
  restart keeps your notebooks. Never use the passcode reset code 111222777 to get out of
  anything: it erases the Kindle, notebooks included.
- **The Kindle's passcode is off after a restart**: the Kindle restarted while Grimoire was open
  (`grimoire.log` says so). Turn the passcode back on in the Kindle's Settings (Device Options >
  Device Passcode); Grimoire leaves that to you.
- **"This page was made by a newer version, and will be viewed as read-only."**: update Grimoire
  to change that page. Nothing on it is lost (see Moving between versions).
- **KOReader**: open Grimoire from KOReader with **Tools > Grimoire**, never with *Execute shell
  script* (see [KOReader](KOReader)). If "Grimoire did not start: KOReader is running." shows near
  the top of the screen after you tap Grimoire in the Library, a KOReader is still running in the
  background (after a crash, say): restart the Kindle.
- **KOReader says "Grimoire could not start. See grimoire/grimoire.log."**: the hand-over did not
  run Grimoire. A `grimoire.sh: not starting` line in `grimoire/grimoire.log` says why (KOReader
  had not let go of the pen and the touch screen, say), and `already running` means another
  Grimoire was still open. With neither, the hand-over failed inside KOReader: KOReader's own log,
  `koreader/crash.log`, has a line starting `Grimoire:` about it. "This Grimoire cannot open from
  KOReader" means the `Grimoire.sh` in `documents` is older than 1.1: copy a newer one.
- **Strokes land away from the pen tip**: **Settings > Calibrate pen and touch**. If you cannot
  reach it, tap **Grimoire - reset pen calibration** in the Library.
- **Everything is upside down** (possible on the 2022 Scribe): put an empty file named
  `screen-flip` or `screen-noflip` in the `grimoire` folder and relaunch.
- **Bits of the Kindle's own screen show through**: Grimoire repaints over them on its own. If it
  keeps happening, put an empty file named `stop-framework` in the `grimoire` folder; Grimoire then
  stops the Kindle interface completely while it runs (it takes a few seconds longer to come back
  after Exit). Opened from KOReader, the file does nothing: KOReader pauses the Kindle interface
  itself.
- **Nothing happens when you tap Grimoire**: look at `grimoire/grimoire.log`. Please attach it to
  any bug report.

## Moving between versions

Updating is one file, and so is going back. From 1.1.0-beta.1 on, Grimoire keeps what a newer
version writes and does not understand (ink data, newer papers, extra page data, unknown lines in
`settings.ini` and `notebook.ini`) and writes it back unchanged:

- A page it cannot fully show opens **read-only**, with "This page was made by a newer version,
  and will be viewed as read-only." Writing, erasing, clearing and changing its paper are refused
  on it; deleting it still works. Update Grimoire to edit it.
- A paper from a newer version shows as blank paper named **Newer paper** and stays the page's
  paper. New pages added after such a page get it too.
- A setting from a newer version that this one lacks works here as the nearest choice it has (a
  switch with a third state counts as on or off), and stays in the file until you change that
  setting.
- A notebook or folder name longer than 95 bytes (about 47 accented letters) is cut to fit, never
  in the middle of an accented letter.

1.0.0-beta and 1.0.0-rc.1 cannot do this. Open a notebook there after a newer version wrote to it,
and data can be lost: they set a page in a newer format aside as `p<id>.qpg.damaged-<time>` and
show it blank (just looking at it in Pages does this), and they drop newer data when they save.
1.1.0-beta.1 and later bring such pages back when you open the notebook again, and keep both
copies if the page was written again in the meantime; a page you deleted in the older version
stays set aside.

**Scribes of two sizes.** The 2022 and 2024 Scribes have a 1860x2480 screen, the Scribe 3 and the
Scribe Colorsoft a bigger one, 1986x2648. From 1.1 on, a notebook keeps the page size it was made
with: on a Scribe of the other size its pages are shown fitted to the screen, the writing on its
lines, and its new pages, PDFs and PNGs have the notebook's size. A notebook from before 1.1
takes the size most of its pages have. Your pen and touch calibration follows a `grimoire` folder
copied to a Scribe of the other size: Grimoire scales it to the new screen when it starts.
Versions before 1.1 stretch every page to their own screen instead (by the 7% between the sizes,
which moves the writing off its lines); back in 1.1, such a page returns to its notebook's size.
So going back to an older version on a Scribe of the other size moves the ink on every page it
edits there.

**Language.** 1.0.0-beta and 1.0.0-rc.1 forget the Settings > Language choice the next time they
save their settings, so back in 1.1 or later the language is Automatic again.

**KOReader.** An older version leaves KOReader's Grimoire plugin where it is: Tools > Grimoire
stays in KOReader, Notebook PDFs and Reset pen calibration still work, and Open Grimoire says
"This Grimoire cannot open from KOReader", because a `Grimoire.sh` older than 1.1 cannot be opened
from KOReader. Back in 1.1, Open Grimoire works again.

**Pictures as paper.** In 1.0.0-beta a page on a template, an imported page or an annotated page
shows blank paper, with its ink. Back in 1.1, those pages get their pictures back when the
notebook opens. Older versions also show erased ink again, as they do not know white ink or the
precise eraser, and they drop pinned pens from `settings.ini`.
