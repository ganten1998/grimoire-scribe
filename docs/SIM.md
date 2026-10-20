# Desktop simulator

`make sim && ./build/host/grimoire-sim` opens a window showing the 1860x2480 page scaled to fit,
with refreshes emulated (DU regions show as pure black/white until a later GL16 settles them).
Data goes to `./grimoire-data` (or `$GRIMOIRE_ROOT`).

| input | does |
|---|---|
| left mouse | pen (down / drag / up) |
| right mouse | finger |
| `1`..`9`, `0` | pen pressure 0.1..0.9, 1.0 (also `GRIMOIRE_SIM_PRESSURE`) |
| arrow keys | tilt x / y in 5 degree steps (shown in the title bar) |
| hold `E` | eraser end |
| hold `S` | side button |
| `P` | the power button: a short press saves and sleeps (through the fallback, 2.5 s after the release), a press on the sleep screen wakes (3 s after); hold it 4 s to leave Grimoire |
| `Esc`, Cmd+Q | quit |

`GRIMOIRE_SIM_SIZE=WxH` changes the virtual panel (`1986x2648` is the Scribe 3's and the
Colorsoft's); `GRIMOIRE_SIM_STATS=1` prints refresh counts.
Two more slow the simulator down like a Kindle (simulator only): `GRIMOIRE_HTTPD_DELAY_MS` makes
each share server render wait that long first, and `GRIMOIRE_BG_DELAY_MS` each background PDF
export, so the next one queues behind it.

Four more:

- `GRIMOIRE_LANG=es` (or `en`, or a locale name such as `es_MX.UTF-8`) picks the language that
  Settings > Language's Automatic stands for, ranking above `LC_ALL`, `LC_MESSAGES` and `LANG`.
  Without it the simulator follows those; it never reads the Kindle's `/var/local/system/locale`.
- `GRIMOIRE_L10N_DIR=<dir>` (simulator and headless runner only) reads a language's catalog from
  `<dir>/<code>.mo` instead of the binary, once per run, and logs `i18n: (simulator) the catalog
  for es comes from <dir>/es.mo` (or `is not in`, and then the built-in catalog is used): a
  translator can try a compiled .po without building again (docs/TRANSLATING.md).
- `GRIMOIRE_CHECK_FIT=1` (simulator and headless runner only) lists every text cut short to fit,
  once per text and room, as `fit[es]: "..." needs W px, has R`, and the run then ends with
  `fit: N texts cut short to fit` and exit status 3.
- `GRIMOIRE_HOST=koreader` starts it as KOReader's Grimoire plugin does (on the Kindle,
  `app/grimoire.sh --hosted koreader` sets it): the Library's Exit reads Back to KOReader, and
  sleep mode 2 shows the page.

## Headless scripts

`make headless` builds `grimoire-headless`, which links no SDL and never opens a window. It runs a
script and can dump the panel to PNG; `make scenarios` runs every `tests/scenarios/*.qs`.

```
GRIMOIRE_ROOT=/tmp/q GRIMOIRE_SCRIPT=tests/scenarios/a_notebook.qs GRIMOIRE_SHOTS=/tmp/shots build/host/grimoire-headless
```

| command | meaning |
|---|---|
| `pen down X Y [p] [tx] [ty]`, `pen move ...`, `pen up X Y` | raw pen events |
| `pen line X0 Y0 X1 Y1 STEPS P0 P1 [tx ty]` | a stroke with a pressure ramp |
| `pen curve X Y R STEPS P [TURNS] [tx ty]` | cursive-like loops |
| `eraser line X0 Y0 X1 Y1 STEPS` | the same with the eraser end |
| `side on` / `side off` | hold the side button |
| `pentap X Y`, `tap X Y` | pen tap / finger tap |
| `touch down/move/up X Y` | raw finger events |
| `drag pen\|touch X0 Y0 X1 Y1 STEPS` | a slow drag (100 ms per step) |
| `pinch CX CY D0 D1 STEPS [DX DY]` | two fingers around (CX, CY), D0 apart, spreading to D1 while the midpoint drifts by (DX, DY) |
| `rotate N` | the Kindle now turned N quarter turns clockwise (0..3) |
| `sleep`, `wake` | the power service announcing the sleep screen, and its end |
| `wait MS` | let timers run (autosave, toasts, the share server) |
| `shot NAME` | write what the panel shows to `$GRIMOIRE_SHOTS/NAME.png` |
| `expect KEY is\|has\|not VALUE` | check the app's state (see Checks) |
| `power` | a short press of the power button and its release (150 ms) |
| `power down`, `power up` | one half each; the release reports the real time between them, so `power down`, `wait 3000`, `power up` is a 3 s press |
| `power held` | the 4 s mark of a press still down (IN_POWER_HELD alone, what input_evdev.c sends then), so `power down`, `power held` leaves while the key is down (check that with `expect running is 0`) |
| `powerhold` | the press, the 4 s hold and the release at once, with no time passing: Grimoire saves and quits (check that with `expect running is 0`) |
| `powerd STATE` | what the simulated power service reports from now on: `active` (the start), `screenSaver`, `readyToSuspend` or `suspended`; `unknown` or nothing reads as unreadable |
| `storage full`, `storage ok` | from now on every file write fails, as on a full userstore, or works again |
| `quit` | quit (end of script also quits) |

