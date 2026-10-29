# Changelog

## 2.0.0-beta.5 (2026-10-28)

The first release written in Rust. It is what is new since 2.0.0-beta.3: 2.0.0-beta.4 was a
performance candidate that was never published, so its changes (the last section below) are in this
release too.

- **Buttons sheet:** "Swiping past the last page adds a page" and the status display (bar, corner
  or off) can be given a button, in the Pages and Menus groups.
- **The touch log keeps going on a long day:** its 2000-line limit now applies per minute, not per
  run, so touches late in the day still leave their lines in `grimoire.log`.
- **Notebook status bar:** the new default keeps the notebook, page and section, link counts, tool,
  clock, battery, screen sharing and controller visible above the page. Upright and sideways, the
  page fits below it. Display lets you select Bar, Corner or Off, also available as `status-mode`.

- **Grimoire is now written in Rust**: the whole app has been rewritten, screen for screen and file
  for file, from the C it was written in until now. Using it does not change: the same screens,
  pens, papers, shortcuts and settings. The port itself changes no file: your notebooks, templates,
  settings and everything else under `grimoire/` are read and written as before, so there is nothing
  to convert (the new features below add a few files of their own, each described in its entry).
  The point is the versions to come: Rust makes a whole family of crashes (reading or writing past
  the end of a buffer, using memory that was already freed) much harder to write, in everything
  except the small C parts the app still links and that are unchanged: the screen library FBInk,
  the font and image code and the QR code. The maintainer compared it with the C version on a
  Kindle Scribe (2024) before this release. If something looks or behaves differently from
  2.0.0-beta.3 and is not in this list, it is a mistake in the port: please report it.

- **The Index from a button or a key**: five new actions for Buttons... (a new Index group, and
  Whole page sideways in Pages) and for shortcuts.ini: Links on this page opens and closes the Links
  bar; Go to page (Find: a keyboard over the page that takes a page number or part of a page's name,
  accents and capitals ignored, so "arb" finds "Árbol") opens and closes the same keyboard as the
  bar's Go to; Back to the page before returns to the page a link, the Pages view or Find left (up
  to 32, newest first, forgotten when the notebook closes; a toast says when there is none); Show
  link areas is the bar's eye, which draws the dotted, numbered boxes where each link's ink is, on
  and off, remembered; and Whole page sideways is the Display row of the same name.
  Removing or changing a link now redraws the boxes at once, so a deleted link's box no longer
  stays on the page.

- **Draw while looking at your computer**: Live view now sends pen movement on its own connection,
  so the browser can show a quick pressure-sensitive line while the exact page image catches up.
  Menu taps leave no preview ink. Monitor mode (M) fits the page and tucks the controls away; its
  choice is remembered in that browser. An optional Latency readout shows estimated pen and image
  delays. Textured nibs and erasing still settle to Grimoire's exact rendering.

- **Live view, smoother and easier to read**: the pen's line and its dot in the browser no longer
  skip. The browser used to get the pen's movement only when Grimoire's main loop read it, so a heavy
  tool such as a textured brush sent it in bursts; now each browser reads the Kindle's pen sensor
  itself the moment it has a point, and plays the points back at their own times (the Latency
  readout has a Playout delay). The dot has five colours and a colour picker, and three sizes
  (small is the new default, smaller than before; medium is the old size), remembered in that
  browser. The toolbar and the clock and battery corner are layers over the page, each with a
  switch (on by default, remembered). Picture in picture draws at its own window's pixels, so ruled
  lines and thin strokes keep their weight there too. Recording makes an MP4 in Firefox now, where
  its encoder can, and the button says Record (MP4) or Record (WebM) so you know which you get.
  Live view's words are in English and Spanish. A menu or a keyboard on the Kindle takes the
  panel out of the browser's copy while it is open, and the page under the panel arrives in the
  same push as the rest of the screen. At actual size, full screen and Monitor mode scroll to the
  page's far edges. After the pen had been away for over 35 minutes, the browser's dot froze and the
  quick line stopped until 71 minutes had passed: a long silence now starts them again.

