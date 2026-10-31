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

## 2.0.0-rc.8 (device checks pending)

On the Kindle: right after writing, tap toolbar buttons, Library buttons and sheet rows with a
finger while the pen still hovers; each must answer the first time. Drag the brightness slider and
bring the pen close mid-drag: the value must stay. Type quickly on the naming keyboard. Drag the
toolbar to the very top. Open the Graph, add a link and reopen it: the map keeps its shape. In the
browser, try the Live view rail, the size control and picture in picture in each browser you use.

## 2.0.0-rc.7 (device checks pending)

Automated regressions cover fast lift-off packets and deliberate 1.6/2.4-second swipes, with
pen/palm/pinch and stale tracking-ID guards. They do not measure the device's swipe failure rate.
The link scenario draws circles, ellipses, triangles and uneven quadrilaterals, saves/reopens them,
checks circle-corner exclusion and center navigation, and exercises undo/redo and clear. Sidecar
tests protect malformed/unknown data and round-trip bounded geometry. The brush, stroke-hit and pen-settings fixtures were
regenerated from C for the two changed tool defaults, keeping Rust/C parity.

On the Kindle, try quick and slow two-finger page turns both ways, including after writing.
Show Areas and check fine dotted borders with no number badges. Make links with each smart
shape; confirm that visible curves match their tappable area after reopening and undo/redo.
Change one page's paper, then create a linked New page and check it inherits that layout.
Fresh notebooks should start on Narrow, no margin. Check long Library names and the equal-width
KOReader button in Classic/Gothic and landscape. Saved custom tool preferences should survive;
Reset/fresh fountain is 0.25–1 mm opaque, highlighter 5 mm with 25–75% opacity.

If the optional Terminess patch is enabled, explicitly update it in Settings > KOReader and
restart KOReader. Check its battery at several levels, charging and unplugged, in the dropdown
and reading footer. Automated stubs verify native font selection and ownership-safe upgrades;
they do not prove the physical battery glyph rendering.

## 2.0.0-rc.6 (device checks pending)

The candidate adds persistent/decoded preview reuse, explicit multi-select, notebook-hosted
naming and numeric page navigation, live clock previews, and a single-view Live pop-out.
Automated regressions cover cold-process thumbnail reuse, updating only the edited preview,
lossless compaction without reviving an invalidated cache, and changes within a coarse directory
timestamp. Browser stand-ins check that popping out moves the same canvas and restores it on
close without opening another Live client or making a video. These do not measure Kindle or
Firefox responsiveness.

On the Kindle, open Pages from the toolbar count, restart Grimoire and check the cached previews
return promptly. Edit one page and check its preview changes. Tap selects one page; Multi-select
allows several, with an inner selection border. Check the compact New/Copy/Delete controls,
double-tap to open and long-press to add a link. Swipe up and test Name page and Go to page over
the notebook. In Classic, check the Smart shapes title. In Display, change clock size/corner
without closing the dialog; fresh settings default to a small enabled top-left clock.

In Firefox Nightly, reload Live view after installing the candidate and use Pop out. Compare
ink responsiveness, resize it, try keys and recording, then close it and check the same view
returns to the original tab. Keep that tab open. Check the Kindle reports one watcher. Repeat
sleep/wake with Live connected; the earlier reboot recovery does not prove this candidate's
wake changes, which still need device acceptance.

Update the optional KOReader Grimoire plugin and restart KOReader: its old Wi-Fi startup/manual
controls should be absent. KOReader's own network menu remains responsible for connections.
The USB transfer helper remains development-only.

## 2.0.0-rc.4: Wi-Fi controls (superseded by rc.6)

The KOReader startup/manual controls described here were removed in rc.6. The Grimoire Share
controls remain; rc.4's KOReader-specific instructions are historical.

Automated checks cover helper write order, idempotence, unavailable/invalid properties, rejected
and ignored writes, partial failures, strict status parsing, bounded output, stalled children,
descriptor isolation and queued user actions. KOReader stubs cover deferred startup, one enable
per process, the startup opt-out, existing connection attempts, unsupported backends, manual
On/Off and failures that cannot escape the UI task. Wi-Fi dialogs are exercised in English and
Spanish, both styles, upright and sideways, with and without an address, including Live view.

On the Kindle, test from airplane mode: start KOReader and check it reconnects to a saved network.
Turn Wi-Fi off manually and check opening another book or waking does not turn it back on.
Disable the startup option and check the next full restart leaves the radio alone. Re-enable
it and repeat a Grimoire round trip. In Grimoire Share > Wi-Fi, test On, wait for the actual QR
address, transfer a page, then test Off locally (the SSH connection will drop). Check no saved
network is lost. This candidate does not claim these physical transitions have been tested.

