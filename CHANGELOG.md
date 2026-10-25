# Changelog

## 2.0.0-beta.1 (unreleased)

The quality release: what is new since 1.1.0-beta.2. The release candidates 1.1.0-rc.1 to rc.3
(below) are folded into it; there is no 1.1.0. docs/TESTING.md says what was checked on a Kindle.

- **Install with KPM**: the release has a `.kpkg` for Kindle Package Manager on `kindlehf`
  Scribes, built from the same app as the zip. It adds the two Library items, keeps your
  notebooks and settings through install, update and removal, and installs no KOReader plugin or
  patches. KPM numbers this release 2.0.1001. The preview feed starts with this release, and the
  stable feed with the first stable release, 2.0.0; docs/KPM.md says how to add a feed.
- **KOReader integration is your choice**: a new install, or a start from the Library, no longer
  adds Grimoire's plugin to KOReader. Add it with Library > Settings > KOReader > Install / update
  Grimoire plugin, or copy it in by hand. A plugin already installed keeps being updated; once you
  delete it, or install KOReader again, it stays out until you add it again. The plugin is
  refreshed when Grimoire is opened from the Kindle Library, so after updating, open Grimoire once
  from the Library before using Tools > Grimoire.
- **Opening Grimoire from KOReader works on firmware 5.20.1.0.2**: KOReader keeps a key-only input
  device open there, and Grimoire refused to start. A held device with no pen or touch axes no
  longer stops it; a held pen or touch screen, or a device whose kind cannot be read, still does.
- **Keyboard and controller shortcuts**: Library > Settings > Shortcuts > Enable shortcuts writes
  starter mappings to `grimoire/shortcuts.ini` (only when the file does not exist): tools,
  undo/redo, white ink and moving between pages. Edit the file, then Reload shortcuts.ini. The
  sheet shows the last key or button code, to help map a controller. Shortcuts act only in a
  notebook that is idle: not while drawing, with a sheet open, asleep or at the passcode keypad.
  Key and controller presses keep the Kindle awake, as the pen does. Grimoire adds no Bluetooth
  pairing and maps no analog sticks; 8BitDo controllers are not confirmed yet. docs/SHORTCUTS.md
  has the details.
- **Terminess menus in KOReader**: the optional patch picks a matching regular and bold font,
  uses a real bold, and leaves a gap between the dropdown menu's battery icon and its percentage.
  Library > Settings > KOReader updates an unchanged earlier Grimoire patch; a patch you changed
  is left alone.
- **Updates wait for Grimoire to close**: while Grimoire is open, unpacking a new `Grimoire.sh`
  and resetting the pen calibration are refused before they touch the app or your settings. A
  notebook PDF still being written in the background does not block a later start.
- **"Annotate this page in Grimoire" refuses a notebook's own PDF** from `grimoire/PDFs`, which
  would make a notebook of a notebook: open that notebook in Grimoire instead. A copy of the PDF
  kept somewhere else is annotated like any other book.
- **Gothic Library**: a small gap between the header ornament and the notebook cards, upright and
  sideways.
- **No calibration tip at startup**: Grimoire opens without that reminder.
- **No KOReader DPI patch**: the DPI experiment tried in the release candidates is gone, and
  KOReader keeps its own scaling.
