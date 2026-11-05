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

## The automated checks: make check and CI

`make check` runs the same checks as before; what changed in 2.0.0-beta.2 is how they are run.

**The scenario runner.** `tests/scenario_runner.py` runs `make scenarios` (and `make scenarios-late`
and the language runs of `make l10n-check`): every `tests/scenarios/*.qs` against its own data root
(`build/scen/NAME`) and shots folder, as the old loop in the Makefile did, but `SCEN_JOBS` at a time
(default: the number of CPUs; `SCEN_JOBS=1` runs them one after another). A scenario with a
`# root: NAME` line starts once NAME has finished (from what it left, passed or not, as before). Each
run of a batch gets a port of its own for the share server: `GRIMOIRE_TEST_PORT` (else 8765) plus its
slot, so `make scenarios` and `make l10n-check` use the ports from `GRIMOIRE_TEST_PORT` to
`GRIMOIRE_TEST_PORT + SCEN_JOBS - 1`. `make rust-check` (and so `make check`) runs `make host-checks`
beside the languages, SCEN_JOBS ports further on: the late scenarios on `GRIMOIRE_TEST_PORT + SCEN_JOBS`
and `+ SCEN_JOBS + 1`, the share server's and Live view's checks on the first of those. A worktree's
gate therefore uses the ports from `GRIMOIRE_TEST_PORT` to `GRIMOIRE_TEST_PORT + SCEN_JOBS + 1`, and
two worktrees' gates never collide when their `GRIMOIRE_TEST_PORT` values are at least `SCEN_JOBS + 2`
apart (17 at 15 jobs: 8861, 8878, 8895 and so on). `SCEN_JOBS` defaults to the number of CPUs.
Each run's output goes to `build/scen-logs/NAME.log` (the languages' to `build/scen-logs/l10n/`); one
line per run is printed as it ends, the failed runs' logs whole at the end in the order the scenarios
were named, then `SCENARIOS FAILED: ...` and a failing status, as before. `SCENARIOS=`, `SCEN_ENV`,
`SCEN_SKIP_RUST_ONLY` and `SCEN_NO_SELFTEST` work as they did.