## 2.0.0-rc.5: graph and page management

Check October-2026-02: unlinked pages 13/14 and deleted pages must not appear as graph nodes.
There is no Unlinked tray, zoom-button pair or Tidy up button. The compact footer's View button
cycles All, Around and Fit; pinch zoom remains available. Node counts describe the displayed
local and external nodes, orphans are current-notebook pages without valid links, links count
valid outgoing links, and page totals distinguish this notebook from all notebooks.

Pages shows nine previews per screen in either orientation. Tap toggles selection; a second tap
on the same tile within 450 ms opens it. Hold for 600 ms for Open / Add a link / Cancel. Add a
link opens the held page and arms its ink-free area selector; a failed page save must not arm a
link on another page. Choose-from-pages destination picking remains a single-tap operation.

Toggle Areas with both the current page and a linked-page preview showing; it must update on
that press. The preview must stay a preview, with the origin/history unchanged until committed.
Dismiss it, make/cancel/fail a cross-notebook link, and verify no thumbnail or picker pixels linger.
Unavailable previews must stop background work after one attempt; reopening the browser or
explicit cache invalidation permits another attempt. Main Settings stays over the notebook through nested sheets, rotation, pen input and Done; pen
input must not draw through it. Pen feel is grouped beside drawing tools in the toolbar.

## 2.0.0-rc.3: unified Index and notebook links (device checks pending)

Automated development checks cover stable cross-notebook targets, rename/folder-move identity,
unknown metadata preservation, cache invalidation and corruption, nested destination browsing,
combined link Undo/Redo, reopening and cross-notebook Back, cancellation and failed saves.
No physical acceptance of this rework is claimed.

On RC3, check the four shared Index header buttons, default Graph entry and
opposite-swipe dismissal, graph footer controls, link-only List and notebook group boundaries.
Check the upward page-status panel, Name page, and Settings cog returning to the same notebook.
Create links to notebooks in the same folder, nested folders and the root, including matching page
numbers and notebook names. Cancel at both browser levels; rotate and sleep while selecting.
Check cached previews appear without blocking taps and update after ink, erasing, Clear and changes
to image paper. Verify rename/move/reorder, retarget/Undo/Redo, deleted destinations and preservation
of the origin when storage is full. Test English/Spanish and both styles upright and sideways.

## 2.0.0-rc.2 (device checks pending)

This candidate replaces the notebook status bar with the small top-left clock corner, makes the
toolbar link button add a link, limits pinned tools to three, and gives Smart Shapes its own menu.
The menu controls hold speed, four-sided and round-shape preferences, and ink-free link-area
snapping. Clear page removes outgoing link areas together with ink, including otherwise blank
pages; one Undo/Redo restores or clears both. Other pages' links are unchanged. Pen-button hold is
the only built-in Clear page binding; custom click bindings and multi-click recording remain.

Automated checks cover these changes, failed link saves, and ordinary KOReader startup with saved
rotation 0 while the sensor reads upside down. The startup fix requires the updated Grimoire
KOReader plugin. No physical acceptance of RC2 is claimed. On the Kindle, verify:

1. From the Kindle home screen at 180 degrees, start KOReader and check its orientation and taps.
   Repeat with accelerometer ignore and orientation lock enabled, then check a Grimoire round trip.
2. On a page with ink and a link to The Index, Clear page, change pages and reopen the notebook.
   Its link count should be zero. Repeat on an ink-free linked page; Undo/Redo should affect the
   ink and links together, while another page's links remain intact.
3. Check the clock corner and Display menu, direct toolbar link creation, two-finger Links swipe,
   fourth-pin refusal, Smart Shapes preferences and link-area snapping without ink.
4. Continue the gesture checks below, including hold-to-clear, recording multiple clicks and
   preserving custom pen-button assignments.

## 2.0.0-rc.1 (2026-10-29)

This candidate includes Index phases 2–6, the KOReader orientation fix, nested notebook folders
and the landscape toolbar placement. It is prepared with Rust 1.97.1, Zig 0.16.0 and
cargo-zigbuild 0.23.0. Automated coverage includes the model and allocation-failure tests,
handwriting replay, map jobs and cache races, input recording, controller and pen-button scenarios,
English/Spanish screen-fit checks, the ARM build and release packaging checks.

RC1 at `c3b1eda2cd53` was installed over SSH on October 29, with the package and executable
hashes verified, a validated backup, and unchanged Grimoire user-data hashes. The already installed
KOReader plugin was updated without restarting the active reader. The owner subsequently reported:

