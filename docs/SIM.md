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
| `P` | power button: saves, then the sleep screen (a press while asleep wakes) |
| `Esc`, Cmd+Q | quit |

`GRIMOIRE_SIM_SIZE=WxH` changes the virtual panel; `GRIMOIRE_SIM_STATS=1` prints refresh counts.

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
| `power`, `quit` | power button / quit (end of script also quits) |

Coordinates are screen pixels: portrait 1860x2480, or 2480x1860 after `rotate 1` or `rotate 3`,
calibrated or not (as on the Kindle, a calibration maps the digitizer to the upright panel, so one
made upright stays right sideways). The lines up to and including the next `wait`, `shot`, tap,
drag, `pinch`, `rotate`, `sleep`, `wake` or `quit` are delivered as one batch, like a burst of
digitizer samples between two screen refreshes on the device; a check after pen, touch, side,
eraser or power lines ends a batch too. A line the runner does not know fails the run like a
failed check (a misspelt `expect` would otherwise be skipped).

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
meant to quit (Library > Exit on purpose), `expect running is 0` after it holds once it has quit,
and fails while it still runs. The run ends with `expect: all 33 checks passed` or `expect: 2 of 33
checks failed`, and exits with status 3 if any check failed. To find a value, write the check with
a guess: the failure shows what the app holds.

| key | value |
|---|---|
| `running` | `1` while the app runs, `0` for a check after it quit (the runner answers this one) |
| `screen` | `library`, `editor`, `penset` (pen feel), `calib` or `share` |
| `toast` | the toast showing, empty if none |
| `asleep` | `1` while the sleep screen is up, else `0` |
| `rotation` | the quarter turns applied, 0..3 |
| `size` | the screen, `1860x2480` or `2480x1860` |
| `settings.KEY` | the setting as settings.ini stores it, for `theme` (`gothic` or `classic`), `invert`, `mirror_pdf`, `follow_rotation`, `pinch_zoom`, `side_button` (0 nothing, 1 eraser, 2 highlighter), `sleep_mode` (0 the page, 1 a pinned page, 2 the Kindle's screensaver), `sleep_minutes`, `pen.valid` and `touch.valid` (1 once calibrated) |
| `settings.TOOL.PARAM` | a tool's pen feel as settings.ini stores it, four decimals: TOOL as for `editor.tool`, PARAM a slider's settings.ini name (`size_min_mm`, `size_max_mm`, `opacity_min`, `opacity_max`, `pressure_floor`, `pressure_ceiling`, `pressure_gamma`, `tilt_min_deg`, `tilt_max_deg`, `tilt_size`, `tilt_opacity`, `grain`, `smoothing`), e.g. `settings.fountain.pressure_gamma` |
| `light.intensity`, `light.warmth` | the front light (the simulator starts at 12 and 8) |
| `editor.page`, `editor.pages` | the open page (from 1) and the page count, 0 with no notebook open |
| `editor.tool` | the pen's tool by its settings.ini name: `pencil`, `mech`, `ballpoint`, `eraser`, `highlighter`, `fountain`, `fineliner`, `marker`, `charcoal` or `calligraphy` |
| `editor.zoom` | the zoom as the zoom chip shows it, with two decimals: `1.00` is the usual view (100% upright, the page's width sideways) |
| `editor.modal` | what is in front of the page: `none`, `tools` (the tool picker), `paper` (Paper for this page), `music` (Custom music), `display` (the Display sheet) or `pages` (the Pages view) |
| `editor.strokes` | strokes on the open page (a stroke counts once the pen lifts) |
| `editor.ticked` | pages ticked in the Pages view |
| `editor.paper` | the page's paper as its number in src/paper.h (1 medium rule, 2 wide rule, 3 sheet music, 6 medium rule without a margin, 8 grand staff...), `custom` for a Custom music layout, `none` with no notebook open |
| `editor.panel` | `expanded` or `collapsed` |
| `editor.music` | the Custom music layout on the Custom music sheet while it is open, else the page's if its paper is custom, else empty: the systems per page, a colon, then the parts top to bottom (`treble`, `bass`, `alto`, `tenor`, `drums`, `plain`, `tab4` to `tab7`, `lyrics`), e.g. `4: treble tab6 bass` |
| `library.sheet` | what is open over the cards: `none`, `new` (New notebook), `settings`, `more` (a notebook's menu), `folder-more` (a folder's menu), `move` (Move to), `delete`, `keyboard`, `display` or `music` |
| `library.folder` | the open folder, empty at the top level |
| `library.items` | the cards in the open folder, on every page of cards: at the top level its folders and the notebooks in no folder |
| `library.page`, `library.pages` | the page of cards (from 1) and how many there are |
| `library.text` | the keyboard's text while it is open |
| `library.paper` | the paper picked in New notebook, numbered as for `editor.paper` |
| `library.music` | as `editor.music`, for New notebook's Custom music sheet and its picked paper |

The `editor.` and `library.` keys answer on any screen: the editor's describe the open notebook,
also while pen feel is in front.

A headless run ends only once the PDF exports it started are written (the app leaves them to
child processes that outlive it), so the data a run leaves is whole.

`make scenarios` runs every scenario on a fresh data root, or, for a script with a `# root: NAME`
line, on a copy of scenario NAME's (d_persist reopens what b_library made). A run that leaves a
file half-written (`*.tmp`) fails. Every scenario runs, then the rule fails naming those that did
not pass. Last it checks the runner itself: `tests/selftest/pass.qs` must pass every check (exit 0)
with its notebook's PDF written, and in `tests/selftest/fail.qs` every line but the comments, waits
and taps must fail and be reported by its line number (exit 3).