The simulated power service answers a sleep request (powerButton) with a sleep (goingToScreenSaver)
while it is active or unreadable, and with a wake (outOfScreenSaver) while it is in
`screenSaver`, `readyToSuspend` or `suspended`, so a request sent to a dozing Kindle shows up. Its
state changes only through `powerd`. tests/scenarios/p_power.qs, p_power_locked.qs and
p_power_released_late.qs (a 4.1 s press whose hold never arrived) use these commands, and
v_storage_full.qs leaves with a page that cannot be saved.

Coordinates are screen pixels: portrait 1860x2480 (or the size `GRIMOIRE_SIM_SIZE` gives), or
2480x1860 after `rotate 1` or `rotate 3`, calibrated or not (as on the Kindle, a calibration maps
the digitizer to the upright panel, so one made upright stays right sideways). The lines up to and
including the next `wait`, `shot`, tap, drag, `pinch`, `rotate`, `sleep`, `wake`, `powerhold` or
`quit` are delivered as one batch, like a burst of digitizer samples between two screen refreshes
on the device; a check after pen, touch, side, eraser or power lines ends a batch too. A drag or a
pinch is all one batch, so its frames go out as one refresh at its end: to see a fast refresh
(`display.last_fast`), use `touch down` and `touch move` lines, as tests/scenarios/y_live_ink.qs
does. A line the runner does not know fails the run like a failed check (a misspelt `expect` would
otherwise be skipped).

The keyboard's keys in portrait (the Library's keyboard, for every name, at 300 dpi): shift
175 2205, del 1679 2205, Cancel 217 2365, `-` 477 2365, the page key 696 2365 (labelled with a
and e acute on the letters page, abc on the accents page), space 1060 2365, `.` 1382 2365, Done
1643 2365. On the letters page: y 1725 the digits and y 1885 q to p at x = 133 + 177 n, y 2045
a to l at x = 221 + 177 n, y 2205 z to m at x = 393 + 177 n. On the accents page: y 1725 a e i o u with an acute, then with a grave, at x = 133 + 177 n; y 1885 the
same with a circumflex, then a diaeresis, at the same x; y 2045 n tilde, c cedilla, a and o
tilde, sharp s, oe, ae, o slash, a ring at x = 221 + 177 n; y 2205 the inverted question and
exclamation marks, the angle quotes and the middle dot at x = 570 + 177 n. In Spanish the letters
page's y 2045 row is a to l and then n tilde, ten keys at x = 133 + 177 n.
tests/scenarios/u_accents.qs types accented names with them.