- Returning from Grimoire restores KOReader's orientation correctly.
- Starting KOReader from the Kindle home screen while held upside down still starts upright.
- Pen-button hold still clears the page; the default triple-click did not work.
- A link to The Index on page 17 remained after Clear page.

The next candidate fixes startup orientation, retires the triple-click default, and makes Clear
page remove its links with the ink in one undoable operation. Those fixes have automated coverage;
no physical acceptance of the next candidate is claimed. Other device checks remain pending:

1. Update the Grimoire KOReader plugin, leave Grimoire while holding the Kindle upside down,
   check tap alignment, then keep turning the device to confirm subsequent sensor changes.
2. Rotate with Whole page sideways on and the toolbar on either side, then repeat with it off.
3. Create and rename nested folders, move a notebook, and create/edit/undo links between its
   pages. New selection strokes disappear; beta.5 lasso ink remains ordinary ink.
4. Try held shapes, moving on to cancel, recorded finger gestures and pen-button actions on
   actual notebook pages. Check accidental activations, latency, Live view and e-ink refreshes.

The detailed phase-specific checks below remain applicable. The real-hand touch study has not
been supplied; provisional tuning is identified in its notes rather than presented as measured
Kindle success rates. No tag, public release or KPM feed update is part of candidate preparation.

## Index phase 6: pen-button gestures (device checks pending)

Complete Phase 6 owner acceptance remains pending. On RC1 the owner confirmed pen-button hold
still clears, but triple-click did not; the next candidate keeps hold as the sole default. The 350 ms
multi-click gap is intentional: a single click waits 350 ms while double/triple is bound;
triple means each next press within 350 ms of the previous release, not three within 1.2 s.
A hovering hold remains 1200 ms. A click lasts at most 450 ms with no nib contact. Momentary
erasing begins at press without waiting for hold recognition.