- **A Kindle reboot while writing, fixed**: with the developer switch `ink-metrics` on, Grimoire
  followed some ink refreshes through the Kindle's display driver, and a race in the driver could
  then crash the Kindle, which rebooted itself. Grimoire no longer asks the driver for those
  timings. Going to sleep and closing Grimoire could also wait 5 seconds longer than needed for
  the last refresh; they no longer do.
- **No more skips while writing with shortcuts or a controller on**: with Settings > Controller's
  shortcuts on, every 5 seconds of unbroken writing a line could skip: it jumped ahead, then drew
  a short piece backwards, and a stretch of it was lost. Grimoire's check for keyboards and
  controllers that connect held everything up for about a third of a second, long enough for the
  Kindle to throw pen movement away. The check now waits until the pen has left the screen, and
  no longer opens the pen, touch and other devices Grimoire already uses, so it takes next to no
  time. If the Kindle ever does drop pen movement, the line now goes on straight from where it was,
  without the backwards spur; a stroke that ended in the dropped moment (the pen lifted, or turned
  to its eraser end) is not joined to the next by a stray line; and saved pages keep each point's
  real time. grimoire.log says how long any hold-up took.
- **The pen's pressure and tilt kept through slow moments**: turning to a page with a lot of
  writing, opening Paper for the first time in a notebook, going back to the Library, opening the
  Display sheet and dragging its light sliders held Grimoire up for a tenth of a second to more than
  a second, long enough for the Kindle to throw away the pen's pressure and tilt while it hovered
  (26 times in one session), and a stroke begun in such a moment could lose its start. Grimoire now
  keeps reading the pen while it draws a page, a paper, a thumbnail or the screen, and while it
  saves. The front light's levels are read once when Grimoire starts instead of at every Display
  sheet, and a light slider redraws only its own row while it moves. Each point of a stroke also
  takes the pressure and tilt of its own moment, where a stretch read at once used to share one.
  grimoire.log names what held it up.
