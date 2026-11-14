# Backlog

Work that was decided, deferred or found but not done yet. Keep this current: when an item ships,
move it to "Done" with the version; when something is deferred, add it here with who decided and
why. Newest decisions first within each section.

## After the release of 2.1.0 (2026-11-13): what waits

- **Issues open at the release**: #78 (the download check can miss the "preparing" state; pull request #81) and
  #79 (the colour PDF memory test, by 0.4 MiB on one run; pull request #83) wait for their merge. Found on the
  release day: #84 and #86 (scenarios that give the machine a fixed real time: the Graph of all notebooks' two, and
  the single side click's) and #85 (the unit tests' scratch folder is never emptied, so a process id that comes
  round again meets an earlier run's files). #56 (Live view's pen-stall check) was fixed by #82 (Done, below).
  nebuff's: #55 (Japanese: the strings still untranslated show in English), and the Settings redesign, #45, #64,
  #65 and #66.
- **From 2.1.0-beta.2 into 2.1.0**: nebuff's #49 (page turns at the fitted view without a flash, the last three
  pages' drawings kept in memory), #52 (lasso fixes for #50 and #51) and #67 (issue #60), which the owner reported
  checked on 2026-11-13.

## nebuff's main and Clear page (2.1.0-beta.2, lane M, 2026-11-11): what waits

- **Bulk erase slowdown (issue #42) is not fixed in the app.** PR #43 ("Fix slowdown") changed the C tree only
  (`src/page.c`: the render's work pump every 512 points instead of 128; and HOSTCC to gcc in the Makefile). The
  app is the Rust port, whose renderer (`rust/src/page.rs`, `page_render`) still pumps every 128 points, on
  purpose: the pump reads the pens during a long render, whose kernel buffers hold 50 to 175 ms
  (docs/INK-PERFORMANCE.md), so a slower pump would lose pen samples rather than speed the erase. The ~3 s after a
  bulk erase needs a trace on the Scribe (grimoire.log's erase and undo timings on a dense page) before a fix.
- **The C tree's other main commits** (#27 to #34: the share server's route table, its NULL checks and SIGCHLD
  handler, the page turn that skips a save of an unchanged page, the too-small link stroke kept as ink) change only
  `src/`, which the app does not build; the Rust share server and editor have their own code for each. Their
  behaviour in the app was not asked for and is not ported.
- **An Undo button on the toast.** Clear page from a gesture or a key says "Page cleared. Undo brings it back."
  The toast is an overlay with no controls (never in App.screen, a Live view layer), so a button on it would need
  hit testing ahead of the screen's own input, a pen contact on it kept from the page, and Live view's copy. For
  a later version if the owner wants it.
- **Japanese for the new strings** (nebuff): "Display time as 12 hour", "Disable", the 12-hour date
  ("%s %d, %d:%02d %s"), AM and PM (context "date"), "Page cleared. Undo brings it back.". Until then a
  Japanese 12-hour clock reads the month in Japanese and the rest as English writes it.
- **Disable on a paired row's second card** clears only that direction (the first card's Disable clears both).

## Settings > Wi-Fi (2.1.0-beta.2, lane W): choices for the owner, and what waits

- **Not confirmed on a Scribe (2026-11-11)**: the helper's network verbs are built on KOReader's Kindle backend
  and have not met wifid. The lipc-hash-prop output format (the reader knows quoted values and fails closed on
  anything else), whether wifid lists the saved networks with the radio off, and whether ensureConnection
  switches away from a network the Kindle is connected to are the read-only probes and checks at the end of
  docs/TESTING.md ("Wi-Fi networks (lane W)"). If probe 4 shows another format, only the helper's reader changes.
- **Choices the brief left open, taken by lane W**: the button sits in Settings' bottom row beside Pair (no other
  place in Settings had room), which moved Gestures' tap point from x = 890 to 1000; a failed switch changes
  nothing more (no switch back to the network before: the Kindle's own service decides, and only an explicit tap
  changes the network); the saved networks are read from profileData (all of them, in range or not) rather than
  a scan (KOReader's Kindle code does not use the scan list while connected, the case a switch starts from).
- **Left for later**: the Share screen's Wi-Fi card (and Live view's pairing card) still turns the radio on and off
  only; it could open the same list. A scan (scanList) could mark which saved networks are in range, with their
  signal. A saved network whose name the tool writes with an escape (a backslash) is left out of the list. Adding
  a network with its password stays out (the brief: the Kindle's settings and KOReader do it).

## PDF import (2.1.0-beta.2, lane L5): what waits, and what is for the owner

- **S1 ran on the owner's Scribe (2026-11-09)**: every sandbox step works there, a private mapping of a PDF on
  `/mnt/us` works (so PDFs are mapped), and a planner page draws in 90 ms (146 ms at most), a photo page in 230 ms,
  in 27 to 38 MB. The plan's numbers in `sandbox::s1` were kept (30 to 60 times the room those pages need; the
  module's header has the measurements); D12 is not triggered. Not measured: a page that is one huge scan.
  An end-to-end import ran sandboxed on the Scribe on 2026-11-09.
- **A picture folder when no worker can be had (2026-11-10)**: pictures fail closed on the Kindle, and such a
  folder counts no try, so the Library shows "Couldn't import page folder (see grimoire.log)." at every scan
  (at most every 10 s, Library in front) until the sandbox works again; a folder picked in place is not retried.
  Only if the sandbox ever fails on a device: a quiet "no worker" ending the UI does not toast would stop it.
- **Links and the outline (lane L6)**: DONE in 2.1.0-beta.2 (FORMAT.md, "Imports: PDFs"). Left for later: web links
  are counted and left out (the optional QR card, 8.I of the plan, is the way to keep them); the outline names
  only new notebooks' pages (a page's own name would keep it, which matters only when a fill comes in 2.2);
  a worker caps at 60,000 links a PDF (after the water level a notebook keeps 14,400 at most, but a PDF past the
  cap loses its later pages' links before the densest-first trim sees them); `/Next` action chains, JavaScript
  and `/GoToR` are ignored; a Link that is hidden (`/F` bit 2) or not viewed (bit 6) is skipped (a choice the plan
  did not settle: it would be a dead tap area over visible paper); a toast names one thing (when both blank pages
  and left-out links apply, the pages' ending wins and the links are in the log); the name tree's walk order
  decides which entries a 100,000-entry cap keeps.
- **Pictures in the worker (lane L7)**: done in `picwork.rs` (`WorkKind::Picture`). Left, and why: the seccomp
  allow-list and the worker's address space for pictures (`s1::PICTURE_AS_MARGIN`, `PICTURE_SLICE`,
  `PICTURE_CPU_SECS`, `PICTURE_WATCHDOG_MS`) are the plan's numbers, kept after S1 (the Linux `sandbox` tests,
  run as root for the first time on 2026-11-10, decode a picture in the real sandbox:
  `pictures_decode_in_the_real_sandbox`); a
  worker killed by the filter marks its picture `.bad` rather than falling back, so a call the list lacks would
  mark pictures bad on a device until it is added (the log names the call); KOReader's page PNGs and Grimoire's
  own `bg/` pictures stay decoded by the UI (a later option: re-encode KOReader's through the worker, which would
  change the UI process's book flow); the no-worker flag set in one job child (on the Kindle pictures then wait;
  elsewhere they fall back) does not reach the UI or the next child (each finds out for itself, at the cost of one
  failed start). (`install_picture` no longer starts a worker
  on the UI thread: a template or photo with no preview yet waits for the previews job, the fix round's M13.)
- **The picker (lane L8)** and **the notebook PDF's colour and caps (lane L9)**: done in 2.1.0-beta.2 (their own
  sections below).