- **Known issue**: on firmware 5.20.1, KOReader can draw an incomplete first screen, also when it
  comes back from Grimoire. This is a KOReader issue
  ([koreader issue 16283](https://github.com/koreader/koreader/issues/16283)), not fixed by
  Grimoire.
- **For developers**: `make dist` collects the installer zip, the optional patches zip, the KPM
  package and its feeds, checksums and the source revision in `dist/Grimoire-VERSION-COMMIT/`,
  and never overwrites a different build. The release workflow runs the full checks, needs
  ShellCheck and Luacheck, and publishes only from a published GitHub release.

## 1.1.0-rc.3 (candidate, folded into 2.0.0-beta.1)

- Fix KOReader handover on firmware 5.20.1.0.2: a retained key-only input device no longer
  prevents Grimoire from starting. Retained pen/touch devices and unknown input capabilities
  still block the handover; input device numbers are not hardcoded.
- Collect installable builds under `dist/` with version, source commit and checksums. Preserve
  existing different builds and keep Downloads out of the build delivery path.

## 1.1.0-rc.2 (candidate, folded into 2.0.0-beta.1)

- Remove the calibration startup tip. Grimoire opens without that reminder.
- Add a small gap between the Gothic header ornament and Library thumbnails, in both orientations.
- Refuse manual unpack/update and calibration resets while Grimoire is running, before touching
  its cached executable or settings. Background PDF exports do not block later starts.
- Keep KPM's temporary installer bookkeeping out of `grimoire/`, so a first launch can still
  migrate notebooks from the old Quire name. Require shell/Lua linters in the release workflow.
- Remove the experimental KOReader DPI patch and its Settings control. KOReader keeps its native
  scaling policy and the DPI setting already chosen in it. The Terminess font/battery-spacing fix
  remains.
- Retain the corrected KPM installer for the real Documents folder and the driver-owned Share
  test lifetime. KPM version 1.1.5002 sorts after 1.1.0-rc.1 (1.1.5001).

## 1.1.0-rc.1 (local candidate, folded into 2.0.0-beta.1)

- **KPM:** a kindlehf package built from the same installer as the ZIP, with Library launchers,
  install/upgrade/remove hooks and notebook/settings preservation. Fresh installs change no
  KOReader files. The release workflow validates artifacts before publication and maintains
  separate preview/stable feeds. KPM's numeric version 1.1.5001 represents this RC.
- **Explicit KOReader integration:** fresh Library starts no longer install the Grimoire plugin.
  Enable it in Settings > KOReader or copy it manually. An already installed plugin stays updated;
  deleting it or reinstalling KOReader requires opting in again. User patches remain separate.
- **Keyboard/controller shortcuts:** opt-in mappings for tools, undo/redo, white ink and existing
  pages, edited in `grimoire/shortcuts.ini` and reloaded in Settings > Shortcuts. The sheet shows
  the last button code. Drawing, open sheets and sleep/native PIN block shortcut actions. No
  Bluetooth pairing or analog-axis mapping is added; physical 8BitDo compatibility is unconfirmed.
- **Terminess menus:** deterministic regular/bold font-family selection, real bold registration
  and an explicit gap between dropdown battery symbols and percentages. A byte-exact prior
  Grimoire patch can be explicitly upgraded; modified patches stay untouched.
- **Optional Scribe DPI patch:** preserves the existing 320 layout and scales Auto/other presets/
  custom values proportionally. Loads before widgets cache sizes, leaves other devices alone,
  and retains KOReader's restart requirement. This changes KOReader's sizing policy only when
  the separate patch is enabled.
- **Installer follow-up:** stage Library wrappers inside Documents before the final rename.
  The Kindle blocks cross-directory moves into that folder; the real migration from the
  1.1.0-beta.2 launchers now passes. Regression coverage includes rollback after a failed second move.
  The share test keeps its server alive until downloads finish instead of racing a 25-second timer.
- **Validation:** real Scribe/KPM 0.2.2 isolated install, unpack, ARM version and uninstall passed;
  repository beta-app → RC upgrade and the local-file uninstall/reinstall fallback also passed.
  KPM 0.2.2 rejects a direct local-file upgrade; see docs/KPM.md and docs/TESTING.md. Original launchers and the
  running KOReader process stayed unchanged. Lua patch tests and the
  installed KOReader DPI menu code passed on the Kindle. The battery spacing and DPI behavior
  were confirmed by eye on that Kindle. Physical controller input and multi-device handover remain
  candidate checks.

## 1.1.0-beta.2 (2026-10-24)

- **KOReader compatibility:** discovery includes `extensions/koreader` and `documents/koreader`,
  handover accepts an intervening launcher process, and a KOReader launcher that leaves the Kindle
  UI running gets a Grimoire-owned pause/restore. Grimoire failures now reach the plugin instead of
  always reporting success. The plugin retains KOReader's framework service during Grimoire on
  no-framework starts, so it does not discard the service needed for power events.
- **Fast Pages:** a numbered grid loads no previews or other page files. **Go to page...** opens
  a number pad. A regression scenario uses 120 pages and checks that only explicitly opened pages
  enter memory. Selection and deletion still work.
- **Browser reader:** one page at a time; a sticky page counter and number field at the top jump
  directly to a page. Arrows, direct PNG links and Effects for that page remain available.
- **Native Kindle PIN:** Library starts keep the passcode flag enabled throughout. Sleep releases
  input and resumes the native keypad; only the unlock event returns input to Grimoire. Timers
  cannot bypass it. The framework stays running for the keypad, and PIN takes priority over sleep
  pictures. KOReader's own launcher still disables PIN for its session; hosted Grimoire inherits
  that policy. This new native handover needs device confirmation across firmware versions.
- **KOReader wallpaper:** Display's fourth sleep choice uses a custom image, random image or book
  cover exported by the plugin at handover. A missing image falls back to the page. The native PIN
  screen takes priority; KOReader's message/status widgets are not exported.
- **Manage KOReader in Grimoire:** Library > Settings > KOReader installs/updates/removes the
  plugin and toggles the three patches uploaded with beta.1. The three Lua scripts are reused
  unchanged; Terminess fonts, their OFL and manual instructions are included. Different patches
  at the same filename are preserved. Restart KOReader to apply changes. Both release zip assets
  include the optional scripts.
- **Carry-over fixes:** retain all book-title characters (show a box for glyphs the UI font lacks),
  refresh a card when its annotation picture arrives, put `source.ini` in raw exports, return to
  the tool button's own pen on its first tap, and wait for jobs rather than fixed delays in tests.
  Populated Templates and KOReader controls join the language/layout checks.

## 1.1.0-beta.1 (2026-10-20)

- **White ink and a precise eraser**: the tool picker (tap the pen button on the panel) has a new
  bottom row, **Black | White**. With White, every pen draws white with its own texture and
  settings, taking ink away while the paper's lines, staves, grid and dots stay; the pen in use
  shows a hollow icon on the panel (collapsed too) so it never looks broken on white paper.
  **Pen feel > the eraser's tab > Eraser: Whole strokes | Precise** picks the eraser that takes
  whole strokes, as before, or a precise one that takes away only what it touches. Library >
  Settings' pen-button row has a fourth choice: holding the pen button draws white. The
  whole-strokes eraser no longer takes a stroke when it touches only a part that white ink or the
  precise eraser already took away. When a notebook closes, ink erased entirely (every pixel of
  it) is taken out of its page files in the background, so zips and grimoire-pull copies no longer
  carry it; a stroke erased only in part stays whole in the file. Opening the notebook stops that
  at once.
- **Pinned pens**: pin up to four pens to the panel: tap the pen button, then **Pin** at the top
  right of the picker, tap pens to pin or unpin them, then Done. Each gets its own button under the
  pen button, and the panel still fits sideways at full size. Tapping the eraser again goes back
  to the pen used just before it, a pinned one too.
- **Templates**: copy PNG or JPG pictures into `grimoire/templates/` over USB while Grimoire is
  closed (upright, 3:4, 1860 x 2480 is best, light lines on white), then pick one with **More
  papers...**, the last tile of the editor's Paper sheet and of New notebook. Each notebook keeps
  its own copy, so changing or deleting a file there never changes pages already written. The page
  shows white paper for a few seconds while that copy is made.
- **A folder of page pictures becomes a notebook**: put a folder of PNG or JPG pages in
  `grimoire/import/<Name>/` (a PDF saved as pictures, for example with
  `pdftoppm -gray -png -r 200 file.pdf page`) and start Grimoire. It makes the notebook <Name> in
  the background, each picture a page's paper, says "Imported a notebook of N pages.", and moves
  the folder to `grimoire/import/done/`. PDFs are not read on the Kindle.
- **Annotate this page, from KOReader**: in a book, Tools > Grimoire > **Annotate this page in
  Grimoire** (also a gesture action) opens the page you are reading in Grimoire, ready to write
  on. Grimoire keeps one notebook per book, named after its title, with the pages you annotate in
  the book's order; annotating the same page again opens it with what you wrote. Library > Back
  to KOReader returns to the same book and page. A PDF's page is the whole page; an EPUB's is the
  screen as KOReader lays it out.
- **Sheets stay open when the Kindle turns or wakes** in a notebook too (the tool picker, Paper,
  Custom music, Display, Templates), as they do in the Library.
- **What you wait on comes first**: a PDF you asked for and a template's picture are made before
  background compaction and mirror PDFs.
- **Grimoire in Spanish**: every screen, the share pages and the Effects studio now come in
  Spanish too. A new **Settings > Language** row offers Automatic (named with the language it gives:
  "Automatic (English)"), English and Español. Automatic, the default, follows the language of the
  Kindle's own Settings, so a Kindle set to Spanish shows Grimoire in Spanish from the first start
  after the update; Ajustes > Idioma > English (Settings > Language, in Spanish) switches back. A
  choice applies at once and is kept in `settings.ini` (`language=`). The Spanish is a draft until
  a native speaker reviews it: `po/es-glossary.md` lists the words to confirm. In Spanish the
  keyboard's letters page has ñ after l, and a new notebook is named "Cuaderno 20 oct". Pages never
  change with the language: paper looks the same in every one (Cornell notes keep their English
  headings), and so do PDFs, PNGs and thumbnails. grimoire.log stays in English and says which
  language is in use and why (`i18n: language es (auto, from the Kindle's locale es_ES.UTF-8)`).
- **Text that fits**: a notice too long for one line takes two instead of being cut short, and a
  notice is never cut inside a character. The Library's top-bar buttons widen together for a longer
  word, the zoom chip widens for its text, and a paper's name too long for its tile, in New
  notebook or in the editor's paper sheet, goes on two lines. The Library card's line ("3 pages,
  Oct 10, 14:32") is no longer cut sideways in Classic: it takes a smaller size where the style has
  one, else leaves out the time. Custom music's part buttons draw their names at one size, and the
  Settings rows keep their text inside the sheet.
- **Share pages in Grimoire's language**: the share index, a notebook's page and the Effects
  studio are in the language Grimoire is in (their `<html lang>` says which), dates included ("10
  oct, 14:32"), and so are the messages a browser can show (a notebook that is gone, an export
  that failed). `/api/list.json` and the download file names do not change: a notebook's `paper`
  stays the English name. In the Gothic style, bold links keep accented letters legible. The
  studio's export buttons say "page" for a one-page notebook ("PNGs, all 1 page (zip)").
- **Open Grimoire from KOReader**: with KOReader installed, the next start of Grimoire from the
  Library adds **Tools > Grimoire** to KOReader: **Open Grimoire**, **Notebook PDFs** (KOReader's
  file browser in `grimoire/PDFs`) and **Reset pen calibration**, plus an Open Grimoire action for
  KOReader's gestures. Open Grimoire closes KOReader, runs Grimoire in the same session (the
  Kindle's home screen never shows), and when Grimoire ends KOReader starts again, with the book
  that was open. Opened so, the Library's Exit reads **Back to KOReader**, and holding the power
  button 4 s goes back too. The plugin speaks KOReader's language (English or Spanish). It is
  optional, and the zip's new "KOReader plugin (optional)" folder holds a copy for a hand install,
  never a top-level `koreader` folder. Each start from the Library keeps the installed plugin up to
  date, and `grimoire/.koplugin-installed` holds the build it came from; deleting
  `koreader/plugins/grimoire.koplugin` while KOReader stays installed keeps the plugin out (the file
  then reads `removed`). A start that finds no KOReader forgets that, so a KOReader installed later
  gets the plugin. *Execute shell script* on `Grimoire.sh` in KOReader still refuses with status 3.
  - Hosted by KOReader, Grimoire leaves the passcode and the pause of the Kindle's screens to
    KOReader, which keeps both for its whole session (Grimoire only stops again what KOReader's
    exit woke). The Kindle's own screensaver as the sleep screen needs those screens, so the page
    shows instead. Only a KOReader started by `koreader.sh` that paused the Kindle's screens (its
    Library item, KUAL) can open Grimoire. When the hand-over does not run Grimoire, KOReader says
    "Grimoire could not start. See grimoire/grimoire.log.": grimoire.log has the launcher's reason,
    and for a failure inside KOReader, KOReader's `crash.log` has a `Grimoire:` line.
  - For tools: `grimoire/app/grimoire.sh --hosted koreader` (0 Grimoire ran, 1 another Grimoire is
    running, 3 refused), and `Grimoire.sh --hosted koreader`, which unpacks first and then answers
    with that status, or exits 1 when it cannot unpack the app and 2 while the old Quire app still
    runs (docs/KINDLE.md). `hosted` is in the Capabilities lines: `guard unpack-only hosted` in
    `Grimoire.sh` and `app/CAPABILITIES`, `guard hosted` in `app/grimoire.sh`.
- **Notebooks keep their page size**: the 2022 and 2024 Scribes have a 1860x2480 screen, the
  Scribe 3 and the Scribe Colorsoft a 1986x2648 one, and a notebook now keeps the page size it
  was made with (`page_w`, `page_h` in `notebook.ini`). On a Scribe of the other size its pages
  are shown fitted to the screen, the writing on its lines; its new pages, thumbnails and every
  export (PDFs, PNGs and the share pages' pictures, a missing page's blank one included) have the
  notebook's size. Pages an earlier version stretched to another Scribe's screen come back to the
  notebook's size. A `notebook.ini` without a size takes the size most of its page files have, a
  tie going to the first page's. Custom music's "Systems per page" counts the page's own height
  (a 1986x2648 page fits more).
- **Calibration follows the panel**: `settings.ini` records the panel each calibration was made on
  (`pen.panel`, `touch.panel`), and a `grimoire` folder copied to a Scribe of the other size has
  its pen and touch calibration scaled to the new panel at startup. A record gone stale
  (1.0.0-beta.2 keeps the line through a calibration of its own) is caught by the calibration's
  own scale, on either size.
- **Scribe Colorsoft**: on its color screen, a quick refresh other than the pen's own ink now waits
  until the driver has read the frame (`MXCFB_WAIT_FOR_MDP_COMPLETION`), which should stop buttons
  showing half old or striped while the panel is dragged or the page pinched. Writing, erasing,
  the pen feel pad and the calibration's check line never wait. A driver that refuses the wait, or
  takes over a second, is not asked again that session.
- **For developers**: `make check` runs `make l10n-check`: the committed `po/*.mo` must be what
  msgfmt makes of `po/*.po`, and `tests/scenarios/y_all_screens.qs` visits every screen and sheet
  in every language and style with the fit check (`GRIMOIRE_CHECK_FIT=1`), which fails on any text
  cut short (`fit[es]: ...`); its shots in `build/shots/l10n/<lang>-<style>/` show translators every
  screen. `make l10n` (GNU gettext) collects the strings into `po/grimoire.pot` and updates and
  compiles each language. `docs/TRANSLATING.md` says how to improve a translation or add one.
  `GRIMOIRE_LANG` picks the simulator's language, `GRIMOIRE_L10N_DIR` reads a catalog from a
  folder, and `GRIMOIRE_HOST=koreader` starts it as KOReader's plugin does; the scenarios run in
  English. New checks cover the language, the hosted start, page sizes and live ink (`language`,
  `host`, `asleep.mode`, `editor.page_size`, `editor.scale`, `display.last_fast` and more, in
  docs/SIM.md).
  `tests/share_check.sh` also checks the pages in Spanish and from a catalog full of markup, and
  the studio's Copy command (with node). `tests/launcher_check.sh` runs hosted starts and their
  refusals and installs the plugin; `tests/koplugin_check.sh` runs the plugin against stand-ins for
  KOReader (with luajit). The test builds stop at the first undefined behaviour.

## 1.0.0-beta.2 (not released on its own: all of it is in 1.1.0-beta.1)

- **Hold the power button to leave**: hold it 4 seconds to save and leave Grimoire from any
  screen, asleep or awake, and get the Kindle back. On a Kindle that stays locked after a press, a
  second press does the same. When the page cannot be saved (the Kindle's storage is full), the
  hold never says "Saved.": awake, the first hold stays in Grimoire with "Could not save this page.
  Hold power 4 s again to leave anyway.", and a second within a minute leaves with "Leaving
  Grimoire without this page." From the sleep screen, and on a Kindle that stays locked, Grimoire
  leaves at once, and grimoire.log says the page could not be saved. Leaving straight from a
  notebook this way while another notebook's PDF is still being written no longer skips the open
  notebook's PDF.
- **Kindles with a passcode**: a Device Passcode is now paused while Grimoire is open and turned
  back on when you leave, as KOReader does. With a passcode, the Kindle holds back its wake until
  the passcode is typed, but its keypad cannot take a touch while Grimoire holds the touch screen:
  Grimoire's page came back over a Kindle that stayed locked, and leaving painted the old screen
  over the passcode keypad (a tester on 5.20.1 could only get out by restarting). Grimoire says so
  the first time ("Your Kindle passcode is paused while Grimoire is open."). If the Kindle restarts
  while Grimoire is open, the passcode stays off: Grimoire then shows "Your Kindle passcode was
  left off. Turn it back on in Kindle Settings." (in place of its other start notices) at that
  start and the next two while the passcode is still off. When the launcher cannot note the pause
  (the Kindle's storage is full or read-only), it does not pause the passcode.
- **Sleep and wake fixes**: the Kindle no longer falls asleep again right after waking. Three pen
  side-button clicks while asleep no longer clear the page, and a pinch while asleep no longer
  zooms it. A press right after another no longer requests sleep halfway through a long press.
  Grimoire no longer draws over a Kindle that is still locked or asleep after a press, and no
  longer sends a power-button request to a Kindle already in its screensaver. The sleep request is
  ordered (preventScreenSaver 0, then powerButton 1), and "Sleep after: never" is restored after
  each wake. Leaving Grimoire while the Kindle shows its screensaver or passcode prompt no longer
  paints the old screen over it.
- **Notebooks move between versions**: Grimoire now keeps what a newer version writes and does not
  understand (ink data, papers, extra page data, unknown `settings.ini` and `notebook.ini` lines)
  and writes it back unchanged. A page from a newer version opens read-only ("This page was made
  by a newer Grimoire. It is read-only here.") instead of being set aside as damaged, and pages
  1.0.0-beta set aside come back when their notebook is opened, if they are still in its page
  list. Papers from a newer version show blank as "Newer paper" and are kept; a page added after a
  page that holds one imported page's picture (1.1's document pages) gets the notebook's own paper,
  as 1.1 does, never that picture. A setting value from
  a newer version that this one lacks (for example `side_button=3`) works here as the nearest value
  it has. It stays in the file until a save finds the setting changed; from then on the setting is
  saved as you set it, even if you set it back. A switch holding another value than 0 or 1 (a
  newer version's third state, such as `invert=2`) is used as on or off and stays in
  `settings.ini` as it was until the setting is changed. A notebook or folder name longer than 95
  bytes is cut before the character that would cross the limit, never inside an accented letter.
  `docs/FORMAT.md` documents the file format, and the README's "Moving between versions" says what
  older versions do with newer notebooks.
- **A QR code on the Share screen**: point a phone's camera at it to open the share page without
  typing the address. It holds the Kindle's Wi-Fi address; other addresses (USBNetwork's, say) are
  listed under it. Share also has a landscape layout, so the downloads counter no longer covers a
  help line or the third address and text no longer runs past the card. Turning the Kindle or
  waking it on the Share screen no longer restarts the share server, so queued downloads and pages
  being sent are no longer dropped.
- **Share fixes**: asleep on the Share screen, a phone or computer trying to connect no longer
  keeps Grimoire's processor busy until the wake; the connection waits and is answered after the
  wake. With no page to show, the Share screen's sleep screen no longer shows the QR code and the
  address: it shows the "Share over Wi-Fi" card with "Asleep" and "Wake the Kindle to share
  again." The share page's link to a folder's zip works for names up to the 95-byte limit (a long
  accented name got "not found").
- **Accents on the keyboard**: a new **áé** key beside the space bar opens a page with á é í ó ú,
  à è ì ò ù, â ê î ô û, ä ë ï ö ü, ñ ç ã õ ß œ æ ø å and ¿ ¡ « », and the middle dot of Catalan
  l·l. Shift gives the capitals and **abc** goes back; backspace removes a whole character.
  Renaming keeps accents: the keyboard used to drop every non-ASCII byte from the name it opened,
  so "Música" came back as "Msica". Folders whose names begin with an accented letter now sort
  with that letter (Álgebra among the A's), not after Z.
- **Gothic bold keeps accents legible**: bold labels, toasts, menus, Settings rows and keyboard keys
  keep the two dots of ü ö ä ë, the tilde of ñ ã õ, the hat of â ê ô û and the ring of å, and the
  carons, double acutes and ring of Czech, Slovak and Hungarian (č š ž ř ň ě ď ť ő ű ů) keep their
  gap. The bold overprint used to fill them in (ü looked like ū). In bold, ß keeps the notch that
  tells it from B, and ø and Ø keep their counters open around the slash. In every Gothic weight
  the inverted exclamation mark now hangs below the line, as type usually sets it; it used to look
  exactly like an i. ASCII text and the Classic style do not change.
- **KOReader**: Grimoire no longer starts from inside KOReader, where it got neither pen nor touch
  and its exit woke the Kindle UI underneath KOReader; KOReader shows status 3 instead. A Library
  tap with a KOReader left running says so on the screen. For the KOReader plugin in 1.1:
  `Grimoire.sh --unpack-only` with documented exit statuses, and Capabilities lines in
  `Grimoire.sh` and `grimoire/app/grimoire.sh` (also `grimoire/app/CAPABILITIES`).
- **Smaller fixes**: a tap on Undo or Clear right after a stroke now works at once (it was ignored
  for about half a second). Turning the Kindle, or waking it, no longer closes an open Library
  sheet (Settings, New notebook, the keyboard...). In the Library, a notice that is up when the
  keyboard or a tall sheet opens (New notebook; Settings and Custom music sideways) moves above
  its keys or buttons instead of covering them. Sideways, the New notebook sheet shows "Narrow, no
  margin" and its neighbours on two lines instead of cutting them short. The paper pickers and
  Library cards no longer show a paper they do not know as the 7-string bass tab. A page that
  cannot be saved (the storage is full) is tried again every 2 s instead of at every turn of the
  main loop. The notice for a damaged page set aside now fits the screen in the Gothic style.
  `settings.ini` is no longer written through a fixed 8 KB buffer that was never checked.
- **Effects preview**: the transparency checkerboard covers only the page, which now has a thin
  outline on a gray surround, so you can see where the page ends.
- **Other Kindles**: the screen guard works on 32 bpp panels (Scribe Colorsoft); Grimoire closes
  cleanly, giving the Kindle its screen back, if the panel takes no refresh at all; the dpi of a
  device FBInk does not know comes from the panel's physical size.
- **A more useful grimoire.log**: it starts with diagnostics for bug reports (the model code, never
  the serial number; memory, display and input details; the sleep settings; the Kindle's language,
  whether a passcode is set, the power state, Special Offers and KOReader's version) and ends with
  the power state at exit. None of these lines identify the Kindle or its network. The Share
  screen logs which interface its address is on and the port, never the address itself, and a
  folder's zip download is logged without the folder's name. A `watcher-multi` switch (an empty
  file in the `grimoire` folder) listens to the power service the way KOReader does, for a device
  test.
- **Licenses**: the app ships the licenses of FBInk, stb, musl, zig's compiler-rt and the QR code
  generator, and the notices of cutef8 and Bjoern Hoehrmann's UTF-8 decoder (both compiled in with
  FBInk), with the GPL and the font licenses, in `grimoire/app/licenses`.
- **For developers**: `make check` builds and inspects the zip and runs both launchers against
  stand-ins for the Kindle's tools. The scripted scenarios check what the app holds after each
  step (`expect` lines) and fail the build when a check fails or a script line is misspelt. After
  the Kindle turns, the simulator maps input through the calibration the way the Kindle does, so
  script taps land where they say on calibrated data too. `tests/test_golden.c` holds renders
  recorded from 1.0.0-beta that every later version must reproduce, and `tests/test_format.c`
  checks pages against a frozen copy of 1.0.0-beta's page reader. The runner's `power held` and
  `storage full`, its `watching`, `toast.rect` and `editor.unsaved` checks and a scenario's
  `# pdfs: N` line cover the hold, a full storage, Share asleep, notices over sheets and the PDFs
  written at exit (docs/SIM.md).

## 1.0.0-beta (2026-10-18)

- **A logo, and a bat on the cover**: "Grimoire" in the pixel blackletter with a pixel bat over
  it; the same bat now sits in the window of the Library cover, in place of the diamond.
- **Effects in Share over Wi-Fi**: a studio in the browser makes new pictures of any page:
  transparent, white, colored or gradient backgrounds; paper lines shown, hidden or recolored;
  ink in a color or a gradient across the writing (a Signature preset: transparent, gradient,
  trimmed); tones (gray, duotone, invert), warmth, tint, brightness and contrast; a crop to the
  ink; presets from Sepia to Blueprint. It downloads a PNG, a zip of PNGs or a PDF, and shows the
  ImageMagick command that makes the same picture.
- **Share over Wi-Fi fixes**: a browser asking for several things at once (the page previews of
  a notebook) could get the wrong file or a dropped connection, because the server answered a
  new connection from the previous request before the new one had arrived. Each connection now
  has its own buffer, downloads beyond the two the Kindle renders at once wait their turn
  instead of being refused, an idle connection no longer holds up the others, and notebook page
  previews load at a third of the size.
- **Share pages in your style**: the browser pages use Grimoire's own fonts, Jacquard 24 titles
  and Jersey 10 text in the Gothic style (kept pixel-sharp), Terminess in Classic.
- **Lighter notebook names**: in the Gothic style, notebook and folder names on the Library
  cards use the pixel font's regular weight instead of its bold, which turned a name in
  capitals into a solid black bar.

## 1.0.0-rc.1 (2026-10-17)

First public pre-release, and a new name: **Quire is now Grimoire**.

- **Gothic style**: crisp pixel type like an old console game (Jacquard 24 pixel blackletter
  titles, Jersey 10 text, drawn as exact pixel blocks), carved-stone frames, an arcade of
  pointed arches, lozenge checkboxes and sliders, a new Library cover. Settings > Style switches
  to the plain Classic look.
- **Two new pens**: **charcoal** (coarse grain, crumbly edges, loose dust, broad and soft on
  its side) and a **calligraphy pen**, a broad-edge nib held at 90 degrees to the pen's real
  lean, so a right hand leaning to the bottom right writes classic 45 degree italic and a left
  hand gets the mirror image. The tool picker is now a 3 x 3 grid of nine pens.
- **Moving from Quire**: the first launch moves `quire/` to `grimoire/` and removes the old
  launcher; nothing to do by hand.
- **Safer storage**: a page that cannot be read is set aside (`*.damaged-<time>`) instead of
  being overwritten by the blank shown in its place; a damaged notebook index rebuilds its page
  list from the page files; a save that hits a full disk fails loudly and keeps the page in
  memory instead of dropping it; notebooks created in the same second can no longer collide.
- **Share over Wi-Fi**: PDF, zip and PNG downloads render in a separate process, so a big
  download no longer freezes the Kindle or trips the watchdog; requests for foreign hostnames
  are refused (DNS rebinding); the firewall follows a custom port.
- **Sleep and launch fixes**: waking from sleep no longer sends you to the Kindle home screen;
  a leftover background export can no longer stop Grimoire from launching; a relaunch waits for
  the previous session to finish handing the screen back; the power-service watcher exits with
  Grimoire even after a crash.
- **Other Kindles**: a clear notice when no pen is found; the warmth slider is hidden on Kindles
  without warm light.

Earlier (as Quire, private): notebooks and folders, 18 papers plus a custom music builder, seven
tunable pens, the floating panel, pages view, pinch zoom, rotation, sleep screens, Wi-Fi / USB /
SSH export.