- **The Index's keyboard types as fast as the Library's**: naming a page (and Go to page's number
  pad) lagged further behind with every key. Each key refreshed most of the keyboard at once, so
  the next key waited on the screen for it, and the pen resting on a key drew the whole keyboard
  again many times over. Now only the text and the key refresh, the keyboard is drawn once a key,
  and a moment after typing stops the keys are cleaned up, exactly as on the Library's keyboard.
  Naming a page right after writing on it no longer leaves the toolbar drawn over the keyboard,
  and a notice stays above the keys (Go to page's "Enter a page number" covered its 0 and del).

- **A small clock and battery on the notebook page**: tap a corner of the page with a finger (the top
  left, or the corner you pick in Settings > Display) and a small box shows the time, a battery drawn
  in pixel art that shows the charge, with its percent beside it, and a bolt over the battery while it
  charges; tap it again to hide it. It shows on the notebook page and nowhere else: not on the
  Library, Settings or any other screen, and not while a sheet, a menu, the Links bar, Find or the
  Pages view is in front of the page. Settings > Display, beside Done, has its size (small, medium
  or large; the battery grows with it) and its corner (top left, top right, bottom right, bottom
  left): the tap zone follows it, a 14 mm square and the box's own place with 4 mm around it, shown or
  not (half the taps to bring it back went where it had been, outside the square). One tap is one toggle, at once: a fingertip that
  rolls inside the corner still taps, slow presses too (up to 0.8 second), nothing makes the next tap wait, and the box is refreshed as the
  finger lifts, even when a pen left the screen without a word. A key or a controller button can turn
  it on and off or move it to the next corner (Buttons... > Menus > Clock and battery, and Clock and
  battery corner). It changes each minute and when the battery or its charging changes, with one
  small refresh of its own, and holds back while the pen is near or you are writing, so it never
  touches a stroke. It is only ever on the screen: never in a page, a PDF or a picture, Live view
  or the sleep screen. Older versions leave the three new lines in settings.ini alone, and count
  `status-toggle` and `status-corner` among the shortcuts they do not know.

- **Live view's icon beside the panel**: while a computer watches with Live view, the small icon that
  says so is no longer in the top right corner of a notebook page, where it covered the page: it sits
  beside the panel, level with its header, and moves with it when you drag or fold the panel. On the
  other screens it stays in the top right corner.

- **Menus over the panel**: the panel is only ever over the notebook page. The Links bar, Find and its
  keyboard, Display, Paper, the tool picker and the Pages view are all drawn over it, however the page
  or the panel is redrawn, so a button of the panel can no longer show through a menu or a keyboard.
  In Live view the browser's copy of the panel goes while one of them is open.

- **Two-finger gestures and the Links bar**: on a notebook page, two fingers down open Pages and two
  fingers up open the Links bar, left turns to the next page and right to the previous one (a page
  turned left on the last one adds a page, as the next-page key does; a row in Display, under
  Sideways, turns that off). The bar is a card along the bottom with one row for each page
  the open page's links go to, numbered as the page's dotted link areas are (Areas in the bar, or
  Buttons... > Index, show them over the page, never in a PDF): tap a row to look at that page, tap
  the page to stay on it, or X to go back where you were. Go to opens a keyboard that takes a page
  number or part of a page's name from the Index, and Back to the page before returns. The panel has
  a Links button after Next. The pen always wins: with it near the screen or just lifted, a gesture
  does not fire, and a resting palm fires nothing. Sideways now shows the whole page (Display >
  "Sideways, show the whole page" turns it off), so two-finger swipes work there too. A new
  gestures.ini beside shortcuts.ini changes what each gesture does (docs/SHORTCUTS.md). After the
  update, the first notebook you open and the first turn sideways say what changed, once. Older
  versions keep the four new settings.ini lines and never read gestures.ini.

- **Updates from Grimoire**: Library > Settings > Updates checks Grimoire's feed through KPM (betas
  and releases on a beta, releases only on a release; a tap changes it) and installs a newer one:
  Grimoire closes, KPM installs it with its progress on the screen, and Grimoire starts again. A
  failed install is reported at the next start. Hotfixes go out the same way: docs/KPM.md,
  Hotfixes. Settings' bottom row now has Controller and Updates side by side.

- **Controllers**: Settings > Controller (was Shortcuts) shows the controller or keyboard connected,
  and Buttons... learns which controller button does what: tap a row, press the button
  (grimoire/controller.ini). A short toast says when one connects or goes, and when another app
  holds it so Grimoire gets nothing. The D-pad (or a hat, as an arcade stick's lever) turns pages
  and tools by default, also with a shortcuts.ini from an earlier version, which binds no D-pad (the
  file is not changed). Buttons... names a Switch-mode controller's buttons as printed on it (B, A,
  Y, X, L, R, ZL, ZR, -, +) instead of by number or the Linux names. Next page on the last page adds
  a page, as the editor's + button does (it used to stop there).
- **More for a controller's buttons**: every tool on its own, Erase while held (the pen erases
  while the button is down), the eraser's mode, clear page, first, last and new page, fit page,
  and the editor's menus: the tool picker, Paper, Pages, Index, Display, Pen feel, fold the panel,
  the Library and Close menu. A menu's button opens it and closes it again, or swaps the menu in
  front for its own. Buttons... groups them under Editing, Tools, Pages and Menus, and a toast says
  when a button learned there leaves another action. Older versions refuse the new names in
  shortcuts.ini and count them; their other lines still work.
- **Next and previous tool stay on the panel**: they now go through the pens on the panel only, the
  tool button's pen and the pinned ones, top to bottom and round again, so pressing next over and
  over never sends the highlight into the tool picker and back. With nothing pinned they go through
  the tool picker's nine pens in its order; with only the tool button's own pen pinned, the first
  press brings the picker's next pen onto the tool button, and from then on they go between those
  two. From the eraser they go on from the pen it goes back to, instead of starting again at the top
  or the bottom. A new shortcuts.ini numbers keys 1 to 9 in the picker's order too (3 charcoal, 4
  ballpoint, 7 fountain pen); an existing one is never changed.