- **Not done**: a card for the notebook being made. An update while a detached import runs has no scenario: the
  headless runner cannot make a second build to update to, and how the Kindle's package manager replaces a
  running binary (a rename over it, or a write into it) is unknown. What holds by construction: the detached
  child and its writer go on in the old image; on the Kindle a worker is `/proc/self/exe`, the old image too;
  a worker that answers another build is refused as a worker that cannot start (nothing counted, the pages made
  kept, the session's imports stop), and the next start resumes the import with the new build. To check on the
  Scribe with a real update (TESTING.md, the fix round's checks).
- **The CPU budget's numbers wait for S1** (`s1::JOB_CPU_BASE_SECS`, `JOB_CPU_PER_PAGE_SECS`). The mechanism is
  built (the fix round, M10): the budget counts the job's own workers from its probe on, and a job past it stops,
  keeping its pages, and goes on at a later scan with a fresh budget; three stops (a cut-off writer counts as
  one) end the import whole.
- **The fix round's choices (2026-11-09; the plan and the review left them open)**: no room is never the file's
  fault: nothing is counted, the work folder goes (its space is what the user needs back), and the scan takes
  the PDF again only once 16 MB more is free (a session's memory: a restart tries once); a later slice whose
  workers cannot open the PDF three times fails the import whole (the review's "bounded"); a worker that cannot
  start stops the session's PDF imports as a sandbox failure does (the review's "as the sandbox failure"), so a
  worker that never starts cannot loop at every scan; a copy of a PDF being imported ends at once and is taken
  again by the scan after that import (the review's first option for H9); a picture-folder import whose card
  the Library shows toasts nothing, as a PDF's (D20); a picked PDF's or folder's mark goes whatever the end.
- **Kept from L5**: a page's blank replacement is a white image page (shared by content); the colour tolerance is
  6 levels (pdfwork::GRAY_TOLERANCE); a PDF whose every page is unreadable fails whole (the fix of L5's merge);
  a PDF that fails before any page keeps its file and costs a try (3 tries, then it waits until it changes; a
  pick elsewhere keeps no record); the "Some pages couldn't be read." ending shows even with the Library in front;
  a notebook in a Library folder gets the toast "Imported notebook" since its card is not on the Library's top.
- **Doc drift 9's last three** (the review of 0c26d912): FORMAT.md's "so Copy page and later fills have room" (M16),
  "a picture replaced ... simply drawn again" (H4) and the picture cache's `.bad` meaning (M12) describe code other
  fix lanes change in the same round; they follow those fixes, not this lane's.
- **The fork window of the lock**: a child that is no import closes its copy of the lock's descriptor first thing,
  but for the microseconds between its fork and that close a `lock_take` sees the lock held (a pick tapped in that
  instant waits, and is queued at the next scan). Linux 4.9 has no close-on-fork flag.
- **The scan's prune of import/.attempts runs in the UI** (the record is otherwise the jobs' to write): it reads
  the file again just before it writes, which leaves a few milliseconds in which a job's note can be lost. Found
  in the fix round, not fixed: moving the prune into the jobs is a change of who owns the file.
- **Crates**: skrifa is now a direct dependency (your own fonts, L10), pinned to the 0.46.2 hayro brings: no
  crate is added. Cargo.lock names a second skrifa and read-fonts (glifo's optional dependency), not built; the build
  has three duplicates (fearless_simd, miniz_oxide, syn), not the plan's five. flate2, smallvec and syn were
  moved to versions published before 2026-11-02 so that every crate is two weeks old at a tag on or after
  2026-11-16 (`tools/check-crate-dates.py --tag-date 2026-11-16` passes).

## The keyboard's caret and text copy and paste (2.1.0-beta.2, last round, lane K): choices for the owner, and what waits

Built from issue #36 (docs/DESIGN.md, Text and the keyboard). The lane chose, the owner to confirm:
- **Where the keys are.** ◀ and ▶, and the edit key beside them (the copy icon), sit at the right end of the text
  line on every page but the count pad's, each a letter key's width: no row was added and no key moved. The text
  box is the line left of them (1229 px upright, 1663 sideways, where it was 1760 and 2380).
- **The edit page** (Gboard's way): Select all, Cut, Copy, Paste, Start and End, with Cancel, abc, space, delete
  and Done along the bottom. A range is selected by a drag across the text (pen or finger) or Shift with the
  arrows on a paired keyboard; there is no on-screen Select toggle.
- **Copy says nothing**, as the owner wanted no "Saved" toast (2026-11-04): the selection stays. Copy and Cut
  with nothing selected do nothing (they do not take the whole text); Paste with nothing copied does nothing.
- **The clipboard** lasts until Grimoire closes and is never written to a file. Go to page's number pad pastes
  the digits alone. Control characters are left out, as a name opened on the keyboard drops them.

What waits:
- A paired keyboard's arrow held down moves the caret once: `hidkeys_event` drops the kernel's autorepeat for
  every key (above). A repeat of its own would need a timer of held text keys.
- Letters typed on a paired keyboard still type nothing in a field (that needs a keyboard layout).
- No word-wise moves (Ctrl or Alt with the arrows), no double tap to select a word, and a drag selects only the
  part of a long name the line shows (Select all, Start and End, or Shift with the arrows reach the rest).
- Cut, Copy and Paste are never drawn disabled when they would do nothing.

## Planned, not scheduled: a sleep screen picture of Grimoire's own (the owner, 2026-11-09)

The owner wants a wallpaper Grimoire sets itself, with no KOReader in it ("no koreader folder for the non
koreader version"; "add it to the list but we aren't ready to build yet"). Today Display's "Sleep screen:
KOReader wallpaper" shows `grimoire/koreader-sleep.png`, which only the KOReader plugin writes, at each
hand-over from KOReader (`sendWallpaper`): KOReader's screensaver must be set to a custom image, a random image
or a cover, and Grimoire opened from KOReader at least once; a start from the Kindle's library reuses the last
snapshot, and with none the page shows.

- **The plan:** a fifth sleep choice, "Sleep screen: a photo", from `grimoire/photos/` (the folder page
  backgrounds already use, filled over USB): one photo picked in Display, or a random one at each sleep. The
  photo job (bgwork_template.rs) fits it to the screen in grey in a background child and keeps the result in
  Grimoire's own folder, so sleeping only reads a ready picture. Nothing of KOReader is read or written.
- **To decide when it is built:** whether "KOReader wallpaper" stays for everyone or shows only when the
  KOReader plugin is installed; fit or fill, and the background around a picture that does not fill the screen;
  how a random pick avoids repeating.

## 2.1 foundations (lane L0, 2026-11-07): choices for the owner, and what waits

Lane L0 built the shared pieces of the 2.1 plan (docs/DESIGN.md, Shared building blocks) with nothing new to
see. Defaults taken, for the owner to confirm:

- **The book's progress changes at most every 2 s** (the plan's rate for every staying notice), where the toast
  changed every 0.6 s. The default keeps a long book from ever flashing the panel; 1 s (browser.md's figure)
  would show the count move faster, still without a flash. Recommended: 2 s, as built.
- **The staying notice is in the toast's place** (bottom, above a sheet's buttons), so the book's progress reads
  where it did. The Font sheet's "Checking 3 fonts..." (the plan: "above its buttons") falls there too.

What waits for the lanes that use them:

- The popout's frame, rows, contacts, refreshes and keys (3.2 in the 2.1 plan) were built by the quick size card
  (L1, below); the Library's item cards (2.2) and the font picker reuse its chip with a preview well.
- The notice's other keys: Import (the PDF import, L5), FontCheck (your own fonts, L10), Loading and Previews
  (the picker, L8; Previews in 2.2). Their screens' rules are in `notice_pick`; Import shows on the Library only
  until the browser has screens of its own.
- fsview's writes (delete as a rename into the trash, `renameat2(RENAME_NOREPLACE)` with its fallback, the
  allow-list, "File changed. Try again.", the free-space refusal and "Kindle storage nearly full."), the
  Loading notice over 1 s, and its first readers (the picker, the PDF import, the KOReader hand-over's walk in
  place of `realpath`).
- `name_cmp`'s readers: the browser, the import order and the font list.
- The Library listing from libindex's cache plus stamps (browser.md 17), past reading each notebook.ini once.

## Quick sizes (2.1.0, lane L1, 2026-11-07): defaults taken, the owner's decisions of 2026-11-08, and what waits

The owner's decisions of 2026-11-08 (they replace the chip table and the pair-pick the lane had taken):

- **Five chips per tool**, and no Pen feel chip on the card (Pen feel stays in the tool picker and Settings); the
  eraser keeps its Whole strokes / Precise row.
- **The table** (full-pressure widths in mm): pencil 0.25 0.35 0.5 0.7 1.0; mechanical pencil 0.3 0.5 0.7 0.9 1.3;
  charcoal 0.5 0.7 1.0 1.4 2.0; ballpoint 0.25 0.35 0.5 0.7 1.0; fineliner 0.25 0.35 0.5 0.7 1.0; marker 0.7 1.0
  1.4 2.0 2.8; fountain pen 0.5 0.7 1.0 1.4 2.0; calligraphy pen 1.0 1.4 2.0 2.8 4.0; highlighter 2.0 2.8 4.0 5.6
  8.0; eraser 1 2 4 8 12. White ink shares its pen's chips.
- **A pick keeps the user's thin-to-thick ratio:** size_max_mm is the chip's width, size_min_mm that width times
  the tool's current min/max (default ratio if the max is unusable), rounded to 0.01, clamped to 0.10 .. width;
  nothing else changes. The lit chip matches size_max_mm within 0.005.
- **Defaults nudged onto the series:** pencil 0.13..0.35, charcoal 0.27..0.50, ballpoint 0.31..0.50, fineliner
  0.25, calligraphy 2.0, highlighter 5.6, eraser 2.86..8.0 (mechanical pencil, marker, fountain pen unchanged).
- **Existing users:** settings_version 4 moves a tool's pair to the new default only when both stored widths still
  equal the old default pair.

Built on the plan's other recommended defaults (the owner has not answered them; each can change after trying it):

- **D2, the hold's delay:** one 600 ms hold, `ui::HOLD_MS`, for the pen buttons as for every other hold.
- **D3 (quicksize.md D1 to D12):** labels in millimetres without trailing zeros ("0.5 mm"); the number is the
  width at full pressure (Pen feel's Max width); a pick keeps the tool's own min/max proportion (2026-11-08, above); the hold also makes the pen the pen in use; a pick closes the card;
  slide to pick only after the touch moved past the slop; white ink shares its pen's sizes; five chips per tool
  (the 2026-11-08 table), the tool's default always one of them; no hold on the folded toolbar box; samples drawn at
  the view's zoom with labels in page millimetres; O1 (the eraser's Whole strokes / Precise row) and O7 (the
  one-time tip) in 2.1.
- **Taken within those:** the eraser's mode row toggles and leaves the card open (a size pick closes it); the
  card opens beside the toolbar with the clock corner kept clear and, while a link is being made, the strip too;
  the hold's pen pick and the tip's bit are written with the size, a pass after the close was presented.
- **The eraser's chips are screen millimetres**, so at a zoom under 1 the 12 mm chip is clamped at the format's 12
  page millimetres.

What waits: O2 (the ink row in a pen's card), O3 (size keys and the `quick-size` action, ids 105 to 107), O4 (your
own chips), O6 (hold in the picker), O8 to O10 stay out; the live ink on the largest chips (charcoal 2, marker
2.8, highlighter 8, eraser 12 mm) is for the owner to measure with `tools/bench-ink.c` (docs/TESTING.md): drop a
top chip if live ink lags. The Japanese for "Hold a pen on the toolbar for its sizes." is for nebuff.

## Classic fonts (2.1.0): choices for the owner, and what waits

The owner's ask of 2026-11-04 ("Custom font support/More fonts in the classic theme") is half built: More
fonts in Classic (Done, below). Decided by the lane on the 2.1 plan's defaults, for him to confirm on the
Scribe (docs/TESTING.md, 2.1.0, Classic fonts):

- **The three families**: Atkinson Hyperlegible Next, Noto Sans, Literata (the plan's D4). The alternates
  measured with them were Inter, Fira Sans and Source Serif 4; a swap is a line in tools/fonts_classic.py and
  FONT_FAMILIES, a new CRC table and the l10n runs.
- **Their sizes**: Atkinson x1.10 and Noto Sans x1.065 match Terminess's x-height; Literata x1.15 is 93% of
  it (x1.23 would match, and cuts the music sheet's preset buttons in Spanish). The owner may want them a
  step smaller or larger after seeing them on e-ink; each is one number in font.rs, with a new CRC table.
- **Latin only**: Greek and Cyrillic left out of the subsets (the lane's brief), so a Greek or Russian name
  shows in Terminess beside the family's letters. Keeping them costs about 185 KB for the six files.
- **The chip shows the short name** ("Font: Atkinson"); the sheet's own words stay in the font in use until
  Done (the plan's D6 and fonts.md 5.7, which drew them in Terminess for custom fonts only).
- **No tabular digits** in Atkinson and Literata (Noto Sans has them): a number that changes in place (a
  slider's value, the page counter) shifts by a pixel or two between values (fonts.md 7.B, deferred).

Waits, from the 2.1 plan:

- **Japanese in Classic** (2.2, nebuff): a smooth Japanese face (Noto Sans JP, +0.97 MB) beside DotGothic16.
- **y_all_screens.qs split by screen group** so the per-font l10n runs shard (the plan's L3): the runs are
  whole for now (six more of about 80 s here).

(Your own fonts and the glyph cache's 4 MB byte cap are built: 2.1.0-beta.2, lane L10, below.)

## Your own fonts (2.1.0-beta.2, lane L10): choices for the owner, and what waits

Built to the 2.1 plan's D30 (Done, below). **Nothing of it has run on a Scribe, and its sandbox steps have not
run anywhere** (the CI job `sandbox` is their first run): docs/TESTING.md, "2.1.0-beta.2, L10", has the device
checks. Where the plan did not settle a thing, what the lane chose, for the owner to confirm:

- **The size of text in your font.** The plan's rule as written: capitals as tall as Terminess's, the scale
  kept between 0.85 and 1.25, then lowered until 95 of 100 of Grimoire's strings are no wider than in
  Terminess. It gives Noto Sans 1.06 (the bundled copy has 1.065, set by hand), Atkinson 0.96 (bundled: 1.10)
  and Literata Bold 1.07 (bundled: 1.15): a custom font comes out a little smaller than the three that were
  fitted screen by screen, since nothing fits a custom font's screens. If your fonts look small on the Scribe,
  the 95 is the number to lower (`fit_strings` in fontcheck.rs); a verdict keeps the scale it was given, so
  delete `grimoire/fonts/.cache` after changing it.
- **The line of a font whose tables are far off.** The plan replaces an ascent or descent more than 1.5 times
  what the letters reach. The lane measures that reach over the capitals, the digits, b d f h k l and the
  accented capitals the font has (not the capitals alone: Literata's line would have been squashed to its
  capitals), replaces a line less than two thirds of the reach as well (letters would stick out of it), and
  gives a font without descenders whose descent is deeper than its capitals are tall a quarter of them.
- **Outlines in font units**, not the plan's sixteenths of a pixel: the unscaled outline is what makes a
  TrueType font's bitmaps stb's own byte for byte, and one outline serves every size, so the check's walk
  covers what the UI draws at any size. A CFF font's fractional coordinates are rounded to whole units (stb's
  own CFF reader cuts them the same).
- **The sizes a custom font is drawn at**: 8 to 160 px of line (the UI's text is 34 to 64 px at 300 dpi, a
  family's scale up to 1.25, and the fit steps down from there). Outside them a glyph comes from Terminess.
- **The sample card is drawn by the font itself** (loaded when its row is tapped, under the crash guard, held
  beside the font in use), not by a picture from the check: the sentence is in the UI's language, which the
  check does not know. If it cannot be loaded (32 MB of fonts held already) the card shows the check's picture.
- **The sheet's own words are Terminess only while a font of the owner's own is in use** (L3's reading of the
  plan, kept: with a bundled font in use they stay in that font).
- **The crash guard strikes fonts, not starts.** A start that finds a mark armed draws none of the fonts that
  were armed (the font in use, its Bold file, the sheet's sample), and everything else as usual. So when the
  mark was the font in use, that start is in Terminess, as the plan says; when Grimoire died while only a
  sample was new, the font in use is still drawn at the next start, and the sample's font has the strike.
- **Marks an earlier run left.** A `.checking` mark found when a job starts is taken away and the font checked
  again (a power cut is not the font's fault); only a job the UI sees die on a file marks it `.bad`.
- **A worker ended by its CPU limit is "Took too long to read."**, not "Unreadable font.".
- **No worker at all** (a sandbox step failed): the unchecked rows say "Unreadable font." for that session
  only; nothing is written, and the next start checks them.
- **A truncated file can pass** when the cut is inside a table the check does not read (the names, the
  PostScript table): it still draws every glyph, and what the UI reads later is those same bytes.
- **Bold by a Bold file**: only one in the same folder as the font, of the same family name, weight 600 or
  more, upright; `font_bold=` names another by hand and is forgotten when another font is picked.
- **The preview carries the style** when it is not Regular ("Inter Bold"), so a family's files tell apart.
- **Gothic lets the font go** (its memory too) and Classic reads it again.
- **A folder of grimoire/fonts named as one of the storage's system folders** (`system`, `koreader`,
  `extensions`...) is not looked into: the walk's deny-list is the storage's.

What waits:

- **Add a font...** (the Kindle's own fonts folder, copied in) comes with 2.2's Files (the plan's D30).
- **The Japanese slot** (a custom or bundled smooth Japanese face in Classic, the plan's L14, with nebuff).
  Kana and kanji of a custom font are already drawn at DotGothic16's em.
- **Kerning for custom fonts** (GPOS pairs through read-fonts, fonts.md 7.G): none today; the bundled fonts
  keep stb's.
- **Tabular digits** (fonts.md 7.B) and **the other faces of a collection** (7.I: a `.ttc` gives its first).
- **Checked fonts for PDFs that name a font they do not embed** (hayro's `font_resolver`, fonts.md 8).
- **The verdict's `cover=` counts** (Latin-1, Extended-A, Greek, Cyrillic, kana, kanji) are written and not
  shown anywhere yet (2.2's Files details, the Japanese slot's "has kana").

From the whole-release review of 2.1.0-beta.2 (the fix round, lane F3; the review's finding IDs). Fixed there: H8,
M17, M19, M20, the watchdog timeout that was a verdict for good (a second timeout is the verdict now), the
Bold-only letters of a name, a Done on a font that does not load. Left, with what each costs:

- **Only the regular's advances are fitted** (review, L10 item 3). `fit_strings` measures Grimoire's labels in
  the regular face while most chrome is Bold, so more than 5 labels in 100 can end in "..." with a font whose Bold
  is wider. The cost is a few clipped labels in a bold-heavy font; the fix is to fit on the bold's advances too
  (fontcheck.rs, and every verdict is checked again).
- **Outside 8 to 160 px the glyphs are Terminess's but the line metrics stay the font's** (item 6): a very small
  or very large text in a font with a tall line leaves a gap or crowds a line. Unmeasured on the Scribe, where the
  UI's sizes sit inside the range.
- **A sample tap reads and hashes up to 24 MB on the UI thread** (item 7). Moving it off the UI thread needs the
  loaded face to cross a thread (`SafeFace` holds `Rc`s), so it was not a simple change. The cost is a pause
  (unmeasured; 24 MB over FUSE) at the tap of a very big font's row; the check job's hash is already off-thread.
- **With a bundled family in use the sheet's chrome is that family, not Terminess** (item 8): a family without
  the sheet's letters (or an unreadable one) can make the sheet harder to read; the plan wants Terminess only
  while a font of the owner's own is in use, which is what is built.
- **An italic regular pairs with an upright Bold, and a Bold file added later pairs only at the next load**
  (item 14): `pair_of` looks at weight and family only for the regular, and the pair is found at load.
- **A worker killed for memory, and the check writer's death, are verdicts for good** (`.bad`): a font that
  needed more memory than the margin gave stays "Unreadable font." until its file changes, even after the
  margin is raised. (A watchdog timeout now gets one retry.)
- **A guard mark and a sample.** A sample that reached the panel is settled, so Done on it draws the first whole
  screen with no mark on disk; and the Bold file and the regular are struck together (a strike on one is on
  both). Both are the plan's 4.2.8 as built; a stricter rule is a larger state machine.
- **A chosen font past the 200 listed files shows as "File missing."** in the sheet (the folder listing stops at
  200 files); the font still loads at a start if it was checked, the row alone is wrong.
- **Rows stay at "Checking..." while a failed check waits for its retry** (3 s, 15 s, 60 s, then the session
  stops asking and the rows say "Unreadable font."): the notice says it, the row does not say why.
- **The plan's 3.7 retry "right after a wake"** is built as one retry a minute after a first timeout, not as a
  wake event (the UI has no wake event the check could wait for).

- **The fit check cannot cover a custom font**: l10n-check runs the Font sheet over the fixtures in every
  language, but a real font's widths are its own; labels that do not fit end in "...".
- **Left and Right do not turn the pages of Your fonts** (the sheet's arrows do).
- **`sandbox::s1::FONT_*`** (20 s slices, 10 min a font, 60 s of CPU a worker, a 30 s watchdog, 64 MiB over the
  file) are the plan's numbers, placeholders until S1 measures the Scribe; `s1::WORKER_IMAGE` (11 MiB) is now
  only about 0.9 MiB above the binary's size, which the margins absorb.
- **A What's new line** for 2.1.0-beta.2's notes (docs/whatsnew), which the owner writes.
- **Japanese** for the 15 new strings (nebuff): English shows until then.

## A cleared page's old picture (2.1.0): fixed; what is left for the owner

A page with a current kept drawing showed its old picture after Clear page or a new paper, template or photo, and
kept it (2.0.0 to 2.0.1; CHANGELOG 2.1.0, docs/TESTING.md). Fixed: a kept drawing stands for a page only while the
page has nothing unsaved (editor.rs page_is_its_file). Left open:

- **A new paper on a slow page says nothing for seconds.** It draws the whole page (11 and 28 s on the owner's two
  stress pages), as any page drawn whole did before kept drawings, where 2.0.1, from a kept drawing, was at once (and wrong). It could
  say "Loading page..." as a page that opens does (render_page_open's toast, which is due then: the page is
  changed). The lane's recommendation: yes, in 2.1.0; not done, as it is a new notice.
- **Old pictures 2.0.0 and 2.0.1 kept, on pages and notebooks not opened since** (the retirement commits: every
  drawing they kept is drawn again once, store.rs RENDER_TAG_VERSION). Covered: as the editor draws such a page
  whole from its strokes, nothing unsaved and its picture there (as it opens, as its template's or photo's picture
  comes, or as the Links bar or "Back to page N" stays on it), it removes the old drawing, current for the page
  file or not (a page written on again since included), makes the page's Pages preview again from what it drew
  (a Pages view decodes it again: a tile goes by its preview file too) and page 1's Library card as the page is
  left. The previews job keeps no drawing of its own over an old one, so a page it draws is still drawn from its
  strokes at its next open. Left: until a page is drawn so, a page cleared or given a new paper under 2.0.0 or
  2.0.1 can still show its old picture in the Pages view (the previews job leaves current previews alone), and a
  notebook whose page 1 is not opened again keeps it on its Library card. Making them again without opening the
  page means drawing it whole (on a Kindle, 11 and 28 s for the owner's two stress pages): the previews job could
  do it in the background for each page whose drawing came from 2.0.0 or 2.0.1. The lane's recommendation: not
  unless an owner's notebook shows one; few pages were cleared or given a new paper with a kept drawing current.
- **Undo of a Clear on a dense page keeps no drawing.** It draws the page whole (no patch: Clear keeps none) but
  times only drawings made by render_page_full, so the page is drawn again, once, at its next open, and keeps its
  drawing then. Small; left as it is.

## What's new by version (2.1.0): choices for the owner to confirm

The owner's request of 2026-11-06 ("wire read all button to a list of each UNIQUE release note, and allow users
to easily browse the changes by version") is built (docs/DESIGN.md, What's new). Decided by the lane, for him to
confirm or change:

- **2.0.1 is not released** (the owner, 2026-11-07: "skip 2.0.1 and roll it in with 2.1. The note reading itself
  isn't polished enough to come out of here"): the wake fix and Read all's list are 2.1.0's, and so is the
  draft note, moved to docs/whatsnew/2.1.0.md with its text as written. The 2.0.1 release commit (4425ef9) is
  abandoned; VERSION stays 2.0.0 until 2.1's bump.
- **2.1.0's notes are a draft** (docs/whatsnew/2.1.0.md): the title, the wake fix and this list in one bullet
  each, and the signature. The owner vets and edits them before the release. They do not mention yet that
  Read all now fits each version's notes on one screen (below).
- **Fit all** (the owner, 2026-11-07: "either smaller text for all past messages, or fit all (which i lean
  toward)"): built (docs/DESIGN.md, A version's notes fit one screen). Decided by the lane: the floor is 34 px,
  the size the card sets the same notes in and the list its rows' lines (no sheet sets body text smaller); the
  steps are half a pixel; the gaps and signature shrink with the text, the title, the version line and the
  buttons do not.
- **Set closer before smaller** (the owner, 2026-11-07: "The note reading itself isn't polished enough"; a
  review: in Gothic, fitting by size alone dropped 2.0.0's notes to the card's type with 122 to 127
  characters a line over 1095 of 1940 px upright). Decided by the lane: at 44 px the leading tightens first,
  from 1.3 to at least 1.15, and the gaps to at least 60% of theirs; only then does the size step down, and a
  step down gives its room back to the leading and the gaps, up to the open look, never past it. 2.0.0's
  notes now keep 44 px in Gothic upright (the leading 1.19) and in Classic's Atkinson, Noto Sans and Literata
  sideways, and take 41.5 px at 1.15 in Terminess sideways (38.5 px before). **Left for the owner:** Gothic
  sideways has no room for 44 px type (2.0.0's notes need 1871 px at the tightest, the letter 1460, of 1320),
  so both are still set in Jersey 10's next size, the card's, in the leading they always had, using 1095 and
  997 of the 1320 px the tallest sheet has there (their sheets start under the top bar). Opening the leading
  past 1.3 to fill it, or a narrower measure there, is his call; 1.15 as the leading's floor likewise.
- **Bullets keep the card's proportion** (a review): a note's dots are to its type as the card's are to the
  card's, by the type's height as drawn, so 44 px notes have larger dots than 2.0.0's Read all had (1.29 times
  the card's in Classic, 1.5 in Gothic, whose 44 px type is that much taller); at the card's size, the card's.
- **A sheet stays off the Library's top bar and pager**, not asked for: the sheet of 2.0.0's notes sideways in
  Gothic, once they fit, ended across the pager's Page 1 of 1 and cut it in half, so a note's sheet (and since
  2.1.0's review the list's, which with 6 versions sideways or 9 upright would end there too) that would end
  over the pager reaches down to the margin; 2.0.0-beta.1's letter upright is 7 px taller for it (its sheet
  already covered the label). And a sheet that would start across the top bar's buttons starts at the margin
  above them, as the tallest does, where sheet_rect started it across them (2.0.0's notes and the letter sideways
  in Classic: judged from the shots, the clipped icon tops above the sheet read worse than the tallest sheet's
  slivers). A sheet that starts below the buttons keeps its place, the bat and the buttons whole above it: with
  Noto Sans the letter sideways starts at 186 px, over the bar's rule only, and reaches the margin as before
  (moved up, it would have hidden the bar to show the pager). The card's room ends at the pager too (sideways
  in Gothic it covered the pager's left button and its P): 2.0.0's card shows its first 9 blocks sideways, the
  rest Read all's.
- **Long words and Japanese in the notes**: a word wider than the sheet (a link) is broken between its
  characters on lines of its own, and Japanese or Chinese between any two characters, a short list of closing
  and opening marks kept with their neighbours; no hyphen. None of today's notes have either.
- **Dates come from CHANGELOG.md's headings**, built in (no date line in the notes): 2.1.0 reads "unreleased"
  until the bump commit dates its section. Dates show as 2026-11-05 in every language.
- **docs/WHATSNEW.md is gone** (moved to docs/whatsnew/2.0.0.md), no pointer file: links to it at the 2.0.0 tags
  still work.
- **Notes shared by several versions** are named "2.0.0, 2.0.0-beta.2" (a comma, no word to translate). None do
  today: 2.0.0-beta.2 has no file of its own.
- **Esc goes back one layer**: a version's notes to the list, the list to the card (as the Templates sheet goes
  back to New notebook, and as SHORTCUTS.md's "one level a press"), the card closes. In 2.0.0 Esc on Read all's
  sheet closed What's new; the list has no on-screen way back to the card, only Esc. **Enter on the list opens
  the running version's notes**, as there is no highlighted row (the Library's sheets have none).
- **Japanese for "This version" and "unreleased"** is empty in po/ja.po: the list shows them in English in
  Japanese until nebuff translates them. Back reuses the Templates sheet's, already translated.

## The lasso drag (2.0.0): the owner's report of 2026-11-05, fixed; what is left for the owner

The report, verbatim: "the copy paste lasso function is horrifically slow on drag with anything containing
colored in or multiple strokes." On his Scribe with the published 2.0.0-beta.2, his notebook's page 5 of 6
(Hexagon paper): 12 Pencil strokes of 9,852 points, colored-in hexagons, copied and pasted over the originals,
then dragged, Live view on. The evidence (his grimoire.log, page and clipboard) is in the maintainer's lab folder
(lasso-drag-2026-11-05); his drawing is never a fixture. What it found: every pen sample composed and refreshed
the box joined with the bar, 1680 x 365 px, and drew the selection's halo again over all 9,852 points (about 11 ms
of framebuffer copy and 10 ms of drawing a sample on the device, measured from the log: about 40 overlapping DU
refreshes a second), one sample a loop turn while the pen reports one every 2 to 3 ms. Fixed in 2.1.0 (Done
below; docs/DESIGN.md, The drag). Left open:

- **A ghost of the ink while dragging** (the owner's call; the CHANGELOG has promised the outline alone since
  beta.2, and LASSO-DESIGN.md deferred "a dragged-ink preview"): a 1-bit picture of the selection made once at
  the first move (its strokes as thin lines, or its halo's edge, a few ms on the device for 9,852 points),
  moved with the outline. It would present about the box's area at each show, 1 to 2 ms on the device, and
  the box's inside would then be refreshed by DU.
- **The bar's place shows in dots while dragging** (the owner's call): the first move takes the bar away by one
  DU of its rect (1680 x 130 px on his page), which leaves the page there in black-and-white dots until the
  lift's grayscale refresh. A GL16 of that rect instead would keep it gray, but it is slower on the panel and the
  first move's strips, refreshed by DU right after, may meet it there; not tried on the device.
- **The lift on a page like his** still takes about 0.6 s on the device (263 ms to draw the old place, which
  still holds the originals under the paste, 290 ms to place the moved ink with fresh grain, then one GL16):
  the move itself, unchanged. Its grayscale refresh is merged with the panel's repaint into one rect (1822 x
  1734 there); refreshing the panel on its own would trim it a little.
- **A drag on a cold Kindle** shows less often: each show waits for the last DU, which takes 294 ms at 15 to
  18 C (docs/INK-PERFORMANCE.md), so about 3 a second. Not measured on the device yet (docs/TESTING.md, 2.1.0,
  lasso drag).
- Found while diagnosing it, not part of the fix (for the owner to schedule):
  - **Kept drawings and previews were all retired by each update** (cached=0 for every page at the first
    preview batch after a new build, on Nov 4 and 5), while the renderer's fingerprint stayed the same. A
    hypothesis to check on the device: preview_stamp (store.rs) keys them on the page file's st_ino and
    ctime; on vfat (/mnt/us) Linux makes inode numbers up when it reads the inode, so a remount or an evicted
    inode changes them. Check with `stat -c '%i %Z'` on a page before and after a restart, its `.full.stamp`,
    and `mount | grep /mnt/us`; if it holds, a stamp of size, mtime with nanoseconds and a hash of the file's
    head and tail instead.
  - **A 9.2 s touch on the editor screen** (06:56:24 in his log) was opening his "2.0-b1 DEMO" notebook from
    the Graph, its open page drawn whole on the main loop with no kept drawing (the item above).
  - **The editor and the previews job drew the same dense page at once** (06:42, his Seyes p171, 1.48 MB, 5.4
    and 5.6 s on the two cores): neither had a kept drawing, and the job keeps one only once it has drawn.

## Found on the owner's stress pages (2026-11-05): candidates for 2.0.0, the owner decides

From STRESS TEST 2 (648 strokes, 153,588 points, 2.8 MB) and STRESS TEST 3 (698 strokes, 163,950 points,
3.0 MB) in the owner's "2.0b2 DEMO", written on the Scribe on 2026-11-04 and 05 with 2.0.0-beta.2 builds.
The record and the evidence are in the maintainer's lab folder (stress-tests-2026-11-05/RECORD.md).
Writing stayed fast at any size (median pen to screen 0.6 to 0.9 ms); the costs are elsewhere:

- **The PDF mirror export redraws every page from strokes at each close after a change**: 51.4 s and 48.6 s
  for this 5-page notebook, the second for five strokes on page 1. It could reuse current kept drawings, or
  redraw only the pages that changed.
- **Zoom steps on the stress pages took 0.56 to 0.66 s each**, with frame copies of 0.57 to 0.61 s; a zoom on
  a lighter page minutes later had none. Not explained yet.
- **A stroke-eraser move on STRESS TEST 2 took 2.1 s** at 2.38 MB, its screen update 2 s late (as the CHANGELOG's
  about 3 s for a page-wide marker stroke on the first stress page).
- **"Loading page..." and undo are not logged**, so a record cannot say whether the notice showed or undo ran.
- **Limits, by extrapolation only**: nothing caps strokes or points on a page; the nearest limit is the 120 s
  heartbeat for a page drawn whole, reached at roughly 13 to 15 MB of STRESS TEST 3's mix (about 35 minutes
  of pen contact on one page). Points are kept at the pen's full rate, about 6.3 KB a second of contact.
- **As fixtures**: real device data with strokes of up to 7,028 points (the synthetic dense fixture has 10 to
  129), but 2.8 to 3.0 MB each and the owner's own drawings, so adding one is the owner's call.

## The Terminess diet (2.0.0-beta.2): choices for the owner

The owner chose Terminess with Terminus's text and the Material Design icons only, with the Apache-2.0
notice (assets/README.md). Left open by the builder and the reviews, 2026-11-04:

- **Embedded bitmaps kept.** Dropping them saves about 305 KB a weight (0.6 MB of the app), but FreeType
  (KOReader) would then draw outlines instead of Terminus's own pixel bitmaps at 12 to 32 px.
- **Nerd Fonts' glyphs at ordinary code points are gone** (box drawing and blocks U+2500-U+259F, the
  heart, the lightning, the menu sign, the power symbols, the heavy angle brackets): a name shows
  without them, KOReader draws them from its fallback fonts. Terminus 4.49.3's own box drawing, blocks
  and heart (OFL) could be merged back from upstream, in Terminus's shapes rather than those beta.1 drew.
- **No visible link to the fonts' licenses on the share pages.** `/font/licenses.txt` is served, each
  font's answer names it (`Link: rel="license"`) and the pages' style says where it is; a footer link
  would need a translated label.
- **The published 2.0.0-beta.1 still offers the full fonts** with only the OFL text (no CC BY
  attribution for Font Awesome and Codicons, and the unlicensed Font Logos), until it is withdrawn, its
  notes amended, or beta.2 replaces it.

## The keyboard scheme, the Pages view and the Graph's keys (2.0.0-beta.2): choices for the owner to confirm

Built from the owner's words of 2026-11-03 for a keyboard paired with the Kindle. Left out (owner to confirm):
a keyboard focus in the Library (Left/Right turn the grid's screens instead); a Delete key in the Pages view;
the Kindle noticing a keyboard while shortcuts are off.

The keyboard scheme:

- Owner: "Escape for CMD S for saving anything not saved anywhere": read as Ctrl+S (Cmd+S on an Apple
  keyboard) saves now (everywhere; its "Saved" toast went in the owner's toast pass of 2026-11-04), and Esc
  at the Library's top asks "Exit Grimoire?" (Yes and No). The Exit button still leaves at once.
- Owner: "Left and right do what you'd expect but up and down do nothing here or anywhere else for now":
  Left and Right turn the Library's screens (no focus ring); Up and Down are unbound but in the Graph, and the
  count pad no longer steps with them (its - and + do).
- Esc inside a Library folder goes up to the top first, then asks. Esc on a page with nothing in front goes
  back to the Library, as the toolbar's Library button (saving first).
- R opens the page's naming keyboard as the action does today, back to the page after; O acts in the Library
  only. PageUp and PageDown turn pages on a Kindle keyboard too.
- Not done here: Enter and the Left/Right highlight in the Delete question (Esc answers Keep it; only Exit
  Grimoire? has a highlight).

The Pages view (the owner: "CMD N prompts for a page number with numpad immediately and if enter is pressed or
confirm via touch, 1 will be the default so it will create a new page." "CMD + F says page number or name, but
it needs its own dialog copy in the pages view that doesn't just drag you out to the notebook while you
input"):

- **Ctrl+A also turns Multi-select on**, as the Select all button does (a later plain tap then toggles one
  tile instead of picking it alone); Select none or Multi-select gives the plain tap back.
- **The count pad opens at 1**, the toolbar's + held included.
- **Esc during a drag cancels it** for `close-menu` too, since a drag otherwise swallowed the key: the pages
  stay where they were.
- **Esc on a tile's card closes the card alone for `close-menu` too** (a file that keeps
  `key.Escape=close-menu` keeps that); clearing the ticks stays `back`'s alone.

The Graph's keys (the owner: "Add left right up down positioning ONLY FOR GRAPH VIEW - with - + zoom ... V will
change the view toggle, and I need it to be View: All and View: Adjacent or similar so the wording isn't
jumbled"):

- **The view button after a pan always switches**: two taps (the other view and back, each fitted), or the
  fit-page key.
- **Esc in the Graph**: the selection, then the Graph. A file that keeps `key.Escape=close-menu` closes the
  Graph.
- **Old shortcuts.ini files** that bind none of the Graph's actions get its arrows and V in memory (as the
  D-pad's lines), so an edited file from an earlier build (an unedited starter map is upgraded) still pans in
  the Graph; deleting every Graph line on purpose brings them back the same way. The zoom keys are the file's
  own (the new starter map has - and +).
- **Not built**: the selection is not kept when the Graph closes.

## Next: 2.1 (deferred by the owner, 2026-10-31)

- **Moving a folder onto a folder of the same name: a merge dialog**: DONE in 2.1 (branch v21/l2-merge; the
  owner's toast-review note closed). Not part of it: a failed graph-omit.ini write after a merge or move is
  logged, not rolled back (the Graph's left-out lines stay by the old path until the next change), and a
  merged folder's Leave-out line now covers the folder it merged into.

- **PDF planners with their hyperlinks, imported without KOReader** (owner, 2026-11-03; next after the
  2.0.0-beta.2 release candidate). A user asked to use a PDF planner whose pages link to each other. Two parts:
  1. Links: each PDF link (its box and the page it jumps to) becomes a Grimoire link area pointing at the
     notebook page that holds the target page (source.ini's map); a planner is imported whole so every target
     exists; web links are left out at first.
  2. Owner ruling: imports are no longer gated by KOReader. Grimoire itself finds PDFs placed in a folder it
     watches (as it already does for page images in grimoire/import/<Name>/) and imports them, links included.
     That needs a PDF renderer in Grimoire on the Kindle: choose by licence and size (PDFium, BSD-3, keeps every
     licensing option open; MuPDF is AGPL-3.0; poppler GPL), cross-built for the Kindle, with page rendering in
     the background job so the editor never blocks. KOReader's "Open this book" path stays as a second way in.

- **Cross-platform tablet bridge** (owner, 2026-10-31; started, not finished: the weekly limit). One small
  dependency-free Rust tool for Windows (InjectSyntheticPointerInput PT_PEN), macOS (CGEvent tablet subtype)
  and Linux (uinput pen tablet), reading Grimoire's own Live view pen stream (/live/pen, pairing code; no SSH),
  plus a device-side Tablet mode so the pen draws no ink while it drives the computer. tools/scribe-tablet.c
  (Mac, over KOReader's SSH, round 1) is the stopgap; plan in the rc.12 session notes.
- **The KOReader library entry for Grimoire** as an option in the KOReader sheet (owner, 2026-10-31).
- **Stroke eraser sweeps on dense pages**: batch a sweep's redraws (needs an input-peek API) and cache brush
  checkpoints (the eraser lane's notes, rc.12).
- **Custom Paper builder.** A picker tile that builds custom versions of ruled, dot grid, graph
  paper, cross grid, isometric, hexagon and Seyes paper (spacing, line weight, margins). Needs a
  new paper namespace in the page format (docs/FORMAT.md); older versions show such pages as
  blank "Newer paper".
- **Smart-shape snapping to geometry.** A snapped shape's corners, ends and edges stick to other
  shapes, lines and points already on the page.
- **Open Amazon's own Scribe notebooks in Grimoire.** Native notebooks are folders in
  `/mnt/us/.notebooks/` holding an unencrypted SQLite file `nbk` (tables `fragments`,
  `fragment_properties`, `local_delta_fragments`, `local_action_fragments`, `capabilities`; the
  stroke data likely Amazon Ion). Verify against real notebooks on the device before trusting any
  format notes; the etienneunai fork's converter assumes a 1024-byte header these files do not
  have. Credit the fork if any of its findings hold.

## Exact-area lasso and the live preview (beta.2 lane B3): deferred

Decided by the lane B3 design (docs/beta2/PIXEL-LASSO.md, 2026-11-03) and its builder, owner to confirm:

- **Trimming carried erase copies** to the reach of an exact-area group's ink (design 1.4): fewer scoped
  floors and a smaller clipboard. Copies travel whole today, which is exact inside the group.
- **"Save selection as picture"** (design 3, owner question 3): a raster export of a selection. A raster
  selection as a page object was rejected (no tool id or chunk older versions keep).
- **Remembering the lasso mode in settings.ini** (owner question 1): it lasts until Grimoire restarts.
- **A `select-exact` row in Settings > Shortcuts > Buttons**: its Editing list is full (11 rows); bind it in
  shortcuts.ini or gestures.ini meanwhile (builder, 2026-11-03).
- **Cutting a calligraphy stroke whose tilt widens or lightens it**: its nib's smoothing restarts at a cut
  and only where the tilt changes nothing but the nib can a piece start it where it was, so such a stroke
  is taken whole (builder, 2026-11-03: measured up to 249 levels off near the cut otherwise).
- **Jitter after a cut**: pencil, mechanical pencil and charcoal draw a cut part's per-dab opacity jitter
  afresh (the dab count restarts; no format field holds it): measured up to 54 levels in single pixels,
  0.39 of a level on average over the stroke's box. A per-piece jitter offset would need a format field.
- **Shift flips one lasso's mode, as Alt does** (builder, 2026-11-03, after review; owner to confirm the
  key), on a keyboard paired with the Kindle.
- **TRANSLATORS comments xgettext drops** (builder, 2026-11-03, found in review): xgettext keeps a comment
  only on the line right above the one holding `n_(`/`tr(`. The two lasso actions' comments are fixed (and
  `test_actions` checks them in po/grimoire.pot); about 50 older ones are not, among them the other 11 in
  actions.rs (the Buttons sheet's groups and rows, above `ai(`, `nm(` or `ActionGroup {`) and many in
  library.rs above `csnprintf!(` or `menu_row(`. Not this lane's strings; a sweep for its own change.
- **The exact-area preview's ticks** sit halfway between judged samples (1 mm apart), up to 0.5 mm from the
  final cut at a dab; a stroke grazed by less than 1 mm may be cut without a previewed tick (design 2.3).

## Lasso (beta.2 lane B): deferred (decided by the lane B design, 2026-11-02, owner to confirm)

- **Floor-aware compaction and dead-layer reclaim.** Pages with floors (ext[1] != 0) are left as they are
  by every version's compaction, this one included; `page_droppable` would need to keep markers and pass
  floors through the pieces path (wired in render_from, not yet reachable). Layers are not reclaimed, so a
  page holds at most 255 scoped groups before a paste there is refused.
- **Per-group tooth origin.** Pencil, mechanical pencil, charcoal and white-pencil grain belongs to the
  paper (absolute tooth, absolute charcoal dust): a moved pencil word shows the tooth of its new spot.
  Pixel-identical grain after a move would need per-group data.
- **"Remove erasure" on a selection, and white-out as a movable object** (owner question 1): today a lasso
  around white ink alone selects nothing; uncovering stays Undo.
- **A faster old-place redraw on dense pages** (the stroke eraser's slow path; measured on a generated
  1,000-stroke page in the headless harness: move 12 ms old place + 22 ms new place on the Mac).
- **An index target across notebooks** (with lane C): a pasted link to the Index is left out with a toast.
- **A dragged-ink preview** instead of the bare outline while dragging.
- **Preview stamps of layered pages** (design 3.4): Pages previews are keyed on the page file's stamp, so a
  preview an older version made of a layered page shows its flat look until the page changes. Kept
  drawings (`.full`) already carry the `|L` tag.
- **The C tree** (src/page.c) has no floor branch: the Rust tree is the shipping one; the C tree draws a
  scoped group flat, as 1.1 to 2.0.0-beta.1 do.
- **Enclosure tuning on real handwriting**: half the visible spine, 1.5 mm short-stroke rule; tested on
  synthetic strokes only.

## In progress

- **The panel's page count does not fit four-digit pages** (b3-padd2 agent, 2026-11-03, found by the fit check):
  "1995 / 2000" needs 162 px and the toolbar's page chip has 112, so it is cut short in a notebook of 1000 pages
  or more (z_pages_add_6_toolbar_limit stays on page 1 for that reason). A smaller size for long counts, or the
  current page alone, would fit.
- **Pages: the count pad's - and + do not repeat when held** (b3-padd agent, 2026-11-03; the brief made it
  optional): one step per tap, and Up and Down on a keyboard repeat only as the keyboard sends repeats
  (Grimoire ignores key repeats).
- **Pages: drag shows the gap bar only, never makes room** (b2-pv agent, 2026-11-02): the tiles do not
  shift aside while a page is dragged (each shift would redraw the grid on e-ink); the bar between two
  tiles says where it lands. Revisit after trying it on a Scribe.
- **Copy to notebook...: a picture still waiting to be fitted** (lane A agent, 2026-11-02): a page whose
  `bg/<pic>.source.png` has not been fitted yet (a KOReader page of another size, seconds after it came)
  is copied without its picture and draws white there. Copy the source too and queue its fitting in the
  destination if it ever matters.
- **Live view smoothness.** rc.10 replaced the playout (smooth through stalls); still to measure on
  the device with the owner. Ideas left: a 'Wi-Fi is stalling' hint when lateness stays high,
  decode/compose/average in a Worker, 4-bit tiles on a congested link. The intermittent lag measured so far was the Kindle's 2.4 GHz link
  stalling (5 GHz was clean). When it lags: check the band (`iw dev wlan0 link`) and a
  `ping -i 0.1` from the Kindle while it happens; the page's Latency readout shows playout pinned
  at 20 ms during bursts.

## Built, not installed

- **USB file access hardening** (outside this repo, `~/KindleLab/volume`, branch `rc8`, worktree
  `~/KindleLab/volume-rc8`): a Kindle-side controller that survives crashes and logs why it stops,
  never holds the power service awake, the Mac mount behaving like a drive (no `._` files, unmounts
  when the Kindle leaves), and a read-only guard on Grimoire's live files while it runs. Install
  with the owner present: `install_mac.sh`, then `install_usb_daemon.sh` on the Kindle. The earlier
  Mac service (`com.ganten.kindle-volume`) was stopped and disabled on 2026-11-13: it holds the one MTP
  session the Kindle allows and retries every few seconds, so OpenMTP connected only when it won the race
  (`manage.py start` turns it back on).

## To investigate

- **z_pages_find_over flakes on GitHub's runner** (the v2.1.0-beta.1 release run 42540001325, shard 6; passed on
  rerun, and in the dry run of the same commit): line 50 saw `ui 20 1312 1820 1148, ui 0 0 1860 2480` where it
  wants the first alone, an extra whole-screen Ui present on a slower host. It flaked under Wine's load too
  (2026-11-07). Find what sends the whole-screen present (a tick racing the Find sheet's step) or let the
  scenario wait for it with `waitfor`.

- **Parts of the sleep picture left on the screen after a wake** (the owner's Scribe, 2026-11-05: the KOReader
  wallpaper over parts of the panel until a redraw, while the framebuffer, read over ssh, held Grimoire's screen
  whole). The owner has since seen it in KOReader alone, with Grimoire not running, so it may not be Grimoire's.
  A full refresh 2.5 s after every wake (81ec634) hid it; the owner took it out of 2.1 (2026-11-08: "I don't
  think this is grimoire, so i want it gone until I know"), so a wake is as in 2.0.0 again. Leads: the Kindle's
  screensaver module stays loaded while KOReader runs without stopping the framework (lipc blanket `load`), and
  KOReader unloads it only with STOP_FRAMEWORK=yes; the Terminess patch to KOReader is the other change of that
  week. Evidence: ~/KindleLab/evidence/wake-wallpaper-2026-11-05/ (private). Next: the owner reboots the Scribe
  and tries KOReader alone with and without the Terminess patch.

- **Two checks fail on a Linux host** (2026-11-03, on a Void Linux x86_64 machine; both the same at
  5d3e6fe; they matter before `GRIMOIRE_CI_LINUX` is turned on): `tests/scenarios/y_status_corner.qs` line 82 sees a second present at 632,2250 595 x 110
  (a toast's place) after the corner is tapped back on; `rust/tests/test_templates.rs`'s
  `card_drawn_again_with_its_picture` finds no template job started (`template_install` finds the picture
  file already there, so the card is never drawn again). On the Mac both pass. The Kindle is Linux too:
  whether either is a Kindle bug or a host difference is not known yet.

- **CI on Linux runners** (the CI fan-out, 2026-11-02): build-host, unit, the scenario shards and l10n
  can run on ubuntu-24.04 (repository variable `GRIMOIRE_CI_LINUX=true`), where jobs start sooner and
  far more run at once than on macOS (5 at most on a free plan). Off until a run passes there: the
  headless runner and the test binaries were cross-compiled for Linux (armv7 musl) on the Mac, never run.
  host-checks stays on macOS until share_check.sh's studio picture check has ImageMagick 7 (`magick`)
  there; package stays for the clean-build comparison with the maintainer's Mac.
- **y_all_screens.qs is the longest scenario** (77 s here, about 112 s on the CI runner): it is a shard's
  critical path. Split it by screen group (as z_shape_writing was) if shards get shorter than it.
- **The package job installs cargo-zigbuild from source each run** (about two minutes): a cached build or a
  release binary checked by its sha256 would save that; actions/cache was not added (a new action to pin).
- **make check's host checks still run one after another** after the scenarios (share server, Live view,
  Bluetooth, the late-timer scenarios): they share one port; giving each its own would let them run beside
  l10n-check.

- **Issue #11's refreshes on the Scribe** (claude/b2-refresh, 2026-11-02, no device access then). Check on
  the device: (1) a toolbar drag's release flash (a Clean: GC16 with flash over the trail alone, a mode the
  display had not sent before) flashes only that strip and leaves no copy of the toolbar; (2) a word erased
  with the stroke eraser leaves no trace a page turn would wipe, or, if it does, how many words before it
  shows (the threshold is a quarter of the screen, `DU_WEAR_QUARTERS` in display.rs); (3) the flash after a
  quarter's erasing waits for the pen to leave and never meets the next stroke. The diagnosis assumes the
  driver's GL16 partial update drives only pixels whose value changed (as on i.MX EPDC); if the Scribe's
  hwtcon drives every pixel of a GL16 region, the erase traces are something else. The log says
  `display: cleaning fast refreshes' traces` at each clean.
- **Pinch and pen-button zoom leave DU traces too** (claude/b2-refresh, 2026-11-02, not counted): their
  frames redraw the whole screen by DU and end with a grayscale refresh ("settle without a flash", an
  earlier ruling), so they wear the screen as erasing does. Left out of the wear count so zooming never
  flashes; owner to decide whether a zoom should add to it.

- **The Bluetooth helper's command line on the Kindle** (found 2026-11-02, the beta.2 release-fix lane;
  no device access then). From its build files, kindle-hid-passthrough 3.18 is a wrapper that execs
  `dist/ld-linux-armhf.so.3 --library-path dist dist/main.bin --daemon` from its folder, so
  `pgrep -f kindle-hid-passthrough` cannot find it and bluetooth.sh now finds Grimoire's copy by its
  folder. Confirm on the device with the helper running: `tr '\0' ' ' < /proc/<pid>/cmdline`.
- **Other scenarios timed by the real time between their lines.** `GRIMOIRE_SIM_LATE_MS` shows them
  (`make scenarios SCEN_ENV=GRIMOIRE_SIM_LATE_MS=60`); make check runs only the two that failed on the
  release runner that way (`LATE_SCENARIOS`). v_gestures_4e_single failed once there too (2.0.0-beta.0).

- **The stroke eraser on very dense pages.** Taking a stroke out of the middle still draws what was
  under it again (undo and redo of it are instant since rc.10). On a 1,090-stroke page that can take
  seconds a sweep.
- **Brushes on the Kindle.** A dense page still takes about 30 s to draw the first time (marker 23 s,
  charcoal 10 s of it). Exact speedups left: a per-dab table for core pixels (no sqrt, no powf), the
  graphite path's per-pixel divisions.
- **Times past 2^32 ms (49.7 days of uptime).** A touch gesture crossing the wrap of the frame clock
  fails (found by a unit test); the 2^31 case is fixed in rc.10.
- **The KOReader plugin as a book (owner idea, 2026-10-31).** Installed as a file KOReader's library
  shows, removed by long-press > Delete like any book, instead of a plugin folder. Needs research on
  KOReader's file browser and a precedent before building.

- **Kindle Click (Amazon's Bluetooth page turner, 2026).** Grimoire accepts any input device that
  sends ordinary key codes, so it works if the firmware exposes it as an input device while
  Grimoire runs. Test: pair it in the Kindle's settings, list `/proc/bus/input/devices`, then try
  Settings > Shortcuts > Buttons.

## Deferred from beta.2 lane C (links between notebooks; lane C's decisions, 2026-11-02, for the owner to confirm)

- **A page linked only from other notebooks has no dot in its own Graph.** The brief asked to keep the
  linked-only layout and the map's shape stable, and incoming links change without this notebook being
  edited (another notebook saved), so they never make a page a layout node: its anchor box says
  "-> p. N" instead of drawing a line. Making such pages nodes is a one-line change in graph_build, at the
  cost of the map moving when another notebook changes. Amended by the Graph of all notebooks (graph
  lane, 2026-11-02): in All notebooks such a page has its dot (an incoming link from any notebook makes it
  linked); the item stays open for one notebook's view only, whose graph.ini never depends on other
  notebooks being saved.
- **The C tree has no backlinks** (Rust only, PORTING.md). A notebook it changes is read again at the next check.
- **Linked from lists at most 64 rows** (LB_FROM_MAX) and the cache at most 40000 links between notebooks
  (backlinks_partial says while a notebook is left out; nothing shows it yet). A notebook left out is kept
  as "not read" and taken in by itself once another notebook's links make room.
- **Broken links are display-only**: nothing removes them by itself (a notebook restored from the trash
  makes them work again). A "remove every broken link" action was not built.

## Deferred from beta.2's Graph of all notebooks (proposed by docs/beta2/GRAPH-ALL.md, 2026-11-02; the owner decides)

- **The Graph as a place of its own, beta.2 graph lane's choices (2026-11-03; the owner to confirm).** Built
  from the owner's words of 2026-11-03 (docs/beta2/GRAPH-ALL.md section 10). Choices the lane made: (1) a
  left-out notebook's links are not drawn at all, as links to a notebook that cannot be read are not (the
  page with such a link stays linked only through its other links); (2) in All a left-out notebook's pages
  are not drawn even when it is the open notebook (its own This notebook view, which showed it, went with
  the Notebook chip); (3) the Graph from the Library has no List tab either (it lists the open notebook's links,
  and none is open); (4) the editor's Library button, from a notebook the Graph opened, also returns to the
  Graph (the Library as it was left), and one more Back reaches the cards; (5) the Library's top bar got a
  **Graph** button, left of New, taken out again by the owner the same day ("The graph button if there is
  to be one shouldn't be at the top -- we had good visual soundness with 5 icons. For now just the drag to
  summon on the main menu."): the top bar has its five icons, and `index-map` still works over the Library; (6) a notebook in a left-out
  folder shows a greyed "Left out of the Graph with its folder" row (the folder's menu brings it back); a
  notebook's own setting under a left-out folder is kept for when the folder comes back.
- **"Entirely" left out, for the owner to rule** (graph lane review, 2026-11-03). The owner asked to omit a
  notebook or folder "from the graph view entirely". With the Notebook chip removed, a left-out notebook still
  shows only past the Graph of all notebooks' caps: as boxes at the edge of another notebook's own Graph (its
  anchors), and in the notebooks' browser the Library's Graph opens there (which lists every notebook and
  draws the one picked). Whether either should go is the owner's call.
- **The Library's Graph past the caps** (beta.2 graph lane review, 2026-11-03). Past the Graph of all notebooks' caps the Library's Graph opens the notebooks' browser alone (the lane's
  choice, the reviewer's finding, kept when the Notebook chip was removed; the owner may prefer something
  else, such as the largest notebook, or the first notebooks up to the cap).
- **Left out of the Graph, not done** (graph lane, 2026-11-03): no "Show in the Graph" from inside the Graph
  itself (the setting lives in the Library's menus only); a notebook left out keeps its anchors in other
  notebooks' own Graphs (one notebook's view is unchanged); no starter key for `index-map` in a new
  shortcuts.ini (existing files are never rewritten).
- ~~Orphans in All notebooks.~~ Decided by the owner (2026-11-03): "the view for unlinked pages is beyond
  unnecessary, only linked pages should be shown, and from all notebooks". The orphan build (9a1bfef) was
  taken out again; graph-all.ini's lines from it are dropped when read. Not to come back.
- ~~The Notebook chip.~~ Decided by the owner (2026-11-03, testing on the Kindle): "you left on the all
  notebooks button btw - just getting to testing. fold it in if you can", then, asked whether to fold its
  choices into View or remove the notebook choice: **"remove it."** The chip, its menu, This notebook and
  Other notebook... are gone; the Graph is always every notebook's (less those left out), View is as it was,
  "Reading notebooks" is on the panel's counts line. One notebook's view stays only as the fallback past the
  caps (docs/beta2/GRAPH-ALL.md section 11). Not to come back; remembering a pick is moot.
- **Renaming another notebook's page from the Graph** (writing its nav.ini): Rename is off there.
- **Back reopening the Graph after following a node**: built for the Graph opened from the Library (owner,
  2026-11-03: the Graph as its own place); the editor's own Graph still does not (Back there returns to the
  page the node was followed from), nor do anchors.
- **Filters** (a folder, a search; "find a page in the graph" is below). (A Notebook filter was built and
  removed by the owner's ruling of 2026-11-03.)
- **Folding backlinks.ini into graph-index.ini** (one scan instead of two) once both have shipped.
- **Building the first-ever index in a bgjob child** instead of idle ticks, if the device shows the idle
  fill too slow for big libraries (the log says what a check cost: "libindex: N of M notebooks read again").
- **The library index waits for the backlinks check** (graph lane, 2026-11-02): one cache's work a tick, so
  after a notebook opens the Graph's index is checked once backlinks.ini's check is through. Fine on the
  host; to watch on a big library on the Kindle.
- **Pathological chains lay out long** (graph lane, 2026-11-02): a notebook whose pages link only in a long
  chain (page 1 to 2 to 3...) is laid out as a long thin line in either Graph (the seed's rings; measured on
  the host, a 200-page chain's edges are 4 to 15 rest lengths). Real notebooks were not seen to do it; a
  chain-aware seed is possible.

## Not taken from the RC8 review (revisit when there is room)

- Live view: decode, compose and average in a Worker with OffscreenCanvas; optional 4-bit gray
  tiles when the link is congested.
- Graph: find a page in the graph and Around history. (Other notebooks as anchors: built in beta.2, lane C.)
- Remove the retired status-bar code (`status_reserved()` is always 0) and the graph's satellite
  and tray code unused since RC5 (about 250 lines in mapview.rs).
- A tree-wide rustfmt pass over leftovers of an earlier mechanical reformat.
- Wording left as is: "Somewhere Else" (the only Title Case label), and the RC1 changelog section
  that later RCs reversed.

## Ideas only (from a survey of other Scribe note apps, 2026-11-03; the owner decides, nothing scheduled)

Ideas, not plans: each needs the owner's go and its own design. No code is taken from anywhere.
- **Editable PDF export:** a book page's or a notebook page's ink as real PDF annotations, so other readers can edit it.
- **A restorable trash with a timer:** deleted notebooks and pages kept for 30 days, with Restore.
- **Search over page names** (and, much later, over recognised handwriting).
- **Resize and rotate in the lasso:** the natural next step after beta.2's move, copy and paste.
- **Typed text boxes** on a page, with the keyboard.
- **In-app guide cards** with a "Show me" that walks through a gesture on the screen.
- **A "Test pen and touch" screen** that bug reports can attach (pressure, tilt, offsets, touch slots).
- **An issue form** that asks for the device model, firmware and Grimoire build.
- **More languages from community translations:** the gettext plumbing is there (docs/TRANSLATING.md).

## Product

- Send the fork outreach note (draft on the owner's Desktop) and keep the README's "Name, logo and
  forks" section current.
- Optional pay-what-you-want supporter path ($0 allowed); not set up.
- Promote a quiet 2.0.0 release candidate to 2.0.0 stable (the stable KPM feed updates every 1.2.0
  install).

## Document links (lane L4, 2.1.0-beta.2): what waits

- **Hand-made links keep 2.0's tap rule** (growth first, smallest box); widening exact-area-first to them waits
  for the owner (DESIGN 3.10).
- **Edit a document link.** Their card is read only (Rename the page): Change where it goes, Redraw and Delete
  are not offered for them. A link the user wants gone is not removable in 2.1.
- **Inside the bar's document chips, X goes back to the page's own rows** (a second X closes); a drag down from
  the grip closes at once. Chosen because the bar has no room for a back chip beside the header's own; to
  confirm on the Scribe.
- **KOReader quad links.** `getPageLinks` gives rectangles only, so a KOReader-sent link has no outline; the
  PDF route (lane L6) can pass quads through `DocRecord.quad`.
- **The notebook PDF writes a document link at its exact area** (a user link at its 7 mm-padded one).
- **Paste's seeds:** a pasted stroke or link avoids the seeds of the page's document links (select.rs
  `seed_holders`); the stroke's own seed is not checked against document links when a *fill* picks the
  document link's seed (only the user's links are; ink seeds are 32-bit hashes, a clash would be harmless
  since a document link has an area and never takes an ink box).

From the whole-release review (the fix round, lane F3). Fixed there: M16, M21 (the conservative reading) and the Low
items of Document links. Left:

- **No test pins the Links bar's dropped selection** on opening "From the PDF" (linkbar.rs): the fix is one line
  and needs a page with both a link of its own and tabs to pin in a scenario.
- **Older builds and the plugin's `links=1`.** `links=1` outlives a book page an older build added: those pages
  have no tabs until pages are added again (the plugin then asks for every held page's links) or `links=1` is
  taken out of `source.ini` by hand. A fix that cannot be wrong needs `links=` to carry the held pages' count, a
  format change the owner has not asked for.

## Notebook PDF cache and child caps (lane L9, 2.1.0-beta.2): what waits

- **The mirror's 128 MiB margin on a very dense page** (64 MiB before the fix round, which a Scribe-sized colour
  page did not fit: H3). A colour page on a near-incompressible photo-like picture measured 51 MiB, 69 when the
  next page shares its picture (`test_pdf_colour_memory.rs`, on the Mac); a page with millions of points adds its
  strokes. If a real notebook's mirror fails with the cap on (the log says `export of ... failed` and the child
  died of a failed allocation), raise `child_margin(BgKind::MirrorPdf)` in `childcaps.rs`. Nothing measured on the
  device yet (TESTING L9 check 7, and the fix round's F2 check 1); the CI job `sandbox` runs the colour page under
  the real cap first.
- **The colour page's decode is stb_image's** (fix round, H3): a 13 MB PNG is held whole while stb gathers its
  compressed data and inflates it, about 50 MiB at its peak; decoding it from the file through stb's callbacks
  would save the file's own copy. Pages on one picture keep a second RGB copy for the next page (decoding the
  picture again for each page instead would cost seconds a page on the Kindle).
- **One sweep is a guess** (fix round, M14): a mirror's temporary file is swept when its process is gone
  (`kill(pid, 0)`); a pid reused by another process keeps it until that one ends too. The mirror lock waits 15
  minutes for another mirror of the notebook, then goes on beside it.
- **The cache is read per page even with no change.** A hit reads the page file only when the notebook has a link
  on that page; a planner with a document link on every page still reads and parses every page (no drawing). A
  per-page "links stamp" could skip that too.
- **The share server never writes the cache** (the plan: background jobs only), so a download of a notebook whose
  mirror is off, or not yet made, draws every page; the page progress shows it. Making the mirror job also
  prepare the cache when the mirror is off would fix it.
- **A zip entry is stored, not compressed** (stored deflate blocks, since the PDF's images are compressed
  already); the raw `.qpg`/PNG entries are as before.
- **Caps are Linux only** (the host build and the simulator apply none); `RLIMIT_AS` for the share server's render
  children is a flat 128 MiB over their start: a zip of a notebook with a very big raw page may need more. A child
  whose `/proc/self/statm` does not read is capped at 1 GiB plus its margin (`CHILD_VM_UNREAD`), a ceiling above
  any UI size at a fork rather than a measured one; Live view's viewer children get 64 MiB.
- **Not done in the fix round (F2):** the UI's own foreground export when a fork fails (bgjob: "exporting in the
  foreground") runs uncapped and may now wait on another mirror's lock; the cache's records are still read by
  path, not through `fsview` (the security notes); a share download made twice (no room for a spool) fails when
  the notebook changed between the makings rather than serving the second making.

## Done

- After 2.1.0-beta.2 (unreleased, 56-live-pen-stall-flake; issue #56, the pen-stall check's p90 of 62.8 ms on the
  dry run of 2026-11-12, run 43244209271's first attempt): `tests/live_pen_stall.py` measures the pen viewer's own
  reader once more when its first run misses a check, and the second run decides; the 25 ms bound stays. A miss
  prints how far the feeder itself fell behind its 4 ms frames, the machine's share of the wait. In this test the
  viewer waits for the main loop only where it is meant to (its one pen-down, held while the loop handles an
  event: `LIVE_RAW_HOLD`), so samples that wait for the main loop wait in every run: with every run's samples
  taken from the ring the check still fails (p90 118 ms, both runs). The failing attempt's runner was slow
  throughout (the ring's p50 124 ms, against 74 to 95 ms on the Mac). On the Mac at background priority
  (`taskpolicy -b`, which keeps every process of the test off the CPU at times, the app's among them) the feeder
  fell up to 1.4 s behind and the raw p90 reached 178 to 841 ms in three runs of four.
- 2.1.0-beta.2 (unreleased, v21b2/l10-fonts; the owner, 2026-11-04: "Custom font support/More fonts in the classic
  theme", and 2026-11-09: "Fast track everything up till beta.3 into a beta.2."): your own fonts. `.ttf` and
  `.otf` files copied into `grimoire/fonts` are checked by the sandboxed worker (skrifa reads them; every glyph
  is outlined under the caps and drawn at the smallest and largest size) and listed in Settings > Font under
  "Your fonts", each by its name drawn in itself or by why it is refused; a font that passed draws Classic's
  text through stb's own rasterizer, with Terminess for what it lacks; bold by its weight axis, a Bold file or
  overprint; `font=file:...` and `font_bold=` in settings.ini; the crash guard in `grimoire/.font-inuse`; the
  glyph cache capped at 4 MB. Choices and what waits: Your own fonts (2.1.0-beta.2, lane L10), above.
- 2.1.0 (unreleased, v21/fonts; the owner, 2026-11-04: "Custom font support/More fonts in the classic theme",
  and 2026-11-07: "can we fast track anything that might be cool alongside this that was planned for 2.1 or
  future?"): More fonts in Classic. Settings' Style row has a Font chip in Classic; the Font sheet offers
  Terminess, Atkinson Hyperlegible Next, Noto Sans and Literata with a live sample, applied on Done; `font=` in
  settings.ini (an unknown value kept); the share pages follow; every screen fits in each in English and
  Spanish (make l10n-check's `classic-<font>` styles, tests/l10n/fonts.qs); the size guard in package-check
  (D23). Choices and what waits: Classic fonts (2.1.0), above.
- 2.0.0 (the owner put it into 2.0.0; v21/lasso; the owner, 2026-11-05: "the copy paste lasso function is horrifically slow on
  drag with anything containing colored in or multiple strokes"): a drag of the selection follows the pen
  whatever it holds. A pen sample only records the newest place; the editor's tick shows it once the last DU is
  off the panel and 120 ms went by, or the pen paused 50 ms. The first move takes the bar away and keeps a copy
  of the screen without the outline; each show puts the old outline's strips back from it and draws the new
  ones, each strip a DU of its own, with no compose and no halo or ink drawn. On his page in the headless runner
  (a scratch copy, never in the repository): 0.13 us a sample instead of 1.77 ms, 16 shows of about 13,000 px
  for a 400-sample drag instead of 403 refreshes of 613,000 px; on the device that is about 2 ms a show at most
  about 5 times a second, against about 22 ms a sample before. The lift is unchanged (the screen after it is
  pixel for pixel the old build's). The box moves once the pen has gone 1 mm (a tap that wobbles a pixel is a
  tap), and a sleep in the middle of a drag cancels it (the review: the bar, away, kept its buttons answering at
  their old place after the wake). tests/scenarios/z_lasso_19_drag_dense.qs, z_lasso_20_drag_live.qs and
  z_lasso_21_drag_sleep.qs on a generated page with his page's statistics; z_refresh_4_select's wear and lift
  rect follow the bar no longer going round with the box (57 thousandths of the screen worn on its drag instead of
  384, so no Clean follows).
- 2.0.0 (unreleased, v21/i16; issue #16, reported by nebuff: "Toolbar disappears in live view when a popup menu
  (settings, display settings) is opened then comes back after closing menu"): the toolbar's Live view layer
  stays where the panel is under every menu, sheet and bar of the notebook, holding what the Kindle shows there
  (the toolbar, and a menu's part over it), and is none only under the Pages view, the Graph and the notebooks'
  browser, which cover the screen, and while Settings is over the notebook, whose screen, the toolbar's strip
  beside it included, the frame holds whole (docs/DESIGN.md, Live view). In the headless runner the layer went as the
  Display sheet or the Paper sheet opened, a setting changed or not (76,089 pixels of the toolbar's rect wrong in
  the browser beside the Display sheet); Settings over the notebook, and its own Display sheet, were already
  right, the frame then holding the whole screen. tests/scenarios/y_live_toolbar_menus.qs checks it with two new
  probes, `live.panel` and `live.stale.X.Y.W.H` (docs/SIM.md).
- 2.0.0-beta.2 (unreleased, b2g/del; the owner, 2026-11-05: "hold del long enough in keyboards to quick delete",
  "low ish delay but not too low", "repeat rate high so it feels snappy", "not too high"): the on-screen keyboard's
  Backspace auto-repeat, from the RC8 review's list. Delete held repeats on every keyboard after 400 ms, every
  65 ms, never queueing a refresh behind the panel (docs/DESIGN.md, Text and the keyboard). Left as it is: a
  hardware keyboard's held Backspace on a count pad deletes one digit, as `hidkeys_event` drops the kernel's
  autorepeat for every key (text fields take no typed keys at all, only Esc; 2.1.0-beta.2 adds the arrows, Home,
  End and Ctrl or Cmd with A, C, X and V, so a held arrow moves the caret one letter, see below); repeating it
  would let autorepeat into the path every shortcut shares, for a count of a few digits.
- 2.0.0-beta.2 (unreleased, claude/b3-pv3; the owner's rulings of 2026-11-03, verbatim): (a) "The graph button if
  there is to be one shouldn't be at the top -- we had good visual soundness with 5 icons. For now just the drag
  to summon on the main menu." The Library's top bar is back to its five icons (Exit, Settings, Share, Folder,
  New); two fingers down over the Library still open the Graph, and so does the key bound to `index-map`.
  (b) "Make the pages undo mapped to the same undo flag that the controller sees for it in pages, just keep them
  seperate to each other. That solves it right? And add the redo button." While the Pages view shows its grid,
  the `undo` and `redo` actions (keys, controller buttons, gestures) are its Undo and Redo of the last page move
  or batch of new pages; on the page they stay the page's own. The undo action works with pages ticked too (the
  bottom row only hides its Undo button then, for Move to...'s place). Redo is one step: an Undo makes it, a new
  move or batch drops it, and it applies only while the pages are the list the Undo left. The bottom row shows
  Redo beside Undo's place, in Copy to notebook...'s (disabled with nothing ticked anyway), with nothing ticked.
  A batch's Redo writes the same pages again with the same ids at the same places (each file the blank page the
  batch wrote: the Undo only took them back while they were still that; an image paper listed again in
  bg/pages.ini); a file in the way that is not that blank page refuses it. (c) "Also no need to have the page
  outline show when dragging, just show the black bar." A dragged tile shows only the gap's black bar: the same
  gap refreshes nothing, a new gap refreshes the bar's old and new places, each on its own fast refresh. (d)
  "Batch added pages should have whichever page layout the SELECTED page has - and fall back to the open page if
  none are selected." A batch takes the paper of the page it goes after: in the Pages view the last page ticked
  (with several ticked, the last in page order, which is where the batch goes), else the open page; the
  toolbar's count pad, which goes after the open page, its paper. A tap on New page is a batch of one through
  the same code, so it follows the same rule. (A document picture's paper still falls back to the notebook's,
  store_new_page_paper.) The paper is read from the page file's first 20 bytes (store_page_paper), not its strokes.

- 2.0.0-beta.2 (unreleased, claude/b3-padd2; owner 2026-11-03, "make the page add button not enter the page when
  its made -- and add the long press dialog to the toolbars page adder -- two birds one stone. swiping still works
  for new pages and nav so."). The b3-padd2 agent's reading: "the page add button" is the Pages view's + (New
  page), the button whose tap entered the page it made; "the toolbars page adder" is the toolbar's next-page
  arrow, which turns into a + on the last page; and "swiping still works for new pages and nav" says the editor's
  way of adding and going to a page stays as it is. So: a tap on the Pages view's + adds one page as a batch of
  one (its toast, "Added 1 page: it is page N.", its Undo in the bottom row) and the view stays, on the new
  page's screenful, its tile labelled "Page N (new)" while Undo would take it back (drawn with the grid: no
  refresh of its own). The toolbar's + held 600 ms (the Pages view's hold), or tapped with Shift, Ctrl, Alt or
  Cmd held on a keyboard, opens the same count pad along the bottom over the page; Done adds the pages right
  after the open page (on the last page, the only place the + shows: the end, where its tap adds one), the
  editor stays on its page, and a toast says how many, from which page, and "Undo them in Pages": the batch is
  the notebook's Undo record (pv.add_undo), offered in the Pages view's bottom row under the batch's rules (the
  editor's Undo is the page's own history). Cancel, Esc, Back, Close menu or a tap outside closes the pad with
  nothing added; a full notebook says so instead of opening it; a contact made while a batch is written is let
  go to its lift, as in the Pages view. The toolbar's plain tap still adds a page and goes to it: it is the same
  code as the next-page key and the two-finger swipe (panel_act P_NEXT), which the owner said keep working, so it
  was left alone; owner to say if the tap should stay on the page as well. The `new-pages` action (86, no row in
  the Buttons sheet, which is full: bind it in shortcuts.ini, e.g. `key.n=new-pages`) opens the toolbar's pad on
  any page, the pages going after the open one. The `new-page` action still adds one page and opens it. A
  notebook full of pages: New page tapped in the Pages view says the notebook is full (it said "Could not add a
  page"). Not done: no default key for `new-pages` (Ctrl+N stays `new-page`).

- 2.0.0-beta.2 (unreleased, claude/b3-padd; owner 2026-11-03, "make the page adder an instant addition with
  a modifier which prompts a text field for a number of additions to add. a number pad will work fine here,
  and a -/+ at the bottom"): + in the Pages view adds one
  page at once (as before); held 600 ms (a held tile's time), or tapped with Shift, Ctrl, Alt or Cmd held
  on a keyboard, it opens a count pad: the number pad with a - and + row and an "Add N pages" Done (off at 0), at most what
  the notebook still holds (2000 in all; a toast when the count meets it). A hardware keyboard types in it
  (digits, Backspace, - + and the keypad's, Up and Down, Enter, Esc). One store operation (one notebook.ini
  write; a failure adds none), on New page's paper and at New page's place; annotated notebooks take them as
  New page's note pages. The b3-padd agent chose the Undo: yes, as Move's (the bottom row's Undo, in Move
  to...'s place) while the pages are still blank, none was saved or named or linked since and the open page
  is not one of them; otherwise not offered. After review (b3-padd agent, 2026-11-03): from 25 pages the pad
  goes and "Adding N pages..." shows before the files are written (one refresh; 15 to 25 ms a page on a
  Kindle, so up to about a minute), and a contact made while they were written is let go down to its lift
  (pv.quiet_until), so no queued tap lands on the grid drawn after. No soft cap below the notebook's 2000:
  the toast says it is working. Undo is now not offered either once a page of the batch is named or linked
  (pagesview_add_in_use, the check Undo itself makes), so it never shows only to refuse. A batch on a picture
  paper whose notebook.ini fails unlists its pages from bg/pages.ini in one write, not one a page. The b3-padd
  agent read "a -/+ at the bottom" as under the digits, with the full-width Done below them as on every number
  pad; owner to confirm, or say if - and + should sit under Done.

- 2.0.0-beta.2 (unreleased, claude/b2-pv; owner rulings 2026-11-02, "Move to can be an undo step, yes. Yes
  drag to reorder after holding."): a page move is one undo step, offered as Undo in the Pages view's bottom
  row (in Move to...'s place, nothing ticked) until the pages change again or the notebook closes; the
  editor's Undo button stays per page. Drag a held tile to a gap (deferred by lane A, now done): an outline
  and a gap bar on fast refreshes (the outline taken out in claude/b3-pv3: the bar alone), the screen turns over the top bar or the bottom row, one undo step.

- 2.0.0-beta.2 (unreleased, claude/b2-fhold): **no panel button for Select** (owner ruling 2026-11-02: the
  panel has no room for one). The finger's way in instead: a finger held still on the page off a link for
  the link hold (0.6 s) arms a lasso, shown by a small dashed ring; drawn on without lifting it selects as
  the pen's lasso does, with the same bar; lifted without moving it pastes the clipboard there (nothing when
  it is empty). A hold on a link still opens its card. Guards (no surprises): a palm (TOUCH_F_BROAD, or
  touchgest's palm and rest rules, which end the finger with a cancel) never arms, nor does a hold with
  the pen in range at any point from the down (or gone under 500 ms before it); pen activity after the
  arm drops the lasso, its ring cleared, the lift ignored. No switch: no Settings page lists finger behaviours
  of this kind (the Gestures sheet binds actions to gestures). Select also stays in the tool picker and the
  bindable `select` action.

- 2.0.0-beta.2 (unreleased, claude/b2-refresh): issue #11. A toolbar drag's release cleans all of its trail
  with one Clean refresh (a flash of that rect alone, a new display mode), and draws again every place it went
  (a drag that went round left the page between its ends dithered); a Select drag's end does the same in
  grays. DU wear (display.rs DuWear): the stroke eraser, the precise eraser and white ink, and a dragged
  selection's bar and outline count what fast refreshes turned back to paper; past a quarter of the screen,
  at the next pause, one Clean of where it was. Test probes `display.stale.X.Y.W.H` and `editor.wear`.

- 2.0.0-beta.2 Graph as its own place (owner, 2026-11-03; graph lane, docs/beta2/GRAPH-ALL.md section 10):
  orphans taken out of the Graph of all notebooks again (the owner: "beyond unnecessary"; graph-all.ini's
  orphan lines dropped when read); Leave out of the Graph for a notebook (notebook.ini graph=0) and a folder
  (root/graph-omit.ini, nested folders and notebooks moved in included); the Graph opened from the Library
  (two fingers down, the index-map key; the top bar Graph button it also had was taken out again, the
  owner's ruling of 2026-11-03: "The graph button if there is to be one shouldn't be at the top -- we had good
  visual soundness with 5 icons. For now just the drag to summon on the main menu.") with no notebook open, no List or Pages tab
  and no This notebook, a node opening its notebook and Back returning to the Graph.
- 2.0.0-beta.2 the Notebook chip removed (owner, 2026-11-03: "remove it"; docs/beta2/GRAPH-ALL.md section 11):
  the Graph is always every notebook's, no This notebook or Other notebook..., "Reading notebooks" on the
  panel's counts line; one notebook's view only past the caps.

- 2.0.0-beta.2 Graph of all notebooks (owner ruling 2026-11-02; claude/b2-graph, docs/beta2/GRAPH-ALL.md):
  the Graph opens on every linked page of every notebook, islands per notebook with captions, a Notebook
  chip for this notebook or another, root/graph-index.ini (every notebook's pages, names and links, a cache
  kept on idle ticks) and root/graph-all.ini (the layout by notebook and page id), a new bgjob kind for big
  layouts, new notebooks placed as blocks beside the kept layout.

- 2.0.0-beta.2 (unreleased, claude/b2-relfix): the release runner's two scenario failures fixed (touch
  lines stamped by the script's clock, frames without waits, `make scenarios-late`); bluetooth.sh stops
  and takes back the helper in Grimoire's folder by its folder, recorded or not, with its cursor overlay.
- 2.0.0-beta.2 lane C (links between notebooks, polish; on claude/b2-links, not yet merged): Linked from
  rows in the Links bar and a count on the Pages card, from root/backlinks.ini (a cache); Graph anchors for
  other notebooks' pages, both ways; broken links (page or notebook deleted) dashed, "Page gone" /
  "Notebook gone", one tap to their card (Change where it goes, Delete link); Back across notebooks keeps
  the page, zoom and offset and that notebook's back list.

- 2.0.0-rc.12: folders movable; keyboard apostrophe and symbols; Pages Rename/Delete in the held page's menu and flicks instead of arrows; Graph pen tap selects; Smallest shape is the real floor; ellipses and parallelograms snap; Corner roundness; dithered link outlines; stroke eraser tiles; lost-lift recovery and input diagnostics; ink picker; clock medium; Live view Brightness.
- 2.0.0-rc.11: What's new under the bat (look C, picked by the owner): the owner's note (`docs/WHATSNEW.md`)
  and hand signature with the bat (`assets/signature.pgm`, from the notebook "2.0b1 sig"); opens once after an update.
- 2.0.0-rc.10: instant undo and redo on any page; dense pages open from their kept drawing; the
  heartbeat goes on through long drawing; no background redraw of the open page; faster exact
  brushes on the Kindle; Live view playout smooth through Wi-Fi stalls; gestures past 24.9 days of
  uptime.

- 2.0.0-rc.9: one-finger page turns; pen taps in the Graph; papers in three rows with cross grid,
  isometric, hexagon and Seyes; Custom music 8-line tab, presets and new default; Smallest shape
  slider; Live view picture in picture for Firefox; the toolbar kept in Live view beside the tool
  picker.
- 2.0.0-rc.8: taps no longer swallowed near the pen off the page; sliders keep their value on a
  cancelled touch; Live view rail, size control and picture in picture tiers; steadier, faster
  graph; Pages and Graph untangled.

## Completed for 2.1.0: the Links bar learning check (2026-11-07)

- `z_links_3_bar_learns` waits for the incoming rows themselves, with a 60-second failure bound, instead
  of a fixed 9-second sleep. The slow-runner CI failure is pinned by `GRIMOIRE_SIM_BACKLINKS_STEP_MS=60`
  on the scenario (each tick of the backlinks check 60 ms longer: the old fixed wait fails every run). The
  400-ms-late run covers the waits around the learning, not a slow check.
- The headless runner's new `waitfor KEY OP VALUE` shares expect's probes and operators, idles as `wait`
  does (30 ms a round; app time moves on under `GRIMOIRE_SIM_CLOCK=script`), stops as soon as ready, and
  fails immediately for malformed or unknown probes and for the keys a check starts again. A `touch`
  contact held across it is stamped by the clock again after it, as across `waitbg` or `hold` (docs/SIM.md's
  touch row names all three).

## Deferred: waitfor (2026-11-07)

- `waitfor` ends as soon as its probe matches, also under `GRIMOIRE_SIM_LATE_MS` (a wait ends late there);
  its 60 s bound is real time, also under `GRIMOIRE_SIM_CLOCK=script` (app time then runs no faster than
  real time, so a timer close to 60 s of app time can outlast the bound). No scenario needs otherwise yet.
- The keys a check starts again could be read without starting them again (a peek for each of the five)
  instead of being refused, if a scenario ever needs to wait for a count.
- A `touch` contact held across a `waitfor` could be stamped by the script's own clock moved on by what
  the `waitfor` idled (as a `wait` moves it by its MS) instead of by the clock after it, so that a press
  timed across one is not stretched by a busy machine. No scenario holds a touch across a `waitfor` yet.

## Completed for 2.1.0: dense-page revisits (2026-11-07)

- A page written from blank keeps its whole drawing once its file reaches 512 KB, at idle save or when
  left. First revisits no longer need the slow opening that used to create the first kept drawing.
- Closing after changes on another page retains the Library card when page 1 stayed the same. A changed
  or reordered first page uses a current kept drawing, then its current preview, then a stroke render.
- Idle previews write at most every 30 seconds, including across reopens, and only when stale; the kept
  drawing of a dense page is made when the page is left or the notebook closed, and at an idle save only once
  the page has been quiet for 30 seconds (written in bursts, it was 3.4 MB at every pause); Ctrl+S, the Pages
  view, Settings, the power press and a sleep write the page file and no cache. Generated regression ink has
  STRESS TEST 3's 698 strokes, 163,950 points and a 6,648-point stroke; no owner's page is committed.
- PDF mirror reuse (brief item 1.4) is not done, and not for a scale reason: a background job is a fork of
  the panel's process and inherits the panel's Seyes factor (bgjob.rs, paper.rs seyes_scale), so the export
  draws at the editor's scale. What is left is to find the export's pages that a current kept drawing can stand
  for (same size, same layers) and read them back instead of drawing their strokes (above: 51 s at each close).

## The file picker (2.1.0-beta.2, lane L8): what waits

- **A pick that waits**: fixed in the fix round (H2). The `waiting` mark is queued by the first scan that takes the
  lock afresh, and the Library says so when a pick has to wait ("Another import is still going. This one starts
  after it.").
- **The Loading notice after Back during a slow listing** is cleared once any folder is read (the fix round, M18):
  checked by reading only; a test needs a listing slower than a second in the headless runner.
- **The picker's UI-thread work** (the fix round, Low): entries of no known kind are stat-ed in 20 ms slices before
  the rows are built; the Import place's folder rows are listed every 5 s, the PDF rows as the import's progress
  changes; "has notes" reads the notebooks' keys once a session (one small read for each notebook without
  `source=`, at the first PDF row, on the UI thread). None of it is timed on the Scribe (TESTING.md, L8 check 2).
- **Folder rows count their pictures by listing the folder** (on the idle tick, one folder a tick, at most 200 ms
  each) to show "%d pictures" and to offer Import %d pictures as notebook. On a big `documents/` of big folders over
  FUSE that is unmeasured: TESTING.md has the timing check. A cheaper rule (count only when the card opens) is
  one line in `Browser::work` if the Scribe says so.
- **Pictures as files, fonts, the path bar, Go to, Sort, Show, View, Covers, item cards on every file, Rename,
  Trash, Set as home, previews and "Making previews..."** are 2.2 (L11), as the plan has them; `PickKind` and
  `PickSpec` are ready for Picture and Font.
- **A PDF being imported still offers Import as notebook** on its card in the Import place; a second tap while it
  runs is `AlreadyRunning` and says nothing (its row says Waiting or Importing).
- **A picked folder's import has no progress** (the picture import never had: its row says Waiting until the
  notebook appears).
- **Names that are not UTF-8** are left out of the picker and counted in the log line ("N names left out"):
  the job queue and the screens take paths as text.
- **Not run anywhere but the headless runner**: `rust/tests/test_browser.rs` plants the odd name where the file
  system takes one (not on macOS); the Scribe's checks are in TESTING.md.

## Hardening round (2026-11), storage reads: what waits

- The other reads by path take the plain-file read (`util::open_read_nofollow`): the clipboard, the
  compaction's list, the shortcut files, one read in the fonts' cache, the PDF cache's records.
- The folders a scan lists (the import folders, the marks) are read through the storage walk, as picked
  folders are.
- A page file that is there but does not read fails to open rather than opening blank; with it, a size
  limit on page files.
- `mkdir_p` stays as it is (an existing entry counts as the folder): the data root, a platform's folders
  and the tests' folders may sit behind a link.
- Tidy-up: the PDF cache's and the share spool's own "made afresh" onto `util::create_fresh_nofollow`;
  `nav.rs` `read_sidecar` onto `util::open_read_nofollow`.
- If no other lane has taken them by the merge: the marks in `pdfimport.rs` read with their own limits,
  and the link tests run as root in CI's sandbox job.