These hold in both styles, in English and in Spanish (tests/scenarios/y_all_screens.qs taps them
in each). Upright: the Library's top bar has New at 965 95, Folder 1155 95, Share 1345 95 and
Settings 1535 95. Its five buttons widen together for a longer word, up to 220 px each: New is at
965 only while they are at most 191 px wide, Folder at 1155 up to 198 and Share at 1345 up to 212
(the same sideways), so a language with wider top-bar words moves them (docs/TRANSLATING.md).
Hosted by KOReader they move left, as Back to KOReader takes the room it needs. Settings' rows
are at x 930: Pen sensitivity y 460, Calibrate 620, Display 780, Style 930, Mirror PDFs 1060, the
pen button's row 1190, Turn with the Kindle 1320, Pinch 1450 and Language 1580. The Language list
has Automatic at 930 785, English 930 945, Español 930 1105 and Cancel 930 1265. Pen feel's tabs
are at x = 130 + 177 n, y 289, in the order pencil, mechanical pencil, charcoal, ballpoint,
fineliner, marker, fountain pen, calligraphy pen, highlighter, eraser. Sideways (2480x1860): the
top bar has New at 1585 95, Share 1965 95 and Settings 2155 95; Settings' rows are at x 1240: Pen
sensitivity y 260, Calibrate 425, Display 590, Language 1385; the Display sheet's sleep screen row
is 1240 1005 and its Done 1770 1325; the Language list's Cancel 1240 1055. The first card's menu
button is 431 896; in a notebook's menu Move to folder is 1240 790 and Delete 1240 1110, Delete's
Keep it 1240 845, Move to's Cancel 1240 1055, and a folder's menu has Cancel at 1240 950. Share's
Done is 2160 1485. In New notebook, Rename is 1950 1424 and the Custom music tile 1800 1115 (its
sheet's Cancel 598 1693); the keyboard's page key is 928 1745 and its Cancel 279 1745. Pen feel's
tabs are at x = 161 + 239 n, y 289 (the calligraphy pen's 1834). Calibration's corners are 298 223,
2182 223, 2182 1637 and 298 1637, its center 1240 930, touch's skip 1240 1475 and the check's
Cancel 1240 1675. The editor's tool button is 2388 358.

### Checks

`expect KEY is VALUE` (exactly), `expect KEY has TEXT` (contains it) and `expect KEY not VALUE`
check the app's state. The value is the rest of the line after one space: it may hold spaces, it
may be empty (`expect toast is` holds while no toast shows), and it has no trailing comment.

A check sees the app once everything before it was handled, ticked and flushed. Right after a
line that ends a batch (a tap, `wait` or `shot`, say) it runs at once, adding no refresh and no
timer tick, so checks can go into a script without changing what the script does; that is one
main-loop pass before a shot in its place would be taken, so a timer due at that very moment can
read one way for the check and the other for the shot. After pen, touch, side, eraser or power
lines a check waits for their batch to be handled and flushed, like a shot.

A failed check is printed and the run goes on:

```
expect failed (line 12): screen is "library", wanted is "editor"
```

An unknown key, a line that cannot be read, an unknown command, and a check the script never
reached because the app quit first (a tap that lands on Library > Exit) fail too. Where the app is
meant to quit (Library > Exit on purpose, `power held` or `powerhold`, the second press on a Kindle
that stays locked), `expect running is 0` after it holds once it has quit, and fails while it
still runs.
The run ends with `expect: all 33 checks passed` or `expect: 2 of 33 checks failed`, and exits
with status 3 if any check failed. To find a value, write the check with a guess: the failure
shows what the app holds.

| key | value |
|---|---|
| `running` | `1` while the app runs, `0` for a check after it quit (the runner answers this one) |
| `watching` | `1` while input_poll also watches the share server's socket, else `0` (the runner answers this one too) |
| `screen` | `library`, `editor`, `penset` (pen feel), `calib` or `share` |
| `toast` | the toast showing, empty if none |
| `toast.rect` | where the visible toast is, `x y w h` in screen pixels, empty if none |
| `asleep` | `1` while the sleep screen is up, else `0` |
| `asleep.mode` | while asleep, which sleep screen is up, numbered as `settings.sleep_mode` after the hosted fallback (hosted by KOReader, mode 2 reads `0`); empty while awake |
| `host` | `koreader` when started as KOReader's Grimoire plugin starts it (`GRIMOIRE_HOST=koreader`), else empty |
| `language` | the language in use, `en` or `es` |
| `rotation` | the quarter turns applied, 0..3 |
| `size` | the screen, `1860x2480` or `2480x1860` (`1986x2648` or `2648x1986` under `GRIMOIRE_SIM_SIZE=1986x2648`) |
| `settings.KEY` | the setting as settings.ini stores it, for `theme` (`gothic` or `classic`), `invert`, `mirror_pdf`, `follow_rotation`, `pinch_zoom`, `side_button` (0 nothing, 1 eraser, 2 highlighter), `sleep_mode` (0 the page, 1 a pinned page, 2 the Kindle's screensaver), `sleep_minutes`, `pen.valid` and `touch.valid` (1 once calibrated); `language` (`auto`, `en` or `es`, and `auto` for a value this version does not speak, which settings.ini keeps as it was); `pen.panel` and `touch.panel` (the panel the calibration was made on, `WxH`, empty when not recorded) |
| `settings.TOOL.PARAM` | a tool's pen feel as settings.ini stores it, four decimals: TOOL as for `editor.tool`, PARAM a slider's settings.ini name (`size_min_mm`, `size_max_mm`, `opacity_min`, `opacity_max`, `pressure_floor`, `pressure_ceiling`, `pressure_gamma`, `tilt_min_deg`, `tilt_max_deg`, `tilt_size`, `tilt_opacity`, `grain`, `smoothing`), e.g. `settings.fountain.pressure_gamma` |
| `light.intensity`, `light.warmth` | the front light (the simulator starts at 12 and 8) |
| `editor.page`, `editor.pages` | the open page (from 1) and the page count, 0 with no notebook open |
| `editor.tool` | the pen's tool by its settings.ini name: `pencil`, `mech`, `ballpoint`, `eraser`, `highlighter`, `fountain`, `fineliner`, `marker`, `charcoal` or `calligraphy` |
| `editor.zoom` | the zoom as the zoom chip shows it, with two decimals: `1.00` is the usual view (the page's width fitting the screen's: 100% upright for a notebook of this screen's size) |
| `editor.zoomed` | `1` while the zoom chip shows, else `0` |
| `editor.scale` | screen px per page px, two decimals: `1.00` for a notebook of this screen's size upright, `1.07` for an 1860x2480 notebook on a 1986x2648 screen, `0.94` the other way; `0.00` with no notebook open |
| `editor.page_size` | the open page's own size, `WxH`: its notebook's, so a page that came at the screen's size instead would show here; empty with no notebook open |
| `editor.stroke_start` | where the open page's newest stroke starts, in page px rounded, `x y`; empty with no stroke |
| `editor.modal` | what is in front of the page: `none`, `tools` (the tool picker), `paper` (Paper for this page), `music` (Custom music), `display` (the Display sheet) or `pages` (the Pages view) |
| `editor.strokes` | strokes on the open page (a stroke counts once the pen lifts) |
| `editor.unsaved` | `1` while the open notebook has changes not yet written, else `0` |
| `editor.ticked` | pages ticked in the Pages view |
| `editor.paper` | the page's paper as its number in src/paper.h (1 medium rule, 2 wide rule, 3 sheet music, 6 medium rule without a margin, 8 grand staff...), `custom` for a Custom music layout, `none` with no notebook open |
| `editor.panel` | `expanded` or `collapsed` |
| `editor.music` | the Custom music layout on the Custom music sheet while it is open, else the page's if its paper is custom, else empty: the systems per page, a colon, then the parts top to bottom (`treble`, `bass`, `alto`, `tenor`, `drums`, `plain`, `tab4` to `tab7`, `lyrics`), e.g. `4: treble tab6 bass` |
| `editor.music_max` | while the editor's Custom music sheet is open, the most systems its layout fits on the page it is for (that page's height), else empty |
| `library.sheet` | what is open over the cards: `none`, `new` (New notebook), `settings`, `more` (a notebook's menu), `folder-more` (a folder's menu), `move` (Move to), `delete`, `keyboard`, `display`, `music` or `language` (Settings > Language's list) |
| `library.hint` | `1` while the empty Library's (or an empty folder's) two lines show ("No notebooks yet" or "This folder is empty", then "Tap New to start one."), `0` when there are cards, and under every sheet but the keyboard |
| `library.folder` | the open folder, empty at the top level |
| `library.items` | the cards in the open folder, on every page of cards: at the top level its folders and the notebooks in no folder |
| `library.page`, `library.pages` | the page of cards (from 1) and how many there are |
| `library.text` | the keyboard's text while it is open |
| `library.paper` | the paper picked in New notebook, numbered as for `editor.paper` |
| `library.music` | as `editor.music`, for New notebook's Custom music sheet and its picked paper |
| `display.ink` | `1` while a pen stroke is marked live ink (`display_live_ink`), else `0` |
| `display.pen_mode` | `1` while the app asks for MTK pen mode (the editor's ink strokes only), else `0` |
| `display.last_fast` | `ink` if the last fast refresh was live ink, `wait` if a color panel would have waited for the driver's read after it (docs/DESIGN.md), empty before any |

The `editor.` and `library.` keys answer on any screen: the editor's describe the open notebook,
also while pen feel is in front. The `display.` keys are the simulated panel's (the runner answers
them, like `running` and `watching`); tests/scenarios/y_live_ink.qs uses them, on a fresh root at
1860x2480: ink and the eraser in the editor, the pen feel pad and the calibration's check stroke
are live ink, and a pressed button or a dragged slider is not.

y_pagesize_1_ksc opens b_library's data on a 1986x2648 screen (fitted notebooks, new pages at the
notebook's size, Custom music for its page, a scaled calibration, a notebook made there), and
y_pagesize_2_back takes it back to 1860x2480. y_koreader_hosted runs as the plugin starts Grimoire:
its shots are 01 and 02 (Back to KOReader in Gothic, upright and sideways), 03 (sleep mode 2 asleep:
the page) and 04 and 05 (Classic); y_koreader_not_hosted, from the Library, takes no shots and
holds that mode 2 stays the Kindle's own screensaver there (`asleep.mode` 2).

A headless run ends only once the PDF exports it started are written (the app leaves them to
child processes that outlive it), so the data a run leaves is whole.

`make scenarios` runs every scenario on a fresh data root, or, for a script with a `# root: NAME`
line, on a copy of scenario NAME's (d_persist reopens what b_library made; NAME must sort before
the script). The order is the shell's glob order, which follows the locale's collation and may
pass over punctuation: a scenario that starts from another must differ from its name by a letter
or a digit, not only by punctuation or length (hence y_pagesize_1_ksc and y_pagesize_2_back).
`# env: K=V` lines set the environment of that one scenario (values without spaces;
r_passcode, s_passcode_once, t_passcode_left_off and t_toast_over_sheets set `GRIMOIRE_PASSCODE`,
as the launcher does on the Kindle, v_leave_from_editor sets `GRIMOIRE_BG_DELAY_MS`,
y_koreader_hosted sets `GRIMOIRE_HOST` as the KOReader plugin's start does and
y_koreader_not_hosted sets it empty, y_language and z_language_kept set `GRIMOIRE_LANG`,
y_pagesize_1_ksc sets `GRIMOIRE_SIM_SIZE=1986x2648`, and y_all_screens sets `GRIMOIRE_CHECK_FIT`
and a made-up `GRIMOIRE_SIM_ADDRS`). The rule first unsets `GRIMOIRE_HOST`, `GRIMOIRE_PASSCODE`,
`GRIMOIRE_SIM_ADDRS`, `GRIMOIRE_HTTPD_DELAY_MS`, `GRIMOIRE_BG_DELAY_MS`, `GRIMOIRE_SIM_PRESSURE`,
`GRIMOIRE_SIM_STATS`, `GRIMOIRE_SIM_SIZE`, `GRIMOIRE_CHECK_FIT`, `GRIMOIRE_LANG` and
`GRIMOIRE_L10N_DIR` (tests/share_check.sh does too), so only `# env:` lines set them, and runs
every scenario in English (`GRIMOIRE_LANG=en`) unless its `# env:` line names another language:
the scenarios tap by coordinates and check English text. A `# pdfs: N` line wants the run to
leave exactly N PDFs in its root's PDFs/ folder (v_leave_from_editor: 2, y_koreader_hosted: 1).
With `GRIMOIRE_TEST_PORT` set, the rule writes
`http_port=` that port into every scenario's settings.ini before the run (over a copied root's
too), so the share servers of checks running in several worktrees never collide on 8765.
`# settings: key=value` lines are appended to the scenario's settings.ini before the run (after
the port), for settings that have no control on screen yet. These header lines are read by the
Makefile rule only: running `grimoire-headless` by hand skips them like any comment. A run that
leaves a file half-written (`*.tmp`) fails. Every scenario runs, then the rule fails naming those
that did not pass. Last it checks the runner itself: `tests/selftest/pass.qs` must pass every
check (exit 0) with its notebook's PDF written, and in `tests/selftest/fail.qs` every line but the
comments, waits and taps must fail and be reported by its line number (exit 3).