- **Next and previous in the tool picker**: two new actions for a button or a key (picker-previous
  and picker-next, a row of their own in Buttons... under Tools) move the open tool picker's choice
  and leave it open; with it closed they switch tools through all nine pens. Next and previous tool
  leave the open picker alone.
- **Links and page names from a button**: two new actions for a button or a key (add-link and
  name-page, rows of their own in Buttons... under Pages). Add a link starts a link on the open page
  in one press, without going through the Pages view and its Index: draw around what links, then
  tap where it goes. Pressed again before you draw, the link is not made. Name the page opens the
  keyboard for the open page's name and goes back to the page after Done or Cancel.
- **Bluetooth controllers**: with the kindle-hid-passthrough helper unpacked in grimoire/bluetooth
  (only its files: its installer adds a boot-time button mapper that takes controllers away),
  Bluetooth controller starts it with Grimoire and Pair a controller... finds and pairs one. It runs
  only while Grimoire does: it stops when Grimoire leaves, and the Kindle's own Bluetooth comes back.
- **LT and RT on Xbox-layout controllers**: on 8BitDo's Xbox mode (and other controllers whose
  triggers are analog) LT and RT did nothing, because Linux reads them as axes, not buttons.
  Grimoire now reads each as a button: pressed past half its travel, let go near its rest, once each
  way however the hand wavers. Buttons... learns them and names them LT and RT. A right stick is
  never taken for one.
- **Hold a row in Buttons... to clear it**: holding a row for a moment takes its buttons away, that
  row's alone; it reads none and a toast says so. Reset buttons still clears every row.