**The timing table.** `tests/scenario-times.txt` holds each scenario's seconds from a full parallel
run on the maintainer's Mac. The runner starts the longest chains first (a root counts its
dependants), so the long scenarios never start last; the CI shards are balanced by it. A scenario
missing from it counts 5 s. `make scenario-times` runs the scenarios and rewrites it (commit the
result after adding or slowing down scenarios; `build/scen-times.txt` has the last run's times).

**The serial list.** `SCEN_SERIAL` in the Makefile names scenarios that must run alone, after the
parallel batch, because they depend on an idle machine and cannot be fixed yet, each with its reason.
It is empty: the whole set passed in parallel at 3 jobs, five times at 15 and at 32. The one failure the parallel runs
found was not load but a clock: 23 scenarios count the screen's refreshes, and a minute turning over
during the count added the clock corner's refresh; they now hold the corner's clock still
(`clock 1700000000`), as the refresh scenarios already did.

**Fewer long runs.** The handwriting replay (z_shape_writing, 85 s in real time) is four scenarios,
z_shape_writing_1 to _4, a quarter of the strokes each, which run side by side; the replay stays real
time, since the hold that the test is about fires on the real clock, also inside strokes, and with its
700 ms hold no stroke is part of a writing burst, so no part depends on the strokes before it.
y_all_screens.qs runs once in English and Gothic, in `make scenarios`; when that run was among its
scenarios (`SCENARIOS` names y_all_screens.qs and no `SCEN_SHARD` is set), `make rust-check` passes
`L10N_SKIP=en-gothic` to `make l10n-check`, whose English Gothic run was the same run (same script,
same `GRIMOIRE_CHECK_FIT=1` and address; the scenario's one `# settings:` line, status_mode=1, is what
every settings file reads as anyway). `make l10n-check` alone still makes all four.

**Times on the maintainer's Mac** (15 cores, no other gate running): the scenarios took 118 s at 15
jobs (the same set took about 30 minutes one after another), 592 s at 3 jobs (the CI runner's cores).
`make l10n-check`'s 41 runs (the Index and Wi-Fi language checks are among them now) took 78 s
instead of 175 s. `make rust-check` runs the unit tests beside the scripted checks, and l10n-check
beside `make host-checks` (on ports apart), so `make -j15 check` took 4 min 41 s instead of 44 min.

**CI** (`.github/workflows/release.yml`) splits `make check` into jobs that run at once:

| Job | Runner | What it runs (the make check part it replaces) |
|---|---|---|
| changes | Linux | Decides what runs: a pull request that changes only `*.md` and `docs/` (not `docs/WHATSNEW.md`, which the app compiles in) runs the docs job alone; a file moved into `docs/` counts its old path too |
| docs | Linux | `git diff --check` on a docs-only pull request, from where it branched off |
| build-host | macOS | `make headless tools`, uploaded for the jobs below |
| unit (host, sim) | macOS | `make test-host`, `make test-sim`: the two halves of `make test` |
| scenarios (1 to 6) | macOS | `make scenarios SCEN_SHARD=N/6`: whole `# root:` chains, balanced by the timing table (a big tree's root goes with each part); the selftest with shard 1 |
| l10n | macOS | `make l10n-check L10N_SKIP=en-gothic` (a shard made that run) |
| host-checks | macOS | b_library and s_shape_1_record, then `make host-checks`: the late-timer scenarios, the pages cache, mirror cleanup, the share server, Live view, Bluetooth, the exporter |
| package | macOS | `make package-contents-check launcher-check dev-check kpm-check kopatch-check wifi-check usb-stage-check runner-check dist` (the Kindle build, the zip and its launchers, KPM, the KOReader patches, the scenario runner's own tests); on pushes to main and releases also `make repro-check` (two clean builds) and the release files |
| gate | Linux | The one required check: every job above passed (docs-only: the docs job did); it also runs, and fails, in a cancelled run |
| publish | Linux | Releases only, after gate |

Pull requests skip `make repro-check` and the release files: they are never published, and the clean
builds take two Kindle builds' time; pushes to main and releases run them, so `main` and every
release keep the whole of `make check`. A newer push to a pull request cancels its older run; a
release run is never cancelled. The repository variable `GRIMOIRE_CI_LINUX=true` moves build-host,
unit, the shards and l10n to Linux runners, which start sooner and of which a free plan runs far more
at once (macOS: 5); it stays off until a run passes there (docs/BACKLOG.md).

**The required check.** Branch protection on `main` must require the `gate` job, and only it (one
setting, made once on GitHub: Settings, Branches, the rule for `main`, Require status checks, `gate`).
Before this split the only job was `package`; a rule that still requires `package` lets a pull request
merge with a failed unit, scenario, l10n or host-checks job. A skipped required check counts as passed
on GitHub, so `gate` runs even in a cancelled run and fails there.

**The runner's own tests.** `make runner-check` (tests/test_scenario_runner.py, part of `make check`
and of CI's package job) checks that the CI shards' split ends and keeps each scenario beside its
roots at any shard count (`SCENARIO_SHARDS` can go up safely), that a scenario under a skipped
rust-only root is skipped whatever the order, and that a failed run says why.

## 2.0.0-beta.2, Live view picture in picture in Firefox Nightly (browser checks pending)

Automated: tests/live_browser_check.js (in `make check`, through live_check.sh) checks that in video
PiP with the tab hidden a batch and a pen packet are each a frame at once, with no timer or animation
frame run, all the ink drawn and the dot at the newest sample; that a frame never asks for the next;
that the pen merely moving on draws once 8 ms at most, the next packet drawing what waited; and that a
lift draws at once; with the tab shown, that the ticker stands in for animation frames only after 34 ms
and draws nothing when one has just come, and that a frame draws only the box that changed. Measured,
not in `make check` (it needs the browsers and an idle machine):
`node tests/live_pip_bench.js` serves the page with a stand-in Kindle writing at a realistic pace (a
sample every 4 ms, sent as it comes, tile batches every 33 ms), opens it in headless Firefox
Nightly, Firefox and Chrome (each found in /Applications; `--browsers` picks), and prints, windowed and in
video PiP with the tab hidden and visible: our time per frame (averageInto, vpipDraw, the quick ink),
the time from a batch's arrival to its frame being handed to the capture, frames handed per second and
the gaps between them, and which frames the video itself shows (read back every 4 ms). `--budget`
fails when a hidden tab's PiP hands batches later than 50 ms at p90 or fewer than 30 frames a second
while writing. It also reads the page's own Latency meter (pen age at paint, p50/p90/p99/max, with the
playout delay and the time from a sample's arrival to its paint), checks that a frame drawn piecemeal
matches the same picture drawn whole pixel for pixel, and takes `--pip-size WxH` (the PiP window in CSS
pixels at 2x; 930x1240 is the Kindle's 1:1), `--fps` (Firefox's refresh rate, 60), `--pen-ms` (how
often the pen stream sends, 4), `--capture-ms` (main-thread ms per million pixels each captured frame
costs, emulated) and `--prefs name=value,...`. Headless Firefox draws canvases in software, so a GPU's
cost per pixel does not show there. In a real browser: open Live view in Firefox Nightly over plain http, press the rail's
Picture in picture, switch to another tab or app and write: the floating picture follows the pen
smoothly, as the page itself does. Then make the PiP window large and compare the Latency readout's pen
age at paint with the page's (Live view Settings, Diagnostics): within a couple of milliseconds.

## 2.0.0-beta.2, lasso (device checks pending)

Automated: tests/test_lasso.rs, tests/test_lasso_render.rs, the editor's unit tests
select_held_on_a_pen_drag and select_held_on_a_pen_hold, and the scenarios z_lasso_1_select_move (undo of
a scoped paste refreshes only its place), z_lasso_2_link_copy, z_lasso_3_dense_timing (a generated
1,000-stroke page; the move and paste times are in grimoire.log as `lasso: move ... src_ms=... dst_ms=...`),
z_lasso_4_cut_paste (a generated pair of notebooks: Cut then Paste in place, Copy then Paste 3 mm off, pen
and finger long-press paste, cut and paste across pages, the read-only page's refusals, scale-to-fit into
a smaller page size) and z_lasso_5_link_follow (a tap on a link pasted into another notebook opens the
first notebook's page 3). The live preview (docs/beta2/PIXEL-LASSO.md 2): tests/test_lasso_live.rs (the
preview folded in batches of any size equals the lift on 500 random lassos, a removed edge subtracts exactly
what it added, ties decided alike; its cost on the dense page, printed with `--nocapture`, the owner's page
with GRIMOIRE_DENSE_PAGE=a copy of p72.qpg), z_lasso_10_preview_presents (the halo follows the lasso, one
small fast refresh per word that flips, counted, the lift covers them in one Ui refresh) and
z_lasso_11_preview_leave (a word taken in and left out again; a finger's lasso shows the halo too). On the
Kindle: draw a lasso slowly round a paragraph and watch the halo follow it; lift and check the selection is
what the halo showed. The exact-area lasso (docs/beta2/PIXEL-LASSO.md 1): tests/test_lasso_cut.rs (the dab
log is the real draw's; for every tool and tip a cut stroke's pieces draw what it drew, the counted rare
one-level pixel aside, jitter tools and calligraphy pinned; pieces round-trip through the file; rect_inside
against brute force; the plan's rules; splice and back; an exact move undone and redone; the dense page's
plan time with `--nocapture`), z_lasso_12_exact_move (cut a line, move and delete its right part, undo and
redo), z_lasso_13_exact_erased_copy (a white-ink gap travels with the cut part, Copy leaves the page byte
for byte, Paste) and z_lasso_14_exact_toggle (`select-exact` on a key, the preview's cut tick, the mode
button and the key judge the same lasso again, Alt held as a lasso begins; the `mods` script line), z_lasso_16_exact_shift_key (Shift
and Alt held on a paired keyboard flip one lasso's mode), and
z_lasso_15_exact_es_gothic and _classic (the bar in Spanish with the fit check on: y_all_screens does not
open Select mode; the bar keeps beside the panel, its plain buttons narrower when six do not fit). On
the Kindle: Exact area on the bar, lasso through the middle of a written line and drag the part out; check
the part left behind and the ends; undo. On the Kindle: open the tool picker,
Select, lasso a word and its i-dots, drag it; erase part of a word with the precise eraser, lasso and move
it, then paste a copy over other ink (the ink under it must not lighten); copy a linked item into another
notebook and tap it there. Bind `select` to the pen button's drag (Settings > Gestures): hold the
button and lasso, let go, drag the selection, tap outside it and write; hold the pen still on empty page in
Select mode to paste. Time a move on the 1,090-stroke page (October-2026-02, page 72).

## 2.0.0-beta.2, page previews and dense pages (device checks pending)

Automated: tests/test_page_previews.rs (the light pages' previews written before a dense page's, a current
kept drawing read back and a stale one never used, a dense page's drawing kept by the job exactly as drawn
and no lighter page's however slow, nor again a light page's that kept one before, a preview cut short made
again, what counts as dense (a page file of 512 KB or more; a kept drawing does not make a page dense); a kept
drawing by another renderer not taken for a quick page; a page whose picture is missing keeps no drawing and
gets no preview), bgjob.rs's unit tests (a tile shows the loading mark until its preview lands and its number alone
once its batch ended without one, a check names only the tiles that changed; a waited batch goes first;
batches of screens turned away from leave the queue; never two children on the same pages; a second child
that cannot be forked puts its batch back; a running batch not asked for here is not waited on, a queued one
is), and the scenarios on
tests/previews_fixture.py (a notebook whose first page has 1,000 strokes, 1.3 MB):
z_previews_1_tiles (the light tiles come while the dense page is still drawn, which shows an hourglass, then
its tile alone is refreshed), z_previews_2_kept (the job keeps the dense page's drawing, which then opens
from it, and no light page's), z_previews_3_loading ("Loading page..." reaches the panel before a dense page
is drawn, as a notebook opens on it and as the page is turned to; never for a light page or a kept drawing),
z_previews_4_picker (the destination picker does the same), z_previews_5_turn (a screen turned to while a
batch runs gets its previews after it), z_previews_6_beside (with the cores, beside it), z_previews_7_once
(Pages opened over the idle warm-up of its own screen waits on it: no second process draws the dense page
again) and z_previews_8_moved (a page deleted while a batch runs, the view opened again: the page moved into
the screen is asked for, not left with its number). The dense pages make the job slow with
GRIMOIRE_SIM_DENSE_DRAW_MS, as a Kindle draws them; tests/pages_cache_check.py still
holds that page turns and saves write no preview. Each batch logs `previews: nb=... pages=... cached=...
kept=... drawn=... keep=... ms=... first=... slow=... cores=...` in grimoire.log.

On the Kindle, in a notebook with dense pages (2.0-b1 DEMO: pages 1, 7, 8 and 12 are 0.6 to 2.6 MB): open
Pages on a screen with a dense page: the other tiles come one by one within a second or two, the dense
page's tile shows an hourglass until its own comes. Turn to the next screen at once: its tiles come without
waiting for the dense page. Open a dense page whose preview was just made: it opens quickly. Edit a dense
page, turn away within two seconds, and come back: "Loading page..." shows until the page is drawn, and goes.
A light page never shows it (under 256 KB since the owner's decision of 2026-11-04: see the end of this
file). Write on a page whose screen has a dense page, pause three seconds (the idle save, and the warm-up
starts), then open Pages: only one `previews:` line has the dense page as its `slow=` page (it is drawn
once, not twice). In grimoire.log, check the `previews:` lines (`first=` is the first tile's
time) and `beside pid` (the second background process).

## 2.0.0-beta.2, the Terminess diet (device checks pending)

Automated: rust/tests/test_terminess_keep.rs (the embedded fonts hold exactly
`assets/terminess-keep.txt`'s code points, 1,197 of Terminus's text and 84 Material Design icons; every
icon the sources draw is listed, is in both weights and draws ink; none of the Nerd Fonts glyphs the diet
leaves out is listed), test_koreader.rs's fonts_follow_the_binary (KOReader's `fonts/Grimoire` written
again when it differs, left alone when equal, made only while the Terminess patch is applied, nothing
written without either, nothing else touched), test_scenarios.rs's
a_start_brings_koreaders_terminess_up_to_date (a start of the runner does that once), test_httpd.rs
(`/font/licenses.txt` serves the licenses of the fonts the share pages use, and each font's answer names
it), test_font and test_golden as they were (both compare on arm64 macOS, where they were recorded), and
package_check (the icons' notice and the Apache-2.0 text in `app/licenses` and beside the fonts in the
KOReader patches folder). Measured once, not in `make check`
(it needs fontTools and freetype-py): every kept code point of both weights, old file against new, drawn
through stb_truetype as font.rs draws it (every quarter pixel from 12 to 100 px, whole pixels to 200,
icons every quarter pixel to 400 px and through font_draw_icon's own sizing for boxes of 16 to 600 px:
721,833 draws a weight) and through FreeType 2.13.2 at 12 to 100 px, hinted, light, unhinted, auto-hinted
and mono, embedded bitmaps on and off (798,063 draws a weight): byte for byte the same. The same check
finds 111,192 of the 114,009 hinted outline draws differ in a subset without hinting (FreeType then
auto-hints), and the 10,773 draws at the bitmap sizes (12 to 32 px) in a subset without the embedded
bitmaps; the shipped subset keeps both.

On the Kindle: Classic and Gothic look as before (the toolbar's tools, the sheets' and the Library's icons,
a music paper's clefs, the clock corner). With the Terminess patch applied in KOReader by an earlier
version: the first start of this build logs `koreader: wrote 2 of Grimoire's Terminess files in
.../fonts/Grimoire`, the files there are the new size (427,436 and 428,220 bytes), and a second start
writes nothing; then in KOReader the menus look as before and the battery shows KOReader's own symbols
in the dropdown and the reading footer, charging and unplugged. (The folder shortcuts' list draws its
"└" from KOReader's fallback fonts now: the Nerd Fonts box drawing is gone from Terminess.) In a
browser at the Share screen's address, `/font/licenses.txt` shows the fonts' licenses.

## 2.0.0-beta.2, big page batches and finger shapes (device checks pending)

Automated: z_pages_add_8_notice ("Adding N pages..." in the refresh that takes the count pad down, from the
toolbar's pad and from the Pages view's, and gone with the refresh that shows the pages: 100 and 51 pages say it,
10 and 50 do not; a batch the storage refuses has "Couldn't add pages." in its place, and adds no page; the
100-page batch's Redo says it too, in a refresh of its own before the pages are written again, a Redo the storage
refuses has "Couldn't redo." in its place, and a 10-page batch's Redo says nothing). editor.add_notices counts
only a notice that a refresh carried before the writing began (App.toast_presents).

On the Kindle: add 60 pages from the Pages view's count pad: "Adding 60 pages..." shows until they are added,
then goes; 30 pages are added with nothing said. Undo the 60 pages, then Redo: the notice again. Each batch of 25
pages or more logs `pages: added=N of N ms=M notice=0|1 paper=drawn|picture` in grimoire.log (a Redo, `pages:
redo added=N of N ms=M notice=0|1`): M / N is a page's cost on this Kindle, which editor.rs ADD_PAGE_MS (40 ms)
estimates, and which sets where the notice starts (2 s: 51 pages at 40 ms).

A finger shape names its action only when the action did something. Automated: editor.rs's
fshape_names_its_action_only_when_it_did_something (a square bound in turn to each action: Previous page on page
1, Next page on the last page, Clear page on a blank page, Undo and Redo with nothing to undo or redo, Close menu
with nothing open, and Redo right after a stroke, whose settle is drawn first, say nothing; done, each says its
name, and so does a setting that changes with nothing drawn) and s_shape_10_nothing (the recorded Z, Undo:
nothing said on a blank page, "Undo" after a stroke, nothing once the stroke is undone). On the Kindle: bind
Undo to a shape (Settings > Gestures), draw it on a blank page: no toast; write, draw it: "Undo" and the stroke
goes; draw it again: no toast.

## 2.0.0-beta.1

The same app as 2.0.0-beta.0. Checked by the owner on the Kindle Scribe 2024 (firmware 5.20.1.0.2), from the
public preview feed after removing Grimoire through KPM: `;kpm install grimoire` from the Kindle's search bar
installed beta.0, it unpacked at its first launch from KOReader and opened with every notebook and setting kept.
The home Library did not list the Grimoire launcher until it rescans (KOReader pauses the Kindle's indexer);
not yet confirmed after a rescan.

## 2.0.0-beta.0

Checked by the owner on 2.0.0-rc.12 (Kindle Scribe 2024, firmware 5.20.1.0.2): What's new and its signature,
undo and redo on the 1,090-stroke page, folders moving, the keyboard's symbols, Pages' flicks and long-press
card, the Graph's pen tap, the new shape modes and sliders, Live view's MP4 recording and picture in picture
in Chrome, and the new layouts. Two things were not right and are fixed in beta.0, not yet seen on a Kindle:
the link outline (now visibly dithered) and link snapping (now with every Smart shapes setting). Still needs a
device: installing this beta from KPM (`;kpm install` from the Kindle's search bar), the first test of the
public feed.

## 2.0.0-rc.12 (device checks pending)

On the Kindle: a folder's menu, Move folder, into another folder and back; type "Joe's notes (2)!" on the keyboard
(the apostrophe beside the space bar, the rest on #+=). In Pages, flick one finger sideways (and a controller's
page-turn buttons) to change screens; hold a page for Rename and Delete. In the Graph, one pen tap selects, a second
opens. Smart shapes with Smallest shape at 4 mm: small circles, squares and triangles snap; slanted parallelograms
snap with Uneven allowed; Corner roundness at about 40% rounds a rectangle. Make a link and check the dithered
outline; then fingers still work (if not, send grimoire.log: it now says why). Erase a stroke on the dense page.
In the browser, drag Warmth and Brightness both ways.

## 2.0.0-rc.11 (device checks pending)

On the Kindle: the first start after installing opens What's new by itself over the Library, once (the
next start does not). The bat left of Grimoire's name, or the name, opens it again; Read all shows the
whole notes; Don't show this again, ticked, keeps the next update's notes from opening by themselves.

## 2.0.0-rc.10 (device checks pending)

On the Kindle: open the notebook with the dense page (October-2026-02, page 72): the first open draws it
(it may take half a minute, and must not be stopped), every later open is at once. After writing on it,
pause two seconds before leaving the page (the idle save keeps its drawing), or the next open draws it
again once. On that page, undo
and redo ten strokes in a row: each is immediate. Write on it with Live view open: the browser's ink
follows smoothly; with the Latency readout on, the playout delay stays a few milliseconds on a clean
link and rises only while Wi-Fi stalls.

## 2.0.0-rc.9 (device checks pending)

On the Kindle: swipe one finger left and right across a page to turn it; a slow or steep stroke
must not. In the Graph, tap dots and Back with the pen. Open Paper and New notebook: check the
three rows, the four new papers, More layouts... and Photos... beside Cancel. In Custom music,
add an 8-line tab and try each Start from layout. In Smart shapes, set Smallest shape to about
15 mm and check that a small held circle stays as drawn while a big one snaps. In Firefox, start
picture in picture from the rail, then use Firefox's own button on the picture.

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
reads View: All or View: Adjacent and a tap always switches to the other one, fitted (2.0.0-beta.2;
in rc.5 it cycled All, Around and Fit); pinch zoom remains available. Node counts describe the displayed
local and external nodes, orphans are current-notebook pages without valid links, links count
valid outgoing links, and page totals distinguish this notebook from all notebooks.

Pages shows nine previews per screen in either orientation. Tap toggles selection; a second tap
on the same tile within 450 ms opens it. Hold for 600 ms for Open / Add a link / Rename / Delete /
Cancel. Add a link opens the held page and arms its ink-free area selector; a failed page save must
not arm a link on another page. Rename opens Page name over the grid for the held page (not the open
one), holding its name. Delete deletes the ticked pages when the held page is one of them (it reads
Delete (N) then), else the held page alone; the bottom row has no Delete. Choose-from-pages
destination picking remains a single-tap operation.

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

## Links between notebooks, polish (2.0.0-beta.2 lane C, not yet on a Kindle)

Not yet on a Kindle. `make check` covers the backlinks cache (cargo tests in backlinks.rs: a save updates it
at once, a lazy check finds what another version wrote and drops deleted notebooks, a cache that does not
read is built again, 4000 links from one notebook fit and the overall limit holds), the Graph's anchors
never moving a dot or changing graph.ini's gen (mapview.rs), and the scenarios z_links_1_backlinks (Linked
from rows, the Pages card count, following a row and Back to the same page and zoom, anchors in both
languages through make l10n-check) and z_links_2_broken (a gone page and a gone notebook, from the bar, the
page and the Graph; Delete link and Undo), z_links_3_bar_learns (a bar opened before the first check is
through gets its Linked from rows by itself; a long press on a row follows it) and z_links_4_back_list (Back
across notebooks brings back that notebook's own back list). It cannot say how long the first check takes on a Kindle with
many notebooks, or whether a bar redrawn when the check finds more is noticed.

1. **Linked from.** In notebook B, link a page to a page of notebook A (Link to... > Somewhere Else). Open A
   at that page and the Links bar: "Linked from B / Page N" is listed (wait a second after opening A the
   first time: the check runs while nothing is touched). Tap it: B opens at that page.
2. **Back keeps the zoom.** Zoom in on a page of A, open the bar, tap a Linked from row into B, press Back
   (your go-back key, or the bar's Back to): A comes back at the same page, zoomed and panned as it was.
3. **Pages card.** In A's Pages, hold that page: its card says "Linked from 1 page".
4. **Graph anchors.** A's Graph shows B's page in a box at the right, with a dashed line (links that come
   here) or a solid one (links that go there); tap it to open it. No dot moved.
5. **Broken links.** Delete B's page (or B): in A, a link to it is a dashed "Page gone" chip; a tap on it,
   on the page or in the Graph opens its card: Change where it goes, Delete link. Undo brings a deleted
   link back. Restore B from trash/ by hand: the link works again.

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
   - A dot is "Link area too small."; Cancel closes it with no toast.
4. **Holding a link.** Hold a finger on a link for about half a second: a heavy dotted border shows
   exactly what is tappable, then a small card opens beside it. The page stays in view the whole time (no
   Pages view, no full flash).
   - Rename in the Index: a keyboard opens over the page with the border still visible; Done renames that
     page in the Index everywhere. On an unnamed page the row reads "Add page N to the Index...".
   - Change where it goes...: the chooser marks the current target and has no Choose from the pages....
   - Redraw the area: circle the new area; Undo brings the old one back.
   - Delete link: it goes with no toast, and Undo brings it back; a border you drew yourself stays, and so does the
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
the package and its clean-build comparison, and the fit of every new string in English, Spanish and
Japanese. It
cannot say how the thresholds feel under a hand, how the Links bar's refreshes look, whether the palm is
ignored, or how Live view looks in a real browser over Wi-Fi.

Install the build and test in a notebook that has your beta.5 links, unless said otherwise.

1. **Upright.** Two fingers down anywhere on a page open Pages with one flash; two fingers up close
   them. Two fingers left turn to the next page, and right to the previous one. On the last page, two
   fingers left add a page (Display > "Swiping past the last page adds a page", on by default; off, they stay
   on the last page and add none, with no toast); on the first page, two fingers
   right stay there. The panel's + still adds a page.
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

## 2.0.0-beta.2, Loading page from 256 KB

The owner's decision of 2026-11-04: "Loading page..." shows as a page of 256 KB or more opens with no kept
drawing (store_page_slow), where it went by the dense page's 512 KB before; the previews job still keeps a
drawing only of a page of 512 KB or more (store_page_dense). A page of handwriting is under 0.3 MB and draws
in well under a second; one of 0.3 to 0.5 MB could sit 2 to 4 s on its first open with nothing said.

Automated: tests/test_page_previews.rs's slow_page_not_dense (a page between 256 and 512 KB is slow and not
dense, one under 256 KB neither, one over 512 KB both; the job makes the slow page's preview and keeps no
drawing of it, every drawing counted as slow; a kept drawing changes neither) and z_previews_9_slow
(tests/previews_fixture.py slow: a 381 KB page with no kept drawing, its preview just made by the job, says
"Loading page..." in a refresh of its own before it is drawn, gone once the page shows; a light page with no
kept drawing says nothing; the slow page again, from the drawing the editor kept, says nothing). The scenario
fails with the notice on the 512 KB rule.

On the Kindle: open a page of 300 to 500 KB (its `p<id>.qpg` in the notebook's folder) that has no kept
drawing (write a stroke on it, turn to another page and come back): "Loading page..." shows until the page is
drawn, then goes. Turn away and back again: nothing (a page that took a second or more to draw keeps its
drawing). A light page (under 256 KB, most pages of handwriting) shows nothing. Write on the 300 to 500 KB
page again, turn to the next page, open Pages and let the page's tile come, go back and open the page:
"Loading page..." still shows, as the job keeps no drawing of a page under 512 KB (a dense page opens at
once then; in grimoire.log the batch's `previews:` line says `keep=0` when its screen has no dense page).

## 2.0.0-beta.2, count pad and drag hold

The owner's Scribe feedback of 2026-11-04: "Make the delay for reordering smaller, so that it can safely be
activated before the long-pressed management dialong. They are too disruptive as a pair." and "Pages - Batch page
adding starts the counter at 0 - and is displayed at a large, intrusive amount."

Automated: keyboard.rs `the_count_pads_opening_one_is_plain_text` (the opening 1 drawn as the pad's title is,
black on white at the title's size, with no box and no caret; a typed count has its caret), and the count pad's
behaviour as before (`the_count_pad_opens_at_one_and_the_first_digit_replaces_it`); the shots
z_pages_add_1_batch/01-pad.png (the Pages view's pad) and z_pages_add_5_toolbar/01-pad.png (the toolbar's).
z_pages_drag_hold: a page held 350 ms and moved past the slop on its own tile draws nothing and is not a drag yet;
moved off the tile it is dragged with no card, its first refresh the gap's bar alone (fast), the card's 600 ms
passing under the drag drawing nothing; a slow tap (held 350 ms, rolled 45 px, lifted on its tile) is the tap it
was: on a ticked page in Select it unticks it and the ticked pages stay where they are, and with Select off it
selects the page with one small repaint; a swipe from a page turns the screen; a contact past the slop at 100 ms
and lifted at 400 ms is a flick, and lifted past 1 s nothing, never a drag; a page held still 700 ms opens its
card; ticked pages and a pen drag on the short hold too. Its short holds are `finger` and `frame` lines, and it
runs again in `make scenarios-late` (every wait 400 ms late). z_pages_drag still drags from the card (held
700 ms); a drag lifted back in its own place takes the bar off with one fast refresh of its rect.

On the Scribe:

1. Pages, hold a page about a third of a second and move it without lifting: the black bar shows as the finger
   leaves the page and follows, and the page's card never flashes up. Drop it in another gap: the page moves,
   and Undo puts it back. Drag it out and back to where it was and lift: only the bar goes, with no redraw of
   the whole screen.
2. Hold a page still: its card opens at the same moment as before (Open, Add a link, Rename, Delete, Move to...,
   Copy to notebook...). With the card up, move without lifting: the drag starts and the card goes.
3. Flick sideways from a page, as quickly as you would to turn: the screen turns, no bar shows. Try a few
   flicks that start with a slight pause; note any that became a drag instead (how long the pause felt).
4. Tap and double-tap pages: a tap still ticks, a double tap still opens, including slow taps that roll a little
   under the finger. With Select on and a few pages ticked, slow-tap a ticked page: it unticks, and no page moves.
   Note any tap that showed the bar instead.
5. Hold New page (+): the count pad reads "Pages to add 1", a plain black 1 the size of "Pages to add", no box
   and no caret after it. Type 7: it reads 7 (not 17) with the caret after it. Add 1 page at once adds one.
   The same on the toolbar's + held on the last page.
6. Note whether the drag feels late to start on a big tile (the bar waits until the finger is off the page); if
   it does, say so, and the pick-up can move back to the slop with a drop in place made harmless instead.

## 2.0.0-beta.2, KOReader battery symbols

The owner's Scribe showed a "JS" badge where KOReader's dropdown has its battery symbol (2026-11-04). KOReader's
battery symbols are U+E778 to U+E783 and U+EDA3, from its own `nerdfonts/symbols.ttf`; a full Terminess Nerd Font
has Devicons and Font Awesome there (U+E781 is the JavaScript badge, a 90 to 99 percent battery). The likeliest
cause: the Kindle still had rc.6's Terminess patch (the RC7 sweep found it there), which takes the full Nerd Font in
`fonts/Terminus` for the menus, the footer and the dropdown's status line alike, and only a toggle in Settings >
KOReader replaced it. Not confirmed on the Kindle: it was not reachable.

Automated: tests/kopatch_check.lua (make kopatch-check) plays the patch against stand-in fonts folders and faces:
Grimoire's own copy in `koreader/fonts/Grimoire` comes first, over the whole Nerd Font in `fonts/Terminus` and over
a clean Terminus, and only its face is opened to check; Grimoire's bold alone is not its copy; without it, a font
with a glyph at a battery code point is passed over for one without, and with none without, the first by rank is
used as before; a face that cannot be read (an error, or nil as KOReader's getFace gives for a font it cannot open)
is passed over; crash.log names the menus' fonts and why each other regular face was passed over; the footer and
the dropdown's status line keep KOReader's own font. Its faces are cdata like KOReader's (an ffi metatype, so a
method a face lacks raises an error, as on the Kindle): `ftsize:hasGlyph` (true or false) as in KOReader
v2026.07.2, the owner's; `checkGlyph` (0 for none) on `ftsize` or on `ftface` as in older builds, which pick the
clean Terminus over the whole Nerd Font all the same; and neither, where nothing can be checked: the first by rank,
crash.log saying the glyphs could not be read (not that the others have one), and with Grimoire's copy, that copy
with a `not checked:` line. test_koreader.rs's
earlier_terminess_patch_follows_the_binary runs koreader_patch_refresh against the app's real files: each copy in
kindle/koreader/legacy is replaced with the current patch (a new file), this build's copy is not written again, a
copy with one byte changed or one line added and an unrelated file are kept, an absent patch stays absent, and the
code's list of earlier copies is the folder's. test_scenarios.rs's a_start_brings_an_earlier_terminess_patch_up_to_date
starts the runner with rc.6's copy in KOReader: it is the current patch after the start, grimoire.log says so, and a
second start says nothing. tests/package_check.sh checks the app carries kindle/koreader/patches and
kindle/koreader/legacy as they are. test_terminess_keep.rs's koreaders_battery_symbols_are_named_and_never_in_terminess
holds that only the patch names the battery code points (the icon scan leaves them out) and that neither embedded
Terminess has a glyph at one.

On the Kindle (with the whole Terminess Nerd Font still in `fonts/Terminus`, as on the owner's Scribe):

1. Install beta.2 and start Grimoire once from the Library. `grimoire/grimoire.log` has
   `koreader: .../koreader/patches/2-terminess-ui-font.lua was Grimoire's earlier patch (2-terminess-ui-font-rc6.lua
   in app/koreader/legacy); it is this build's now` (or `-rc7` or the plain name for another earlier copy). Leave
   Grimoire and start it again: no such line the second time. No line on the first start either: the Kindle's
   copy was already this build's, or not one Grimoire wrote (changed by hand), which Grimoire keeps; compare
   `koreader/patches/2-terminess-ui-font.lua` with the zip's `KOReader patches (optional)` copy, and if they
   differ, delete it and apply the patch again in Settings > KOReader.
2. Start KOReader. `koreader/crash.log` has `terminess ui font patch: menus in
   ./fonts/Grimoire/TerminessNerdFontMono-Regular.ttf and ./fonts/Grimoire/TerminessNerdFontMono-Bold.ttf (Grimoire's
   own copy)`, then a `passed over /mnt/us/fonts/Terminus/... (Grimoire's own copy in koreader/fonts/Grimoire comes
   first)` line for each other regular Terminess, and no `used anyway:`, `not checked:` or `could not be read`
   line. A `not checked: ... (its glyphs could not be read)` line means the patch could not ask this KOReader for
   glyphs at all (its face API differs from v2026.07.2's): the menus still use Grimoire's copy, but send crash.log.
3. Open KOReader's dropdown (tap the top of the screen): the clock, then the battery symbol and the percentage with
   a space between them, not "JS". Check it charging (the plug) and unplugged at a few levels if you can; the
   90 to 99 percent level is the one that drew "JS".
4. Open a book with the reading footer showing the battery: the battery symbol there too, not "JS".
5. The menus still read in Terminess, regular and bold (titles), as before.

## 2.0.0-beta.2, Shortcuts and Pair

Settings' Controller sheet is two now (the owner's ruling of 2026-11-04): **Settings > Shortcuts**, a top row,
and **Settings > Pair**, the bottom row's button where Controller was; each keyboard or controller can have
shortcuts of its own (docs/SHORTCUTS.md, Device profiles). The scenarios cover the sheets and the profiles
with stand-in devices (`tests/scenarios/v_shortcuts_profiles.qs`, `v_shortcuts_buttons_device.qs`,
`y_all_screens.qs`, `tests/bt/`); what
only the Scribe can show:

1. **Settings.** The top rows read Display, Pen sensitivity, Shortcuts, Calibrate pen and touch, upright
   and sideways, in Gothic and Classic, and each opens its own screen. The bottom row reads KOReader | Pair |
   Gestures | Updates | Done, and nothing under the top rows moved (Style, Mirror PDFs, the pen button row,
   turning, pinching, Language).
2. **Pair.** With the helper in `grimoire/bluetooth`, Settings > Pair > Bluetooth controller starts it,
   Pair a controller... finds and pairs a controller, its Done comes back to Pair, and the status line
   names the controller once it connects (or that another app holds it). With shortcuts off the rows are
   greyed and the line says to turn them on in Settings > Shortcuts. Esc (with shortcuts on) goes back to
   Settings.
3. **The device row.** With a keyboard and a controller connected, Settings > Shortcuts' Device row names
   one, a tap the other ("(tap for the next)" while there are two or more). Note the names it shows: they
   are what the profiles are known by.
4. **Own shortcuts.** For the controller, tap Own shortcuts for this device: it reads Own shortcuts (N, 0
   refused), the same N as Reload shortcuts.ini's, and over USB `grimoire/shortcuts/` has
   `<name>-<vendor>-<product>.ini` with its `# Device:` and `# Id:` lines. `shortcuts.ini` and
   `controller.ini` are unchanged (their dates too).
5. **Its buttons.** Buttons... is titled Buttons: NAME. Learn one button to another action, then in a
   notebook check that this controller does the new action and the keyboard (or a second controller) still
   does the default's. `controller.ini` is not touched; `<slug>.controller.ini` appears beside the profile.
   Then choose the keyboard in the Device row and open Buttons... again: it is titled Buttons (the
   default's), and a press on the controller is not learned: a toast names the controller and says to
   choose it in the Device row first.
6. **Off and on.** Turn Own shortcuts off: the controller does the default's actions again, and the files
   are all still there (plus a `.off`). Turn it on again: the learned button is back. Disconnect the
   controller: the Device row still lists it, "NAME, not connected", and its row still turns it on and off.
   Turn Enable shortcuts off with the controller connected: the Device row reads "Devices are listed while
   shortcuts are on" (no device, connected or saved), and turned back on it shows the same device.
7. **Reconnect.** Switch the controller off and on (or out of range and back) and reconnect it over
   Bluetooth: its own shortcuts apply again without opening Settings. Say whether the name it shows is the
   same each time (it is how the profile is found; a controller in another mode is another device).
8. **Edit by hand.** Edit the profile's `.ini` over USB (a `key.` line for the keyboard's profile, say), then
   Reload shortcuts.ini: the change acts for that device only, and a bad line counts in its Own shortcuts row
   as refused.
9. **Upgrade.** A Kindle that had shortcuts on in an earlier beta.2 build with an unedited `shortcuts.ini`:
   after this build's first start its first line names Settings > Shortcuts (grimoire.log says it became
   this build's); an edited one is left as it was.

## 2.0.0-beta.2, Toasts in Live view

Live view shows Grimoire's toasts now (the owner, 2026-11-05: "Enable toasts on the live view display. they
don't show up now."): a third layer of the stream, drawn over the page, the toolbar and the clock corner, with
no setting. It comes with transparency: the toast alone, and what shows under its round ends is what the
browser shows there. The automated checks: `rust/tests/test_live.rs` (the layer's rect and its gray and alpha,
moved, gone, nothing while pairing or asleep; `app_flush` gives the layer the toast alone, neither the live
chip nor a toolbar under its ends, and over what the Kindle had under it the layer is what the panel shows),
`tests/scenarios/y_live_toast.qs` (in the simulator: over the pairing card, moved by a sheet, sideways,
inverted, replaced, timed out, asleep; the layer drawn over the screen against the simulated panel),
`tests/live_stream.py` (the first browser's own toast over a real stream, clear at its corners) and
`tests/live_browser_check.js` (the page draws it on top, clears it, drops it with any state but awake, and
leaves out a layer id it does not know).
What only the Scribe and a real browser can show:

1. **The first one.** Pair a browser. As the card closes, the Kindle says "A computer is watching this screen."
   for 3 s: the browser shows it in the same place with the same look (black, white words, round ends; in
   Gothic its pointed ends and inner line), and it goes from both together.
2. **On a page.** In a notebook, choose the lasso and circle empty paper: "Nothing to select." on both, over
   the page; gone from both after 2 s. To see one toast replace another: bind a key or a controller button to
   light-up (none is bound by default: Settings > Shortcuts > Buttons... for a controller, or a `key.` line in
   shortcuts.ini, docs/SHORTCUTS.md) and press it a few times quickly: each "Light N of 24" replaces the last
   in the browser too.
3. **Switches and the toolbar.** Drag the toolbar down to the foot of the page, under where toasts show, and
   make "Nothing to select." over it: on both, the toast covers the toolbar and the toolbar shows around its
   round ends. In the browser's View settings, switch Show toolbar and Show clock and battery off and make the
   toast again: it still shows, and around its round ends is the page, with no piece of the hidden toolbar.
   The same in Gothic (Settings, Style: tap for Gothic), at its pointed ends. Switch them back on and put
   the toolbar back.
4. **Loading page...** Turn to a dense page that says "Loading page..." as it opens: the browser shows it while
   the Kindle draws, and it goes as the page shows.
5. **It moves.** Make "Nothing to select." and at once open the toolbar's Pages view: the toast moves above the
   Pages view's buttons on the Kindle and in the browser alike. Make one and turn the Kindle sideways: the
   browser shows it in its sideways place.
6. **Asleep.** Make "Nothing to select." and press the power button while it is up (within its 2 s): the
   browser says the Kindle is asleep and the toast leaves its picture at once. Wake the Kindle: no toast on
   either.
7. **The pairing card.** With a browser watching, open Library > Share > Live view again (the card): the
   browser says it is pairing and shows nothing new. To have a toast up as the card closes, turn on a paired
   Bluetooth keyboard or controller while the card shows (Shortcuts on): the Kindle says "Controller
   connected" for 1.5 s; tap Done while it is up, and the browser shows it as the card closes. Again, but wait
   for it to go before Done: it does not come back. (That toast comes at most once in 30 s: turn the device
   off and wait between the two.)
8. **Inverted.** With Invert colors on (Settings > Display), the browser's toast is inverted with the rest of
   the picture, as on the Kindle (white with black words), and the browser's Invert colors switch turns it
   with the picture.