Run [the Phase 6 owner checks](KINDLE.md#phase-6-owner-checks-pending-on-a-kindle) on the actual
build: record/reassign/clear/reset, normal nib taps on card buttons, momentary eraser and key
coexistence, bound drag without page/Live ink, real pan/zoom, side-button ink when unbound,
proximity cancellation, second-button capability and the EN/ES fit matrix. Verify no GL16
or toast during contact. Record build/device/firmware/pen and actual pass/fail for each item.

Focused host coverage and its exact commands/results are tracked in
[index-hook/PHASE6-IMPLEMENTATION.md](index-hook/PHASE6-IMPLEMENTATION.md). The main owner owns
Phase 6 scenarios, integration and full gates. Until those run, focused unit passes are not a
passing release gate. Phase 3 tuning and the owner trace remain pending as documented below;
Phase 6 does not turn synthetic fixtures into owner evidence.

## Index phase 3: gestures and finger shapes (device checks pending)

No phase 3 device results or owner trace are available for this work. `TG_TUNE` is provisional;
synthetic fixtures and runner probes do not measure owner success, accidental fires, hover
spacing or e-ink trail appearance. The main owner will run the full gate and record its actual
result in `index-p3-check.log`; no passing full-gate claim is made here.

Run all [phase 3 owner checks](KINDLE.md#phase-3-owner-checks-pending-on-a-kindle): natural two-finger
swipes/taps, pinch after writing and re-grab; zero accidental fires while writing/holding;
near/far pen hover; five Settings labels upright/sideways in both languages/themes; pattern
record/reassign/refuse; two/three-sample shape recording, V/check refusals, unknown-shape fade;
clock-corner tap and shape; hold-clear, Reset, failed-save/Cancel preservation; seven groups and
source restrictions; toggles/cycles including all four sleep-screen choices; trail over Links,
pen interruption, sleep/layout aborts, no ink/undo/export marks. Capture the build, Kindle,
firmware, language/theme and failures before marking any item passed.

Then run [the two-finger study](KINDLE.md#the-two-finger-study), blocks A–L with 5 s hands-off gaps,
and send the trace/log. [TOUCH-STUDY.md](index-hook/TOUCH-STUDY.md) records the pending thresholds,
acceptance targets and shape freeze gate. Owner blocks J/K must have zero pattern and shape
fires, and L must establish shape acceptance; synthetic negative traces cannot substitute.

## Index phase 2: links without ink and the long-press card (2.0.0-beta.6 candidate, not yet on a Kindle)

Not yet on a Kindle. `make check` covers links.ini's area lines byte for byte, the migration of beta.5 links
on a fixture made by the beta.5 inked flow, link undo and redo, the strip, the lasso, Link to..., Find's
pick mode, the card's rows, placement and refreshes in the headless runner, and the fit of every new string
in English and Spanish. It cannot say how the border and the card feel under a finger, whether circling
keeps up with the pen, or how the DU strips look over gray ink.

Install the build and test first in the notebooks that have your beta.5 links.

1. **Your beta.5 links.** Open each notebook: every link still goes where it went. Turn on Areas in the
   Links bar: each link's dashed box sits where its ring was, and the ring is still on the page as ink.
2. **Ink is no longer the link.** Erase an old ring completely: a finger tap where it was still follows
   the link.
3. **Making a link.** In the bar, New link (or your add-link button): a strip at the bottom says "Draw
   around what links.". Circle some words: a dashed line follows the pen with no flash, and when you lift
   it a dotted rectangle stays and "Where does the link go?" opens. No ink is left on the page, ever.
   - Circle low on the page: the chooser opens at the top, below the clock corner.
   - It lists New page..., The Index, the pages you linked or visited recently, then your Index; Search
     finds any page by number or name; Choose from the pages... opens the Pages view.
   - Pick one: "Linked to ..." (the very first time, a tip about tap and hold); from the bar, the bar comes
     back with the new row. One Undo removes the link; Redo brings it back.
   - New page...: name it; a new last page is added and linked, and you stay where you were.
   - A dot is "Too small"; Cancel says "No link made.".
4. **Holding a link.** Hold a finger on a link for about half a second: a heavy dotted border shows
   exactly what is tappable, then a small card opens beside it. The page stays in view the whole time (no
   Pages view, no full flash).
   - Rename in the Index: a keyboard opens over the page with the border still visible; Done renames that
     page in the Index everywhere. On an unnamed page the row reads "Add page N to the Index...".
   - Change where it goes...: the chooser marks the current target and has no Choose from the pages....
   - Redraw the area: circle the new area; Undo brings the old one back.
   - Delete link: "Link deleted. Undo brings it back."; a border you drew yourself stays, and so does the
     ring of a beta.5 link (deleting a link never deletes ink).
   - A tap outside the card closes it and does nothing else. With the card up the clock is not shown,
     so a tap in the clock corner closes the card too (the brief had it toggle the clock and leave the
     card: say which you prefer).
5. **Holding a row in the bar.** The same card, above the bar. A ×2 row deletes or retargets both links
   in one Undo step, and says "To change one, hold it on the page."; while peeking, holding a row first
   returns to your page.
6. **Nested links.** A small link inside a bigger one: a tap inside the small one follows the small one.
7. **Copy page** keeps the page's links, with the same areas.
8. **Clear page** in RC2 removes ink and outgoing link areas together, including on an otherwise
   blank page. One Undo/Redo restores or clears both. Ordinary erasing still leaves links intact.
9. **Feel.** The border and card appear quickly; circling never lags behind the pen; nothing flashes
   except a page change; writing right after a link is made goes on in fast pen mode.
10. **Going back (optional).** Install the previous build (f9d13f5) again for a moment: your old links
    still work there (through their ink; links made in this build have no ink, so they do nothing
    there); come back to this build: nothing was lost.
11. **Report** any link that goes somewhere else than before, and any place where the card covers what
    you wanted to see.

The original phase 2 build left links after Clear page; RC2 changes that behavior as described
above. Rename is not undone by Undo; a page made by New page... stays after Undo removes its link.
A visible border is ordinary ink, which can also use Smart Shapes in the current candidate.

## 2.0.0-beta.5 (not yet on a Kindle)

Not yet on a Kindle: the owner has not tested this release's bundle, so nothing below is claimed as
seen on a device. `make check` covers the recognizer's thresholds on recorded frames, every gesture and
the bar's and Find's flows in the headless runner (two fingers, taps, the pen near, the corner), the
Live view page and its pen path against a stand-in digitizer, the update script against a stand-in KPM,
the package and its clean-build comparison, and the fit of every new string in English and Spanish. It
cannot say how the thresholds feel under a hand, how the Links bar's refreshes look, whether the palm is
ignored, or how Live view looks in a real browser over Wi-Fi.

Install the build and test in a notebook that has your beta.5 links, unless said otherwise.

1. **Upright.** Two fingers down anywhere on a page open Pages with one flash; two fingers up close
   them. Two fingers left turn to the next page, and right to the previous one. On the last page, two
   fingers left add a page (Display > "Swiping past the last page adds a page", on by default; off, they show
   "This is the last page." and add none); on the first page, two fingers
   right show "This is the first page." The panel's + still adds a page.
2. **The Links bar.** Two fingers up (or the chain button after Next on the panel, which has a small
   dot when the page has links) open the bar along the bottom with no flash. It shows one row for each
   Index entry the page links to, numbered in reading order, with "×2" where two links go to the same
   page. Two fingers down, X, or the chain button again close it.
3. **Hopping.** Tap an entry: the page above the bar becomes that page with no flash, the panel's chip
   shows its number, and the bar keeps the starting page's links. Tap another entry: it shows that one.
   Tap the page above the bar with a finger: you stay there (one flash), and that tap does nothing else
   (no link followed, no panel button pressed). Or press X or "‹ Page 3": you are back where you started,
   zoom included. A pen stroke on the page while peeking also stays, and is not drawn.
4. **Areas.** In the bar, Areas shows dashed, numbered boxes where each link's ink is, matching the row
   numbers. They are not in the PDF. They are remembered after a restart. The first time, a tip explains
   them.
5. **Go to.** The bar's Go to (and the Pages view's "Go to page...") opens a keyboard over the page. "12"
   offers "Go to page 12"; a name finds the page with accents and capitals ignored ("arb" finds
   "Árbol"). The result opens with one flash. A key bound to Back to the page before
   returns.
6. **Sideways.** Turn the Kindle: the whole page shows (on by default). Two-finger swipes work anywhere,
   and a pinch zooms in from the whole page. Display > "Sideways, show the whole page" off: the width
   fills again, vertical two-finger drags pan, and left and right still turn pages. Zoomed in (either
   orientation): two fingers pan with no zoom drift, the panel's page chip and chain button open Pages
   and Links, and the hint about it shows once.
7. **The clock corner.** On a notebook page, a finger tap in the top left corner (14 mm) turns the clock
   and battery on and the next tap off: every tap, light or slow and hard, wherever the fingertip drifts inside the corner, with no wait
   before the next one and no pen in the way (the taps used to be "inconsistent and janky"). The battery is
   pixel art left of the percent, lit to its charge, with a bolt over it while it charges. Settings >
   Display, beside Done, has its size and its corner: each of the four corners takes the tap, and only that
   corner does. It shows on the page only: never on the Library, Settings or Share, and not with the Links
   bar, Find, Display, Paper, the tool picker or the Pages view open. Over a link in the corner the link is
   not followed. A two-finger swipe that starts there fires its gesture and does not toggle the clock.
   Live view's icon is beside the panel (it follows it when dragged or folded), and every menu is drawn
   over the panel, the Find keyboard and the Links bar too, with the panel dragged down the page.
8. **Feel.** Pinch zoom feels as before. A finger resting on a link no longer follows it when the pen
   comes near. After a long page render (a dense page or a PDF page), taps and swipes still register.
   Writing with your palm resting fires nothing.
9. **Live view.** With Live view on, committing a peek with the pen shows no ink in the browser. Write
   quickly with a textured brush (charcoal): the pen's line and its dot in the browser should glide, not
   skip (the Latency readout shows the pen age and the Playout delay). The dot's colour and size, the
   Show toolbar and Show clock and battery switches, and Monitor mode come back after a reload of the
   page. Picture in picture keeps ruled lines and thin strokes as heavy as the page view at its window's
   size. In Firefox, Record says Record (MP4) and the file plays; in Chrome too; with transparent paper
   it says Record (WebM). Menus, keyboards and the panel do not show through in the browser's copy.
10. **Report.** Write, lift, and swipe right at once: how often "Move the pen away, then swipe." appears.
    Rest your palm and thumb while writing: nothing should fire.
11. **Traces.** Create the empty file `grimoire/touch-trace`, restart Grimoire, and run the touch script in
    docs/KINDLE.md (about 15 minutes). Then delete `grimoire/touch-trace` and send `grimoire.log` and
    `grimoire/touch-trace.txt`.
12. **Updates.** Library > Settings > Updates > Check for updates, on a beta and then with releases only:
    it says what it found, the preview feed is taken out of KPM for releases only, and a failed channel
    change says so instead of offering a beta. This needs the published feed (see KPM feeds below).
13. **Settings.** `settings.ini` gains `gestures=1`, `landscape_fit=1`, `hints=...` and `link_areas=...`
    at the end, and your other settings are unchanged. Turning Grimoire back to an older build keeps
    working. After updating from the beta before, the first notebook that opens says once what two
    fingers up and down do now, and the first turn sideways says once why two fingers swipe there.

Known in this build: New link still uses the inked flow; holding a link on the page still opens the Pages
picker; holding a bar row does nothing; three-finger gestures are off.

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