- **Highlights keep the tip they were drawn with**
  ([#5](https://github.com/ganten1998/grimoire-scribe/issues/5)): each stroke now records its
  tip, so highlights drawn with the round tip before 1.2.0 look as they did again, in the editor
  and in exports, and new highlights keep the vertical tip. Highlights drawn with the vertical tip
  in 1.2.0 to 2.0.0-beta.4 recorded no tip and show round too. Later tip changes will never restyle
  old pages: docs/FORMAT.md, Tip variants.
- **Photo pages**: copy JPG or PNG photos into `grimoire/photos` over USB, then in a notebook pick
  Paper > Photos...: the photo becomes the page's paper, fitted to the page and turned upright the
  way your phone or camera took it, ready to write on. A page added after it gets the notebook's
  own paper.
- **KOReader no longer hangs after Grimoire with Keep alive on**: KOReader came back with the Kindle
  still kept awake but its Keep alive shown off, and five idle minutes later its auto-suspend timer
  spun without end, so KOReader stopped answering. The plugin now turns Keep alive back on when
  KOReader starts on a Kindle that is kept awake (update the plugin: Library > Settings >
  KOReader > Install / update Grimoire plugin).
- **KOReader no longer comes back upside down**: with the Kindle turned while in Grimoire, KOReader
  could come back drawn upside down with every tap landing on the opposite side, its menu out of
  reach. The plugin now sets KOReader's rotation right as it starts on a Kindle Scribe.
- **Open a whole book in Grimoire**: in a PDF, DjVu or comic book in KOReader, Tools > Grimoire >
  Open this book in Grimoire, or Open with... > Grimoire in KOReader's file browser, puts every page
  into the book's notebook at once (KOReader draws them first, with a progress message a tap stops)
  and opens the page you were reading. Pages already in the notebook keep what you wrote on them;
  Annotate this page uses the same notebook. Needs the KOReader plugin from this release.
- **Links between pages**: in the Pages view's Index tab, Add a link, draw around what links, and
  tap the page it goes to (or The Index). A finger's tap on the ink you drew follows it; holding a
  finger on it changes where it goes or removes it. The links are your own ink, so erasing or
  undoing it takes a link away, and older versions show the ink. They are kept in the notebook's
  `links.ini`, apart from the pages: docs/FORMAT.md, Links.
- **Live view**: Library > Share > Live view mirrors Grimoire's screen in a computer's browser on
  the same Wi-Fi, as it changes: open the address on the card and type its one-use code, or scan the
  QR code with a phone. Fit or actual size, full screen, transparent paper and recording in the
  browser (an MP4 in Chrome, Edge, Safari and Firefox where the browser can encode one, and WebM
  where it cannot and for transparent paper, which an MP4 cannot keep clear), the pen shown where it is (a see-through dot
  while it hovers, solid while it touches; Show pen turns it off), an Invert colors switch and a
  Warmth slider: the picture follows Invert colors (the whole page goes black) and the front light's
  warmth (its light turns amber) on the Kindle until you change them, and As on the Kindle goes back
  to following. Picture in picture floats it over your other windows, still live while you work
  elsewhere. Ruled lines and thin strokes keep an even weight at any window size (the browser's own
  scaling made some vanish). The pen never waits for it (each browser gets its own background
  process that sends only what changed), the code never shows on the stream, downloads still need
  Share open, and it turns off with Stop, after 10 minutes with nobody watching, or when Grimoire
  closes.
- **Dark share pages with Grimoire's bat**: the pages a browser opens from Share and Live view are
  dark (your pages, thumbnails and Live view's screen show as the Kindle draws them), with the
  pixel bat by the title and as the browser tab's and the phone's icon.
- **The notebook PDF has a table of contents and links**: named sections become the PDF's outline
  (the bookmarks a PDF reader lists), and links drawn to pages are links in the PDF.

## 2.0.0-beta.4 (candidate, folded into 2.0.0-beta.5)

- **Live ink on gray pages goes straight to black**: on a gray PDF page or template, most
  pixels under a new stroke used to flash white first and turn black a whole display waveform
  later, a white halo and extra lag that white pages never had. Live frames now only move a
  pixel toward its new value, with the same average darkness. Black-and-white pages are
  refreshed exactly as before.
- **PDF download progress**: Share shows completed pages while preparing a notebook PDF,
  then the transfer percentage. Files still go through the browser's normal download
  handling. Interrupted or failed downloads show a message instead of a stuck indicator.
- **Less memory while sharing**: PDF, ZIP and image downloads send from their existing
  export buffer, without building a second full copy for the HTTP response.
- **Keep ink moving during a backlog**: input batches yield to the display and timers after
  8 ms or 64 events. Queued pen samples stay in order; none are discarded by this limit.
- **Less brush work**: skip pixels already darker than a pen's ink, reuse the opacity of
  smooth dab interiors, and skip unused tilt calculations. Pen settings and rendered ink
  are unchanged. Measured CPU improvements and their limits are in docs/INK-PERFORMANCE.md.
- **Faster ink with the driver's pen mode**: live ink now goes out in the Kindle display
  driver's pen mode, which starts a frame on the panel within about 1 ms. Plain refreshes that
  overlap ink still being drawn waited in the driver for up to 300 ms, so quick circles and note
  heads fell behind the pen. A stroke takes pen mode only when no other refresh can still be on
  the panel, as the driver requires. A `pen-mode` file with `off` turns it off.
- **Ballpoint follows the nib closer**: its default smoothing is 0.50 instead of 1.00, which
  made the line trail the pen by 6 to 13 ms at writing speed (now about 2). A ballpoint still at
  the old default moves to the new one; a smoothing you set yourself stays.
- **Grays settle when the pen leaves**: the grayscale cleanup of fresh ink waits until the pen
  leaves the page (or 1.5 s with it hovering), so the next word no longer runs into it.
- **Developer latency report**: optional timing logs measure framebuffer copying and refresh
  submission, and now each live ink frame from the pen's sample to the refresh ioctl, without
  input coordinates. docs/INK-PERFORMANCE.md also lists the panel's own
  waveform timing: on gray paper DU first drives a pixel toward white, so ink there starts up to
  59 ms later than on white paper whatever Grimoire does.

## 2.0.0-beta.3 (2026-10-26)

- **Empty folders leave the PDF mirror too**: removing or renaming an empty Library folder
  also removes its empty directory under grimoire/PDFs. Directories with files, symlinks, and
  directories another logical folder still uses are kept.

- **Retuned pen defaults**: use the latest settings tuned on the Kindle for new installs and
  Pen feel > Reset tool. Existing saved profiles and old stroke parameters are kept.
- **Handwritten eraser practice**: the eraser tab starts with the maintainer's blackletter
  “erase me!” sample, centered in the pad. Whole strokes removes the strokes it touches;
  Precise erases only the part under the tip. Reset pad restores the sample. Whole strokes
  remains the default, and Reset tool on the eraser tab restores that mode too.
- **Fine slider control**: hold a Pen feel slider briefly, then move above or below its track
  to reduce sensitivity. Return within a knob-height band around the track to catch up and
  follow your finger or pen directly again. Release away from the track to keep a fine value.
- **Consistent tool selection**: from the eraser or another pen, tapping the top pen button
  selects its pen; tapping it again opens the picker. A grid button still opens directly.

## 1.2.0 (2026-10-25)

This release includes the changes previously tested as 2.0.0-beta.1 and beta.2. The 2.0 beta
remains a historical prerelease; this release continues the stable 1.x series.

- **Vertical highlighter**: a fixed vertical tip replaces the round tip. This also changes how
  older highlights render when a notebook is opened or exported; preserving the old shape is
  tracked in [issue #5](https://github.com/ganten1998/grimoire-scribe/issues/5).
- **Clear writing area**: Index jumps and pinch zoom add no floating Back or reset-zoom buttons.
  Pinch inward to return to the fitted page view.

- **Journal index**: name pages as section starts from the new Index tab beside Pages. Tap a
  section to open it. Names follow their pages through insertions and deletions, survive
  reopening, and travel in the raw notebook ZIP. The optional `index` shortcut works with custom
  keyboard or controller mappings.
- **New page inside a notebook**: in Pages, add a blank note after the open page, or after the
  last ticked page. It uses the notebook's paper, including when the preceding page is an
  imported PDF page. The first page stays the notebook's cover.
- **Copy page**: tick one page in Pages and copy it right after the original, with its ink and
  paper intact. An image page keeps its picture. Later edits to either page stay on that page.
- **Book-page labels**: pages annotated from KOReader show their source page number in Pages,
  so inserted notes do not make the book's page numbers ambiguous.

## 2.0.0-beta.1 (2026-10-25)

The quality release: what is new since 1.1.0-beta.2. The release candidates 1.1.0-rc.1 to rc.3
(below) are folded into it; there is no 1.1.0. docs/TESTING.md says what was checked on a Kindle.

- **Writing over PDF and image paper**: fast pen updates keep unchanged gray paper intact,
  reducing the temporary shadow around a stroke. Writing over gray images can still feel slower
  than writing over black-and-white paper; further pen-latency work is ongoing.
- **Native PIN checked on firmware 5.20.1.0.2**: from a Kindle Library launch, the Scribe 2024
  showed its own keypad and returned to writing after unlock, upright and sideways.
- **Retuned pen defaults**: the pencil, mechanical pencil, ballpoint, highlighter, fountain pen
  and calligraphy pen start with the settings tuned on the Kindle. Pen feel > Reset tool uses
  the same values. Existing saved pen settings and the ink already on your pages stay as they are.

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
  While shortcuts are enabled, key and controller presses keep the Kindle awake, as the pen does.
  Grimoire adds no Bluetooth pairing and maps no analog sticks; 8BitDo controllers are not
  confirmed yet. docs/SHORTCUTS.md has the details.
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