## Languages in tests

`make l10n-check`, part of `make check`, checks every language: English and each po/*.mo.

- Each committed `po/<lang>.mo` must be what msgfmt makes of `po/<lang>.po` now; one that is not
  fails with `l10n: po/es.mo is not what msgfmt makes of po/es.po now (run make l10n)`. Without
  msgfmt this part is skipped, and says so.
- tests/scenarios/y_all_screens.qs visits every screen and sheet, upright and sideways. It runs in
  every language, in Gothic and in Classic, with `GRIMOIRE_CHECK_FIT=1` and a made-up wlan0
  address, and a text cut short to fit fails the run (`fit[es]: ...`). Its taps land in both
  styles, in English and in Spanish (the coordinates above), and it checks only what does not
  depend on the language. Its shots, `build/shots/l10n/<lang>-<style>/` (01-library-empty to
  65-music-landscape), show translators every screen in context. `make scenarios` runs it too, in
  English and Gothic.
- In each language, tests/l10n/long_root.qs opens Settings from a data root whose path is longer
  than the sheet is wide (`build/scen/l10n/long-root/a-long-data-root-...<lang>`): its Data line, a
  path, is cut short as names are, and the check must not count it, or make check would fail in a
  deep build directory.
- tests/l10n/hosted.qs, in every language and style with `GRIMOIRE_HOST=koreader`: the Library's
  top bar as KOReader opens it, Back to KOReader in Exit's place, upright and sideways (shots in
  `build/shots/l10n/hosted-<lang>-<style>/`).

Under make check it runs after the scenarios, since both open the share server on the same port.
The scenarios y_language (with `GRIMOIRE_LANG=es`: Automatic is Spanish, a pick applies at once
and is saved, Cancel changes nothing) and z_language_kept (`# root: y_language`: the pick outlasts
a restart, and Automatic hands the choice back) check Settings > Language, and tests/test_i18n.c
what no scenario reaches.

## The Share screen in tests

`GRIMOIRE_SIM_ADDRS` (simulator and headless runner only) replaces the machine's network
addresses, for example `GRIMOIRE_SIM_ADDRS="wlan0=192.168.0.20,usb0=192.168.15.244"`, or `""` for
none. Without it, the simulator's QR code takes the first wlan* or en* address (a Mac's en0).
`tools/qrdecode.swift` is a macOS Vision decoder (`qrdecode [--polarity] image...`) that
`tests/share_check.sh` builds into `build/host/qrdecode`. The check takes `BUILD` (default
`build`, the Makefile passes its own) for the app, the decoder and the shots, and runs its own
copy of `c_share` held open 25 s longer for its downloads (`make scenarios` runs the short one).
Besides `c_share`, it writes `build/shots/c_share_classic`, `q_share_rotate`, `q_share_invert` and
`share_layouts/<style>-<case>`: the style is gothic or classic, the case one, two, three, usb-only
or none, and each folder holds `portrait.png` and `landscape.png`. It keeps step with
q_share_rotate by its shot names (01-share-portrait to 07-share-landscape-again, with
04-asleep-no-code the sleep screen), so a renamed shot there needs the check changed too. The
check fails on any `share: layout:` line in its app logs, and, over the made-up address lists, on
an address in any line but the simulator's own `share: (simulator) ...` lines, which it compares
with the codes it reads back.

The check runs in English (`GRIMOIRE_LANG=en`) except for two runs. One serves the pages in Spanish:
`<html lang=es>`, the index's line, a notebook's page with the accents of its bold links spared
(`<span class=na>`), the studio's words as `window.GRIMOIRE_L10N` with its code unchanged, and a
missing notebook's answer; the Share screen's shot is `build/shots/c_share_es/01-share.png`, the
pages `build/share-check/index-es.html`, `nb-es.html` and `studio-es.html`. The other serves them
from a catalog of markup: tests/l10n_markup.py puts `</script><x-l10n a='"'>&amp;` before every
translation of po/es.mo, the app reads that catalog through `GRIMOIRE_L10N_DIR`, and the index, a
notebook's page and the studio (its script too) must show it escaped, never as markup. That run
taps Share at 1220 95, where the widened top bar puts it. Last, with node,
tests/studio_copy_check.js tries the studio's Copy command on a page without a clipboard (plain
http), where the browser copies and where it will not.
