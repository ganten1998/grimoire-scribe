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
export, so the next one queues behind it. `GRIMOIRE_SLEEP_MINUTE_MS` (simulator only) makes a
minute of the Display sheet's "Sleep after" last that many milliseconds, so a scenario can see the
timer run out.

Live view's raw digitizer has stand-ins (headless runner only; tests/live_pen_stall.py uses them):
`GRIMOIRE_SIM_RAWPEN=PATH` names a FIFO of `struct input_event` records in the host's layout (x
and y in screen pixels, pressure 0..4095, tilt in hundredths of a degree) that a pen viewer opens
as its own reader of the node (`input_raw_pen`), and `GRIMOIRE_SIM_RAWPEN_MAIN=PATH` another that
the main loop reads as the Kindle's main loop reads the node, its frames becoming pen events with
their kernel times; `GRIMOIRE_SIM_STALL_MS=N` then holds the main loop N ms after each read that
brought frames, as an expensive ink frame holds it on the Kindle. `GRIMOIRE_LIVE_PEN_SOURCE=ring`
(any build) makes the pen viewers take the samples from the main loop's ring even where a raw
digitizer is: the measurement's "before".

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
| `pen hover X Y`, `pen leave` | the pen hovering over (X, Y), not touching, and leaving the screen's range (tests/live_check.sh leaves it hovering for Live view's pen frame) |
| `pen line X0 Y0 X1 Y1 STEPS P0 P1 [tx ty]` | a stroke with a pressure ramp |
| `pen curve X Y R STEPS P [TURNS] [tx ty]` | cursive-like loops |
| `eraser line X0 Y0 X1 Y1 STEPS` | the same with the eraser end |
| `side on` / `side off` | hold/release the first side button; existing event sequence is unchanged |
| `side click N [GAP_MS] [PRESS_MS]` | N hovering button clicks; defaults: 150 ms release-to-next-press gap, 80 ms press |
| `side2 on` / `side2 off`, `side2 click N [GAP_MS] [PRESS_MS]` | the second button, enabled by `GRIMOIRE_SIM_PEN_BUTTON2=1` |
| `key CODE MODS [PAD]` | a key press (Linux evdev code, KEYMOD_* bits), a controller's when PAD is 1 |
| `keyup CODE MODS [PAD]` | its release, as input_evdev.c reports one (a key or button let go, or no longer held after a resync) |
| `axis CODE VALUE` | an axis of a controller as Linux's generic driver reports an Xbox One S's (X, Y, RX and RY 0 to 65535, its triggers ABS_Z 2 and ABS_RZ 5 0 to 1023), decided and decoded as input_evdev.c does: a trigger pulled past half its travel is a press of LT (or RT), let back to a quarter or under its release, once each way; the sticks do nothing |
| `hid attach pad\|key\|held NAME`, `hid detach NAME` | a controller (held: one another app holds), or a keyboard or page turner, connects or goes, as input_evdev.c reports it; announced while shortcuts are on. A controller has sixteen buttons from BTN_A, as Linux's generic driver gives one, so one named `Pro Controller` is a Switch-mode one (the Buttons sheet names its buttons B, A, ZL...) |
| `pentap X Y`, `tap X Y` | pen tap / finger tap |
| `pentap panel NAME`, `tap panel NAME` | the same at the centre of the editor panel's button NAME (the panel's names, below), where the panel draws it now. A stroke queued ahead of the line is handled first (the line ends the batch before it looks), and a stale panel is settled as a human tap settles it. NAME not a button now, or no notebook open, fails the line like a failed check: `expect failed (line N): no panel button NAME` (exit 3). Ends the batch, like `tap` |
| `touch down/move/up X Y` | raw finger events |
| `drag pen\|touch X0 Y0 X1 Y1 STEPS` | a slow drag (100 ms per step) |
| `pinch CX CY D0 D1 STEPS [DX DY]` | two fingers around (CX, CY), D0 apart, spreading to D1 while the midpoint drifts by (DX, DY) |
| `finger ID down X Y [MAJOR] [TOOL]`, `finger ID move X Y [MAJOR] [TOOL]`, `finger ID up X Y` | contact ID (1 to 16) of the frame being built: down adds it, move moves it, up lifts it (the next `frame` no longer has it). MAJOR is its TOUCH_MAJOR (default 0), TOOL 0 a finger, 1 a pen, 2 a palm (default 0). Does not end a batch |
| `frame [MS]` | the fingers down now as one contact frame, through touchgest as the Kindle's panel is, at the frame clock (below); its events are queued, then the clock moves on by MS (default 13). Does not end a batch |
| `swipe N X Y DX DY [STEPS] [SEP] [LANDGAP]` | N fingers (2 or 3, fingers 1 to N) centred on (X, Y), SEP px apart across the motion (default 295, 25 mm), landing LANDGAP ms apart (default 30), moving by (DX, DY) over STEPS frames (default 10) 20 ms apart, then lifting one frame (5 ms) apart; one batch |
| `tapn N X Y [SEP] [MS]` | N fingers (fingers 1 to N) in a row centred on (X, Y), SEP px apart (default 150), each kept on the screen, landing one frame each 10 ms apart, still for MS (default 80), lifting one frame each 10 ms apart: `tapn 2 100 100` puts them at x 25 and 175, their midpoint at 100; one batch |
| `rotate N` | the Kindle now turned N quarter turns clockwise (0..3) |
| `sleep`, `wake` | the power service announcing the sleep screen, and its end |
| `wait MS` | let timers run (autosave, toasts, the share server) |
| `hold [PATH]` | keep timers and the server running until the external test driver stops the process, or resumes the script by creating PATH (no spaces, at most 63 bytes); used to attach Live view's readers before drawing in tests |
| `waitbg [MS]` | let timers run until no background job is queued or running (an import, a template's picture, a PDF), at most MS (60000 if not given): what it waited for took as long as the machine let it, so it ends with that job's toast up |
| `shot NAME` | write what the panel shows to `$GRIMOIRE_SHOTS/NAME.png` |
| `expect KEY is\|has\|not VALUE` | check the app's state (see Checks) |
| `power` | a short press of the power button and its release (150 ms) |
| `power down`, `power up` | one half each; the release reports the real time between them, so `power down`, `wait 3000`, `power up` is a 3 s press |
| `power held` | the 4 s mark of a press still down (IN_POWER_HELD alone, what input_evdev.c sends then), so `power down`, `power held` leaves while the key is down (check that with `expect running is 0`) |
| `powerhold` | the press, the 4 s hold and the release at once, with no time passing: Grimoire saves and quits (check that with `expect running is 0`) |
| `powerd STATE` | what the simulated power service reports from now on: `active` (the start), `screenSaver`, `readyToSuspend` or `suspended`; `unknown` or nothing reads as unreadable |
| `clock SECONDS` | the time the clock and battery corner shows, in unix seconds (`clock 1700000000` is 2023-11-14 22:13:20 UTC), kept where the script puts it: it does not run on; `clock real` is the machine's clock again |
| `battery PERCENT [charging]` | the battery that corner shows (a pixel art battery with the percent beside it), 0 to 100, with the bolt over it when `charging` follows; `battery none` is a battery that cannot be read. The simulator starts at 100 and not charging |
| `storage full`, `storage ok` | from now on every file write fails, as on a full userstore, or works again |
| `quit` | quit (end of script also quits) |

The clock and battery corner (docs/KINDLE.md; src status.rs, the Rust build only) reads the simulated
battery every 100 ms in place of the Kindle's every 10 s, so a script sees a change after a short
`wait`. Its time is local time: a script that checks its text sets `# env: TZ=UTC` (y_status_corner.qs does).
It shows on the notebook page only (`status.front`: no Library, no sheet, menu, bar or Pages view in front; the
long-press card, Link to... and Find PICK leave it in front, unless a keyboard they opened covers the corner),
and a script's taps in its corner (`tap`, `touch`, `finger`) toggle it there and nowhere else. The refreshes
it sends show in `display.presents` as `ui` of its own rect (two, apart, when it moves from one corner to
another), and are held back while the pen is near (`pen hover` until `pen leave`), a stroke is drawn or
`display.pen_mode` is 1. A `pentap` leaves the pen near, as the runner never sends the pen's leaving by
itself: follow it with `pen leave`, or with a finger tap, which is never the pen's hand and clears it
(a `pentap` also leaves the driver in pen mode for about 1.5 s, until the page settles).

The simulated power service answers a sleep request (powerButton) with a sleep (goingToScreenSaver)
while it is active or unreadable, and with a wake (outOfScreenSaver) while it is in
`screenSaver`, `readyToSuspend` or `suspended`, so a request sent to a dozing Kindle shows up. Its
state changes only through `powerd`. tests/scenarios/p_power.qs, p_power_locked.qs and
p_power_released_late.qs (a 4.1 s press whose hold never arrived) use these commands, and
v_storage_full.qs leaves with a page that cannot be saved.

The editor panel's buttons, by the names `tap panel` and `editor.panel_at.X.Y` use: `grip`, `collapse` and
`expand` (the collapsed panel is one `expand` button), `tool-picker` (the tool button, or the grid when its pen
is pinned), `tool:NAME` (a pinned pen, NAME as `editor.tool` has it), `eraser`, `undo`, `redo`, `clear`,
`previous`, `pages` (the page chip), `next`, `links` (the Links button), `paper`, `display`, `pen-feel` and `library`. A disabled button
(Undo with nothing to undo, Clear on an empty page, Previous on page 1) has no hit and no name. A scenario
that wants a panel button taps it by name, so that it does not break when the panel gains a button;
`editor.panel_at.1768.594 is undo` upright with no pins (and the other coordinates of tests/scenarios/
y_touch_selftest.qs) says which coordinate is which.

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

`finger`, `frame`, `swipe` and `tapn` are contact-level touch: their frames go through the recognizer
the Kindle's touch panel goes through (touchgest.rs), which makes them one finger's events, a pinch,
a gesture or nothing, and the `touch.*` keys read it. The legacy `tap`, `touch`, `drag touch` and
`pinch` inject their events directly and do not pass through it. Each frame is stamped (its events'
`t_ms` and `k_ms`) with the frame clock: the first `finger` or `frame` line of a batch starts it at
now if that is later than where it was left, and `frame MS` moves it on by MS, so a contact sequence
is timed by its offsets, as `drag` (100 ms steps) and `pinch` (160 ms) are, even when an `expect` or
a `wait` splits it while fingers are down; it never goes back. There is no virtual app clock: a hold
still needs a real `wait`, and so does a tap's refusal when its last finger stays (decided as time
passes, by the frame clock's now). touchgest learns the pen from the pen lines: `pen down` and `move`
put it near and touching (and say where), `pen up` leaves it near, `pen hover` near, `pen leave` gone
at that line's time, and a `pentap` ends with the pen gone at its up (a nib tap lifted away; the
events it queues are as they were). A pen line while fingers are down ends them first, as the pen's
arrival does on the Kindle. So contact-level touch after pen input waits out the pen window: `pen
leave` if the pen is hovering, then `wait 1100` (two fingers are refused for 300 ms after the pen
leaves and kept 40 mm from where it last touched until 1 s; one finger on the notebook's ink, off
the panel, is the writing hand by the same rules, and anywhere else, the Library, a sheet or a
keyboard, a finger is always a finger).

The keyboard's keys in portrait (the Library's keyboard, for every name, at 300 dpi): shift
175 2205, del 1679 2205, and along the bottom Cancel 175 2365, `-` 393 2365, the symbols key 570
2365 (labelled #+=, abc on the symbols page), the accents key 747 2365 (labelled with a and e
acute, abc on the accents page), space 1018 2365, `'` 1289 2365, `.` 1466 2365 and Done 1685
2365 (Cancel 217, the accents key 696, space 1060 and Done 1643, where they were before the
symbols page came, are still on them). On the letters page: y 1725 the digits and y 1885 q to p at x = 133 + 177 n, y 2045
a to l at x = 221 + 177 n, y 2205 z to m at x = 393 + 177 n. On the accents page: y 1725 a e i o u with an acute, then with a grave, at x = 133 + 177 n; y 1885 the
same with a circumflex, then a diaeresis, at the same x; y 2045 n tilde, c cedilla, a and o
tilde, sharp s, oe, ae, o slash, a ring at x = 221 + 177 n; y 2205 the inverted question and
exclamation marks, the angle quotes and the middle dot at x = 570 + 177 n. On the symbols page:
y 1725 the digits, y 1885 `! @ # $ % ^ & * ( )`, y 2045 `? , " : ; _ + = / ~`, each at x = 133 +
177 n; y 2205 `[ ] < >`, the euro, the pound and the degree sign at x = 393 + 177 n. In Spanish
the letters page's y 2045 row is a to l and then n tilde, ten keys at x = 133 + 177 n.
tests/scenarios/u_accents.qs types accented names with them, u_keyboard_symbols.qs a name with
symbols.

These hold in both styles, in English and in Spanish (tests/scenarios/y_all_screens.qs taps them
in each). Upright: the Library's top bar has New at 965 95, Folder 1155 95, Share 1345 95 and
Settings 1535 95. Its five buttons widen together for a longer word, up to 220 px each: New is at
965 only while they are at most 191 px wide, Folder at 1155 up to 198 and Share at 1345 up to 212
(the same sideways), so a language with wider top-bar words moves them (docs/TRANSLATING.md).
Hosted by KOReader they move left, as Back to KOReader takes the room it needs. Settings' rows
are at x 930: Pen sensitivity y 460, Calibrate 620, Display 780, Style 930, Mirror PDFs 1060, the
pen button's row 1190, Turn with the Kindle 1320, Pinch 1450 and Language 1580. Upright from the
Library, the Display sheet's "Sideways, show the whole page" row is 930 1032, its sleep screen row 930 1308, and its clock and battery size cell 472 1626 and corner cell 997 1626, beside its Done at 1460 1626 (in a notebook the sheet has the Pin this page row too, so they are 472 1720, 997 1720 and 1460 1720). The Language list
has Automatic at 930 785, English 930 945, Español 930 1105 and Cancel 930 1265. Pen feel's tabs
are at x = 130 + 177 n, y 289, in the order pencil, mechanical pencil, charcoal, ballpoint,
fineliner, marker, fountain pen, calligraphy pen, highlighter, eraser. Sideways (2480x1860): the
top bar has New at 1585 95, Share 1965 95 and Settings 2155 95; Settings' rows are at x 1240: Pen
sensitivity y 260, Calibrate 425, Display 590, Language 1385; the Display sheet's sleep screen row
is 1240 1099 and its Done 1770 1419; the Language list's Cancel 1240 1055. The first card's menu
button is 431 896; in a notebook's menu Move to folder is 1240 790 and Delete 1240 1110, Delete's
Keep it 1240 845, Move to's Cancel 1240 1055, and a folder's menu has Move folder at 1240 790
and Cancel at 1240 1110. Share's
Done is 2160 1485. In New notebook, Rename is 1950 1424 and the Custom music tile 1800 1115 (its
sheet's Cancel 598 1693); the keyboard's bottom row is at y 1745: Cancel 221, `-` 517, the
symbols key 756, the accents key 995 (928 is on it too), space 1360, `'` 1723, `.` 1962 and Done
2258 (Cancel's 279 is on it too). Pen feel's
tabs are at x = 161 + 239 n, y 289 (the calligraphy pen's 1834). Calibration's corners are 298 223,
2182 223, 2182 1637 and 298 1637, its center 1240 930, touch's skip 1240 1475 and the check's
Cancel 1240 1675. The editor's tool button is 2388 358.

### Phase 6 pen-button recording

`side click 2` is a double-click with 80 ms presses and 150 ms release-to-next-press gaps.
With a double or triple slot bound, follow the final release with `wait 400` before checking
dispatch: the recognizer waits 350 ms. Without either multi-click slot a single click acts at
release. Triple-click requires each next press within 350 ms of the preceding release (the old
three-clicks-within-1.2-seconds rule is retired). `side on`, `wait 1200`, `side off` records a
hovering hold; `side on`, `pen down`, `pen move`, `pen up`, `side off` records a drag. Existing
`side on/off` scripts retain their event sequence; the helper additionally establishes hover.

The card owns a separate recognizer, so its result is `library.record is pattern pen.Button.double`
and its translated name is in `library.record.line`. Tap Use (including `pentap`) to save through
the normal conflict/reassignment flow. Pan/zoom accept only drag, and Hold eraser only hold/drag.
A normal nib tap operates the card's buttons; a recorded drag never clicks them or draws ink.
The editor's last gesture and pending count are `pen.gesture` / `pen.clicks`; recording does not
change the editor's last gesture. `gestures.ACTION` includes pen keys after saving.

`GRIMOIRE_SIM_PEN_BUTTON2=1` adds BTN_STYLUS2 capability and enables `side2` commands. Leave it
unset to check that Button2 lines are carried without live rows. Eraser lines are always carried.
The main scenario owner owns Phase 6 scenarios and the `pass-rust.qs` / `fail-rust.qs` probe checks;
this document does not claim they have run. These helpers test event routing and timing, not
physical button bounce, e-ink appearance or owner success rates.

### Phase 3 touch runner and recording

| Command | Meaning |
| --- | --- |
| `shape NAME X Y SIZE_MM [MS] [ROT] [SEED] [REV]` | Synthetic one-finger frames through touchgest every 13 ms; names from `fingershape_synth::SHAPES`, including `Z`, `N`, `S`, `circle`, `alpha`, `triangle`, `V`, `check`, `zigzag`, `square`. Defaults: 900 ms, rotation 0 degrees, seed 1. A trailing REV token reverses drawing direction. Coordinates are the centre in screen pixels; size is millimetres. |
| `fpath X1 Y1 X2 Y2 ... [MS]` | Exact screen-pixel polyline through touchgest; at least two points, default 600 ms. An odd final argument is the duration. |
| `trace FILE` | Replay `F` contact frames and `P` pen states on their kernel-time deltas using the shared trace parser; advances ticks between frames. Recorded `E` decisions are evidence, not injected actions. |

`GRIMOIRE_SIM_TOUCH_SLOTS=N` selects the simulated panel's slot capability.
`GRIMOIRE_TOUCH_TUNE=v1` selects the old tuning table; otherwise the simulator and runner use
the provisional `TG_TUNE`. The scenario harness clears both hooks between runs. Synthetic
shapes and `tests/fixtures/touch/synthetic.txt` are regression inputs, not owner evidence.
The trace contains no pen coordinates: replay cannot measure hover-distance acceptance.

The Settings bottom row is KOReader | Controller | Gestures | Updates | Done, in either
orientation. These interior tap points were checked with `library.ui_at` in English and
Spanish, Gothic and Classic, on the 1860 x 2480 panel (2480 x 1860 sideways):

| Target | Upright | Sideways |
|---|---|---|
| Controller | 580, 1930 | 863, 1695 |
| Gestures | 890, 1930 | 1170, 1695 |
| Updates | 1200, 1930 | 1470, 1695 |
| Settings Done | 1550, 1930 | 1820, 1695 |
| Record Use (when enabled) | 500, 1650 | 810, 1340 |
| Record Try again | 930, 1650 | 1240, 1340 |
| Record Cancel | 1360, 1650 | 1670, 1340 |

The three sample slots have interior centres at x = 630 / 930 / 1230, y = 1110 upright,
and x = 940 / 1240 / 1540, y = 800 sideways. They display samples and have no hit action.
Use `library.ui_at.X.Y` to check a candidate hit before depending on it. Its
values include `controller`, `gestures`, `updates`, `done`, `gestures.master`,
`gestures.group.NAME`, `gestures.row.N`, `gestures.reset`, `gestures.reset.confirm`,
`gestures.reset.cancel`, `record.use`, `record.retry` and `record.cancel`. This is a hit-test
probe, not a coordinate locator; existing unrelated coordinates above remain as documented.

**Recording a gesture in a scenario.** Open Settings > Gestures and a group; verify each
candidate tap with `library.ui_at.X.Y`. Tap an action, expect `library.gestures.learning`
to name it and `library.record` to be `wait`. A paired row continues with the second member
after saving the first. Use `tapn`/`swipe` for a pattern; expect `pattern touch.2.tap` (for example), then tap
Use. For a shape, use `shape Z ...` with distinct seeds for repeated samples, checking
`shape 1`, then `ready`; a third sample is optional. Use runs the slip/conflict checks before
saving, or the third sample runs them immediately. If the first Use discovers that another
action owns the shape, the card names it and a second Use confirms the move.
Check `library.record.line` for the refusal or reassignment reason, and check
`gestures.ACTION`, `gestures.shape.ACTION` and the file after success. Try again restarts the
attempt; Cancel leaves the bindings unchanged. To exercise a failed save, use the main
scenario owner's fixture and assert that the live binding and card survive unchanged.

The host tool uses the same recognizer without opening the app:

```sh
cargo run --manifest-path rust/Cargo.toml --bin tgstudy -- tests/fixtures/touch/synthetic.txt --tune new
# An owner trace, once supplied:
# tgstudy TRACE --tune new --set swipe_mm=16 --sweep cos_min=0.7:0.8:0.05 --blocks BLOCKS.txt
```

`tgstudy` splits at hands-off gaps of at least 4 s (the study asks for 5 s), prints block sizes,
session measures and outcomes, intended-fire counts and wrong/negative fires. Check the split
before interpreting a report. `--blocks` supplies names in order. No coordinate or success-rate
claim from synthetic replay establishes the owner study or the finger-shape freeze gate.

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
| `power.pokes` | how many times the app told the power service someone is here: at most once a minute, for pen, touch, or a key while shortcuts are on (the runner answers this one too) |
| `main.longest_unpumped_ms` | the longest stretch between two reads of input or pump calls since this key was last checked, presents and file saves left out: `ok` under the runner's bound of 250 ms (host time of a debug build: it catches a missing pump, not the Kindle's ~100 ms), else the stretch in ms. The check starts the count again, so `expect main.longest_unpumped_ms not -1` only starts it (the runner answers this one too) |
| `touch.contacts` | contacts down now, as touchgest follows them (the contact lines' fingers; the runner answers the `touch.` keys) |
| `touch.class` | what they are doing: `none`, `one` (one finger's stream), `landing` (two fingers, not committed yet), `zoom`, `pan`, `swipe`, `tap` (a two-finger tap waiting for its last finger), `suppressed` (every contact waits to lift, or the pen's hand is held) |
| `touch.suppressed` | `1` while contacts wait to lift (or a lone finger is held: the pen's hand, one beside a resting palm), else `0` |
| `touch.gesture` | the last gesture fired, as gestures.ini names it after `touch.` (`1.up` through `1.right`, `2.up` through `2.tap`, and `3.up` through `3.tap`); empty before any |
| `touch.reject` | the last refusal: `pen`, `apart`, `straight`, `short`, `direction`, `hand`, `spread`, `late`, `three`, `keyboard`, `off` or `slow`; empty before any, and a gesture fired clears it |
| `touch.hand` | why the last session's lone finger on the notebook's ink was taken for the pen's writing hand and sent nothing: `touching` (the pen touched), `near` (it hovered, with no fresh hover sample 60 mm away) or `gone` (it had left under a second before, the finger near where it last touched); empty for a finger let through, and from the next session's first contact on (grimoire.log says the same at the lift: `touch: a finger on the page was the pen's hand: ...`) |
| `pump.calls` | how many times long work called the work pump (`work_pump`, common.h: on the Kindle it reads the pens meanwhile) since this key was last checked, so `expect pump.calls not -1` only starts the count (the runner answers this one too) |
| `keyboard.draws` | how many times an on-screen keyboard was drawn whole (`keyboard_draw`, keyboard.h) since this key was last checked, counted the same way (the runner answers this one too) |
| `screen` | `library`, `editor`, `penset` (pen feel), `calib` or `share` |
| `toast` | the toast showing, empty if none |
| `toast.rect` | where the visible toast is, `x y w h` in screen pixels, empty if none |
| `asleep` | `1` while the sleep screen is up, else `0` |
| `asleep.mode` | while asleep, which sleep screen is up, numbered as `settings.sleep_mode` after fallback/override (native PIN forces `2`; unavailable wallpaper uses `0`) (hosted by KOReader, mode 2 reads `0`); empty while awake |
| `host` | `koreader` when started as KOReader's Grimoire plugin starts it (`GRIMOIRE_HOST=koreader`), else empty |
| `language` | the language in use, `en` or `es` |
| `rotation` | the quarter turns applied, 0..3 |
| `size` | the screen, `1860x2480` or `2480x1860` (`1986x2648` or `2648x1986` under `GRIMOIRE_SIM_SIZE=1986x2648`) |
| `settings.KEY` | the setting as settings.ini stores it, for `theme` (`gothic` or `classic`), `invert`, `mirror_pdf`, `follow_rotation`, `pinch_zoom`, `side_button` (0 nothing, 1 eraser, 2 highlighter, 3 white ink), `sleep_mode` (0 the page, 1 a pinned page, 2 the Kindle's screensaver, 3 KOReader wallpaper), `sleep_minutes`, `pen.valid` and `touch.valid` (1 once calibrated); `language` (`auto`, `en` or `es`, and `auto` for a value this version does not speak, which settings.ini keeps as it was); `pen.panel` and `touch.panel` (the panel the calibration was made on, `WxH`, empty when not recorded) |
| `settings.TOOL.PARAM` | a tool's pen feel as settings.ini stores it, four decimals: TOOL as for `editor.tool`, PARAM a slider's settings.ini name (`size_min_mm`, `size_max_mm`, `opacity_min`, `opacity_max`, `pressure_floor`, `pressure_ceiling`, `pressure_gamma`, `tilt_min_deg`, `tilt_max_deg`, `tilt_size`, `tilt_opacity`, `grain`, `smoothing`), e.g. `settings.fountain.pressure_gamma` |
| `light.intensity`, `light.warmth` | the front light (the simulator starts at 12 and 8) |
| `editor.page`, `editor.pages` | the open page (from 1) and the page count, 0 with no notebook open |
| `editor.tool` | the pen's tool by its settings.ini name: `pencil`, `mech`, `ballpoint`, `eraser`, `highlighter`, `fountain`, `fineliner`, `marker`, `charcoal` or `calligraphy` |
| `editor.zoom` | the zoom with two decimals: `1.00` is the usual view (the page's width fitting the screen's: 100% upright for a notebook of this screen's size; sideways the whole page while `landscape_fit` is on, its default) |
| `editor.zoomed` | `1` while the view is zoomed or panned, else `0` |
| `editor.back` | the back list (Back to the page before): `DEPTH TOP`, how many pages it holds and the page number of the newest (`2 3`), `0 0` when it is empty or no notebook is open. Pages deleted since are not counted |
| `editor.scrolls` | `V H`: whether the page scrolls on the vertical and the horizontal axis now (`1 0`), what touchgest.rs reads to tell a two-finger pan from a swipe: `0 0` at the whole page, with a surface in front that does not pan (the Pages view, a sheet), or with no notebook open |
| `editor.areas` | `0` while the eye (the link areas, `settings.link_areas`) is off, else how many dotted boxes it draws over the open page (one for every link whose ink is on it, up to 64) |
| `editor.find` | the Find sheet (Go to page or name; the bar's Go to, the `go-to-page` action and the Pages view's Go to page... open it): `0` while it is closed, `go` while it is open, `pick` while it picks where a new link goes (Link to...'s Search) |
| `editor.find.rows` | how many rows its card shows now (five upright at most, three sideways), 0 with nothing typed or while it is closed |
| `editor.find.N` | row N of the card as drawn, from 1: `Go to page 3`, a name (`Árbol`), `There is no page 9 (5 pages).` or `No page names match.` (the last two are lines, not buttons); empty past the last |
| `editor.bar` | the Links bar: `0` while it is closed, `open` while it is up, `peek` while it shows another page above itself (an entry tapped, or "Back to ..."): the open page, its history and `notebook.last_page` stay the page the bar began on until that page is stayed on |
| `editor.bar.entries` | how many entries the bar lists (one per place the open page's links go; two to one page are one entry, counted), 0 while it is closed |
| `editor.bar.N` | entry N of the bar (from 1, in reading order): `p3` for page 3, `p3 x2` when two links go there, `index` for the Index, `missing` for a page that is gone; empty past the last |
| `editor.bar.pager` | the bar's screenful out of how many (`2/3`), six entries a screen; empty with one screenful or while it is closed |
| `editor.bar.peek` | the page number the bar shows above itself, `0` when it shows none (also while it is closed) |
| `editor.canvas_cache` | what the bar's page canvas cache did since the bar opened, `HITS MISSES ENTRIES`: taps that found their page drawn already, taps that did not (it was read and drawn then, or only partly drawn ahead), and the pages it holds now (the pages the entries go to are got ready, a 160-row band a tick, while nothing touches the screen: `1 0 2`) |
| `editor.bar.rect` | the bar's card, `x y w h`: upright 20 from each side and 20 above the bottom edge, 316 + 140 R px tall (R rows of chips, 1 to 3; two columns upright, three sideways, two rows at most), and 60 more for its second header line the first three times it opens; empty while it is closed |
| `editor.scale` | screen px per page px, two decimals: `1.00` for a notebook of this screen's size upright, `1.07` for an 1860x2480 notebook on a 1986x2648 screen, `0.94` the other way; `0.00` with no notebook open |
| `editor.page_size` | the open page's own size, `WxH`: its notebook's, so a page that came at the screen's size instead would show here; empty with no notebook open |
| `editor.stroke_start` | where the open page's newest stroke starts, in page px rounded, `x y`; empty with no stroke |
| `editor.stroke_points` | sample count of the newest stored stroke, zero with no stroke |
| `editor.stroke_tip` | the newest stroke's tip variant (src/brush.h: 0 its tool's first tip, 1 the vertical highlighter); empty with no stroke |
| `editor.modal` | what is in front of the page: `none`, `find` (the Find sheet), `card` (the long-press card, or Rename's keyboard over it), `linkto` (Link to..., or New page...'s keyboard over it), `links` (the Links bar), `tools` (the tool picker), `paper` (Paper for this page), `music` (Custom music), `display` (the Display sheet) or `pages` (the Pages view) |
| `editor.layers` | whether the screen is what its layers make of it: `ok`, or `X Y`, the first pixel that differs from the screen composed again whole (the page, the panel and then each menu that is up in its order) by more than 64 gray levels (the edges of a frame drawn again over itself differ by a little). A panel drawn over a sheet, the Find sheet's keyboard or the Links bar shows here. The check leaves the screen as it composed it, so it says what was wrong once |
| `editor.panel_at.X.Y` | the name of the panel's button at screen point (X, Y) (the panel's names, above; between two buttons or in the frame's margin beside one, the nearest, as a finger there takes it), empty for none (a greyed-out button), after settling a stale panel as a tap would: the way to check what a raw coordinate hits. Empty while no notebook is open |
| `editor.strokes` | strokes on the open page (a stroke counts once the pen lifts) |
| `editor.erase_tiles` | `DRAWN BOXED`: the 16 px tiles the stroke eraser drew the page again in since the editor began (only those the strokes it took off drew into, page_render_tiles), and the tiles of those strokes' boxes, all it drew before 2.0.0-rc.12 (`391 11011`) |
| `editor.owner` | who has the contact now: `none`, or the owner and the input that began it, as grimoire.log's contact lines name them (`draw pen`, `lasso pen`, `link finger`, `linkto finger`, `outside pen`, `panel finger`, `strip finger`, `pen-drag pen`...). A down from the input that still owns a gesture ends that gesture as a cancel (its lift was lost: a stroke kept as drawn, a lasso back to the strip, a press let go) before it starts its own; legacy `touch down` twice, or `pen down` twice, with no lift between, makes one (g_gest_15b_lost_lift) |
| `editor.unsaved` | `1` while the open notebook has changes not yet written, else `0` |
| `editor.loaded_pages` | pages currently loaded in the editor; opening or paging the number grid must not increase this |
| `native_pin` | native PIN ownership path is active (`GRIMOIRE_PASSCODE=enabled`) |
| `input.yielded` | pen and touch have been handed to the native UI for sleep/unlock |
| `sleep.wallpaper` | KOReader wallpaper is decoded and cached |
| `editor.ticked` | selected pages in management (tap tile selects, double tap opens) |
| `editor.pages.grid` | actual column and row counts of the current Pages geometry |
| `editor.pages.context` | held page number while its context menu is open, else `0`; its rows are `tap panel pages.open`, `pages.link`, `pages.rename` (Page name for that page, over the grid), `pages.delete` (the ticked pages when the held one is ticked, else that page alone) and `pages.cancel` |
| `editor.pages.cached`, `editor.destination.cached` | how many decoded page previews the Pages view, or Link to...'s notebook picker, keeps for the screenful on show |
| `editor.screen_composed` | whether the current screen matches a fresh complete composition, without repairing stale pixels |
| `editor.map.visible` | sorted page IDs actually shown in the current graph view |
| `editor.map.view` | `all` or `around`, after `fit ` when a pan or pinch has moved the view (the next View press fits it again) |
| `editor.map.stats` | graph nodes (All), current notebook orphans, valid links, current notebook pages, library page total |
| `editor.page_name`, `editor.index_rows` | the open page's section name (empty if unnamed), and the notebook's named-section count |
| `editor.pages_tab` | `pages` or `index`, the Pages view's selected tab |
| `editor.tile.N` | the label under page N in the open Pages view, with its book-page label separated by a vertical bar when present; empty while Pages is closed |
| `editor.keyboard`, `editor.text` | the keyboard that is up over the page and the text it holds, both empty with none: the Pages view's (its title `Page name`, the Index's naming keyboard, as Name this page and the `name-page` shortcut open it; `Go to page` while a link's page is being picked, where Go to page... is still a number pad) the Find sheet's (`Page number or name`) or New page...'s (`Name the new page`) |
| `live.on`, `live.viewers`, `live.pairing` | Live view: `1` while it is on, how many computers watch, `1` while its pairing card is the screen |
| `live.chip` | where Live view's chip is, `x y w h`, while it is on; else empty. On a notebook page it is beside the editor's panel (level with its header, on its left, or on its right at the screen's left edge) and moves with it; with a menu, sheet, bar or the Pages view in front it is empty; on the other screens it is the top right corner |
| `editor.picking` | `new` while the Pages view picks where a new link goes (Link to...'s Choose from the pages...), else empty (it no longer reads `had`: a link held on opens the long-press card) |
| `editor.link_mode` | `1` while the link-making strip is up (after Add a link: armed, or drawing the lasso), else `0` |
| `editor.strip` | `new` while the strip is up for a new link (armed or drawing the lasso), `redraw` while it is up for the card's Redraw the area, else `0` |
| `editor.lasso` | how many points the lasso being drawn holds (at most 4096), `0` otherwise |
| `editor.link_area` | the area a new link waits to be given a target for (from the lasso's lift to the pick), `x y w h` in page px; empty otherwise |
| `editor.undo_kind`, `editor.redo_kind` | the kind of step an undo (a redo) of the open page would apply: `add`, `erase`, `paper`, `clear` or `link`; empty for none |
| `editor.ui_at.X.Y` | the name of what the long-press card or link making shows at screen point (X, Y): on the card `card.rename`, `card.retarget`, `card.redraw`, `card.delete` (a disabled row has none); `strip.cancel`, or on Link to... `linkto.chip.N` (N from 1 across its screens), `linkto.search`, `linkto.pages`, `linkto.cancel`, `linkto.prev`, `linkto.next`; empty for none. The way to check a raw tap on it, as `editor.panel_at` does for the panel |
| `editor.linkto` | `pick` while Link to... is open (up, or hidden under New page...'s keyboard or the Pages view it opened), else `0` |
| `editor.linkto.rect`, `editor.linkto.dock` | its card, `x y w h`, and `top` or `bottom`; both empty while it is closed |
| `editor.linkto.chips`, `editor.linkto.N`, `editor.linkto.screen` | how many chips it has on all its screens; chip N from 1: `new`, `index`, `recent p<page>`, `p<page>` (an Index entry), then ` current` on a marked target, empty past the last; its pager's screen, `S/T` |
| `editor.recent` | the targets links were made to this session (Link to...'s Recent targets come from them, then from the back list), as page numbers newest first (`3 2`); empty for none |
| `editor.link_card` | `1` while the long-press card is up, else `0` |
| `editor.link_card.rows` | its rows in order, space separated (`rename retarget redraw delete`), a disabled one with a leading `-`; `readonly` for a read-only page's card; empty while it is closed |
| `editor.link_card.title`, `editor.link_card.rect`, `editor.link_card.links` | its title as drawn (`Árbol`, `Page 2`, `The Index`, `Page not here`), its card `x y w h`, and how many links it acts on; empty (or `0`) while it is closed |
| `editor.link_card.border` | how many boxes carry the card's heavy border now (it stays while Change where it goes... or Redraw the area goes on), `0` for none |
| `editor.kb_dock` | where the keyboard in front is docked (Find, Rename, New page...): `top` or `bottom`; empty with none |
| `editor.links` | the open page's links: how many it has, then how many of them have a box: an area, or for a link without one (beta.5's) its ink on the page (`2 1`) |
| `editor.link.N` | where the open page's link N goes (from 1, in `links.ini`'s order): `index`, `p<page id>`, or another version's target; empty past the last |
| `editor.link.N.area` | the open page's link N's area (links.ini's `area.` line), `x y w h` in page px; empty for a link without one (a beta.5 link not migrated yet, or a dormant one) and past the last |
| `editor.migrate_left` | how many pages of the open notebook have a link without an area and have not been tried this session: the idle ticks migrate one a tick (a page is tried once, at its first read or by the idle ticks) |
| `editor.links_dirty` | `1` while migrated link areas wait to be written to links.ini (after a 2 s pause, or 2 s after a failed write), else `0` |
| `editor.paper` | the page's paper as its number in src/paper.h (1 medium rule, 2 wide rule, 3 sheet music, 6 medium rule without a margin, 8 grand staff...), `custom` for a Custom music layout, `none` with no notebook open |
| `editor.panel` | `expanded` or `collapsed` |
| `editor.hold_eraser` | `1` while a controller's Erase while held is down (from its press to its release), else `0` |
| `editor.music` | the Custom music layout on the Custom music sheet while it is open, else the page's if its paper is custom, else empty: the systems per page, a colon, then the parts top to bottom (`treble`, `bass`, `alto`, `tenor`, `drums`, `plain`, `tab4` to `tab7`, `lyrics`), e.g. `4: treble tab6 bass` |
| `editor.music_max` | while the editor's Custom music sheet is open, the most systems its layout fits on the page it is for (that page's height), else empty |
| `library.sheet` | what is open over the cards: `none`, `new` (New notebook), `settings`, `more` (a notebook's menu), `folder-more` (a folder's menu), `move` (Move to), `delete`, `keyboard`, `display`, `music`, `koreader`, `language` (Settings > Language's list), `shortcuts` (Settings > Controller), `buttons` (its Buttons sheet), `pair` (its Pair sheet), `updates`, `gestures` or `about` (What's new, Rust only) |
| `library.learning` | the action the Buttons sheet waits for a press for (`undo`, `next-page`), else empty |
| `library.buttons.ACTION` | the Buttons sheet's buttons for ACTION (`library.buttons.undo`): as the files name them (`South, Up`), or as printed while the first controller connected is a Switch-mode one (`B, Up`); `none` for none |
| `library.buttons_group` | the Buttons sheet's group open: `editing`, `tools`, `pages`, `index` (the Index's actions, Rust only) `menus`, `screen` or `system`; empty on its list (or with no Buttons sheet) |
| `library.pair` | the Pair sheet: `scanning`, `pairing` or `idle`, then how many controllers it found and how many are paired (`idle 2 1`) |
| `library.bluetooth` | the Bluetooth helper as Grimoire sees it: `stopped`, `starting`, `owned` (Grimoire's), `external` (someone else's, or the test's stand-in), `failed` or `stopping` |
| `gestures.ACTION` | binding for ACTION: touch key (`touch.1.right`, `touch.2.down`), `shape` for a known/carried newer shape, or pen key (`pen.Button.double`); built-ins in FORMAT.md; empty for none. |
| `pen.gesture` | last editor-recognized button gesture: `Button.tap`, `Button.double`, `Button.triple`, `Button.hold`, `Button.drag` (or `Button2.*`); empty before any recognition |
| `pen.clicks` | first button's pending multi-click count |
| `editor.hold_eraser` | `1` while the momentary eraser is held, otherwise `0` |
| `gestures.shape.ACTION` | sample count for ACTION's recorded shape; empty for none |
| `gestures.shape_count`, `gestures.refused` | number of recorded shapes, and lines refused at the last map load |
| `library.gestures_group` | open group (`editing`, `tools`, `pages`, `index`, `menus`, `screen`, `system`), else empty |
| `library.gestures.N` | 1-based visible row N's text as drawn at the root or in a group, in the current language |
| `library.gestures.learning` | action being recorded, or `0` |
| `library.record` | `0`, `wait`, `pattern touch.N.KIND`, `shape K` (sample count), `checking` or `ready` |
| `library.record.line` | record card's progress/refusal line, empty without a card |
| `library.ui_at.X.Y` | semantic name of the Library hit at screen pixel X,Y, empty for none; see recording workflow. A notebook's menu has `more.move`; a folder's `fmore.rename`, `fmore.move` and `fmore.remove` (Cancel is `done` in both); Move to `move.top`, `move.to.PATH` (a folder by its path, `move.to.School/Math`), `move.new`, `move.prev` and `move.next` |
| `library.about.page`, `library.about.pages`, `library.about.full` | What's new while it is open (else `0`): the page shown (from 1) and the pages the notes take on the sheet, and `1` once Read all shows the whole notes (the card: `0`). Its hits: `about` (the bat and the name beside it, at the top level; in a folder `back` is where the bat is), `about.close`, `about.hide`, `about.more` (Read all), `about.prev`, `about.next` |
| `settings.whatsnew_hide` | `1` while What's new's Don't show this again is ticked |
| `settings.rotation_lock` | `0` follows the sensor, `1..4` locks rotation to stored value minus 1 |
| `touch.tune` | `v1` or `new` (provisional `TG_TUNE`) |
| `fshape.last` | `<fired\|reject\|refused\|cancel> <action\|reason> <d1> <thr> <d2>`, empty before any outcome |
| `fshape.fired` | fired count since last read; reading consumes the count |
| `fshape.state` | `idle`, `pending`, `drawing`, `dead` |
| `fshape.trail` | current trail rectangle `x y w h`, empty for none |
| `fshape.armed` | `1` when page capture may arm, otherwise `0` |
| `pads.ACTION` | the controller buttons that do ACTION (`pads.undo`, `pads.tool:eraser`), as the files name them, comma separated; empty for none |
| `settings.controller` | `1` when the Bluetooth helper starts with Grimoire |
| `settings.updates` | Settings > Updates' channel: `0` by the version, `1` releases only, `2` betas too |
| `settings.status_on`, `settings.status_size`, `settings.status_corner` | the clock and battery corner as settings.ini stores it: `1` while it is on, its size, 0 small, 1 medium, 2 large, and its corner, 0 top left, 1 top right, 2 bottom right, 3 bottom left |
| `settings.gestures`, `settings.landscape_fit`, `settings.hints`, `settings.link_areas` | the gestures' settings as settings.ini stores them: `1` while on, `landscape_fit` on by default, `hints` the one-time tips shown as a bitmask (FORMAT.md), `link_areas` `1` while the eye is on |
| `notebook.last_page` | the open notebook's `last_page` plus 1: the page its notebook.ini reopens it on (a peek of another page never changes it); `0` with no notebook open |
| `status.shown` | `1` while the panel holds the clock and battery corner, else `0`. A tap in the corner or the `status-toggle` shortcut turns it on at once (`settings.status_on`), but it is shown only on a notebook page with nothing in front of it (`status.front`), and only once its refresh has been sent: not while the pen is near, nor while asleep |
| `status.text`, `status.rect` | what the corner says (`Nov 14, 22:13  87%`: the time as the Library's cards write it, then the percent, which is drawn right of the battery; the time alone when the battery cannot be read) and where it is, `x y w h` in screen pixels; both empty while it is not shown |
| `status.icon`, `status.fill` | the battery's pixel art, `x y w h` (18 by 9 units, a unit about a tenth of the text's height: 54 by 27 px at the small size), and how many of its 12 columns of charge are lit (`10` at 87%, `1` for any charge, `0` at 0%); empty while it is not shown or has no battery |
| `status.charging` | `1` while the corner shows its bolt, `0` while it does not; empty while it is not shown |
| `status.zone` | the corner's square, `x y w h`, where a finger's tap finds it: 14 mm square in the corner `settings.status_corner` names (`0 0 165 165` at 300 dpi), and the indicator's box is part of the corner too while it is shown; a tap also finds the box's place in that corner with 4 mm around it, shown or not (`status_hit`) |
| `status.front` | `1` while the notebook's page is in front, the only place the corner shows or takes a tap, else `0` |
| `library.update` | the Updates sheet's check: `idle`, `checking`, `newer VERSION`, `newest VERSION` or `failed WHY` (`network`, `no-kpm`, `feed`, `failed`) |
| `update.requested` | what Update now asked the launcher for (`preview` or `stable`), empty for nothing |
| `library.hint` | `1` while the empty Library's (or an empty folder's) two lines show ("No notebooks yet" or "This folder is empty", then "Tap New to start one."), `0` when there are cards, and under every sheet but the keyboard |
| `library.folder` | the open folder, empty at the top level |
| `library.folders` | every folder by its path, as Move to lists them, joined by `, ` (`Archive, Archive/School`); empty with none |
| `library.mirror` | the mirror PDFs (`PDFs/`) by directory: `.` for the top and each directory in it, sorted, with how many PDFs it holds (`. 0, Archive_School 1`), so a directory left behind shows as `School 0` |
| `library.items` | the cards in the open folder, on every page of cards: at the top level its folders and the notebooks in no folder |
| `library.page`, `library.pages` | the page of cards (from 1) and how many there are |
| `library.text` | the keyboard's text while it is open |
| `library.paper` | the paper picked in New notebook, numbered as for `editor.paper` |
| `library.music` | as `editor.music`, for New notebook's Custom music sheet and its picked paper |
| `display.ink` | `1` while a pen stroke is marked live ink (`display_live_ink`), else `0` |
| `display.ink_frames` | fast refreshes submitted while a stroke was still active; used to check input backlogs |
| `display.pen_mode` | `1` while the app asks for MTK pen mode (the editor's ink strokes only), else `0` |
| `display.last_fast` | `ink` if the last fast refresh was live ink, `wait` if a color panel would have waited for the driver's read after it (docs/DESIGN.md), empty before any |
| `display.last_fast_rect` | where the last fast refresh was, `x y w h` in screen pixels, empty before any |
| `display.presents` | the refreshes sent since this key was last checked, in order, each its mode (`fast`, `ui` or `full`) and `x y w h`, joined by `, ` (`fast 50 1510 1760 120, fast 138 1970 167 150`), with `, and N more` past the eighth; empty for none. The check starts the count again, so `expect display.presents not -1` only starts it |

The `editor.` and `library.` keys answer on any screen: the editor's describe the open notebook,
also while pen feel is in front. The `display.` keys are the simulated panel's (the runner answers
them, like `running` and `watching`); tests/scenarios/y_live_ink.qs uses them, on a fresh root at
1860x2480: ink and the eraser in the editor, the pen feel pad and the calibration's check stroke
are live ink, and a pressed button or a dragged slider is not. tests/scenarios/j_pages_keyboard.qs
holds the Pages view's keyboard to the Library's with `display.presents` and `keyboard.draws`: a
key's press refreshes the text line and then the key, each on its own (`fast 50 1510 1760 120,
fast 138 1970 167 150` for a), and draws the keyboard once, the pen's samples while it rests on the
key draw nothing, the release refreshes the key alone, and a pause gets one GL16 of the keyboard
(`ui 20 1480 1820 980`); a settle due on the page draws nothing under the view.

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
v_shortcuts_awake and v_shortcuts_disabled set `GRIMOIRE_SLEEP_MINUTE_MS`, y_status_corner sets `TZ=UTC`,
y_koreader_hosted sets `GRIMOIRE_HOST` as the KOReader plugin's start does and
y_koreader_not_hosted sets it empty, y_language and z_language_kept set `GRIMOIRE_LANG`,
y_pagesize_1_ksc sets `GRIMOIRE_SIM_SIZE=1986x2648`, and y_all_screens sets `GRIMOIRE_CHECK_FIT`
and a made-up `GRIMOIRE_SIM_ADDRS`). The rule first unsets `GRIMOIRE_HOST`, `GRIMOIRE_PASSCODE`,
`GRIMOIRE_SIM_ADDRS`, `GRIMOIRE_HTTPD_DELAY_MS`, `GRIMOIRE_BG_DELAY_MS`, `GRIMOIRE_SIM_PRESSURE`,
`GRIMOIRE_SIM_STATS`, `GRIMOIRE_SIM_SIZE`, `GRIMOIRE_CHECK_FIT`, `GRIMOIRE_LANG`,
`GRIMOIRE_L10N_DIR`, `GRIMOIRE_SIM_PEN_BUTTON2` and `GRIMOIRE_SLEEP_MINUTE_MS` (tests/share_check.sh does too), so only
`# env:` lines set them, and runs every scenario in English (`GRIMOIRE_LANG=en`) unless its
`# env:` line names another language: the scenarios tap by coordinates and check English text. A
`# pdfs: N` line wants the run to leave exactly N PDFs in its root's PDFs/ folder
(v_leave_from_editor: 2, y_koreader_hosted: 1).
With `GRIMOIRE_TEST_PORT` set, the rule writes
`http_port=` that port into every scenario's settings.ini before the run (over a copied root's
too), so the share servers of checks running in several worktrees never collide on 8765.
`# settings: key=value` lines are appended to the scenario's settings.ini before the run (after
the port), for settings that have no control on screen yet. These header lines are read by the
Makefile rule only: running `grimoire-headless` by hand skips them like any comment. A run that
leaves a file half-written (`*.tmp`) fails. Every scenario runs, then the rule fails naming those
that did not pass. Last it checks the runner itself: `tests/selftest/pass.qs` must pass every
check (exit 0) with its notebook's PDF written, and in `tests/selftest/fail.qs` every line but the
comments, waits and taps must fail and be reported by its line number (exit 3). Then the same for
`tests/selftest/pass-rust.qs` (exit 0) and `tests/selftest/fail-rust.qs` (exit 3, its lines
reported), which use the commands and probes only the Rust build has.

A scenario with a `# rust-only` line is skipped when the rule runs against the C build (`make
c-check`, which sets `SCEN_SKIP_RUST_ONLY=1`): it uses commands, probes or behaviour only the Rust
build has, unless `tests/c-scenarios/` holds a C variant of it (the script as it was before its panel taps
became names), which runs in its place: the two roots whose data others use (`b_library`, which
`share_check.sh`, the exporter check, `d_persist` and `x_turn_calibrated` start from, and `z_import_1_startup`)
have one, and a variant must be kept in step with its original. A scenario whose `# root:` was skipped is skipped
with it (it would copy a data root that was never made), even if it has no `# rust-only` line itself. The two rust selftests are skipped there too, and
so is `make l10n-check`'s run of tests/scenarios/y_all_screens.qs, which visits the Rust build's screens.

### Making links

Add a link (`key 38 0`, the Index's Add a link, or the Links bar's New link) shows the strip along the bottom
(upright at y 2330 to 2460, its Cancel 260 x 100 at x 1565, centre 1695 2395), and the next pen contact on the page
draws the dashed lasso around the link's area: a `pen line` (or `pen down`, `pen move` lines and `pen up`) is that
lasso, never ink, whatever the tool. Its box is the area (`editor.link_area`), unless it is under 5 mm both ways (a
`pentap`): then the strip stays and says so. At the lift Link to... opens: "Where does the link go?" with the chips
New page..., The Index, up to four recent targets and the Index's entries, and a tool row of Search, Choose from the
pages... and Cancel. It is the Links bar's frame, 316 + 140 R px tall for R rows of chips (two columns of 878 x 124
upright, at x 479 and 1381, rows 140 apart from its top + 214; three of 784 sideways), docked at the bottom 20 px
above the edge, or at the top when the centre of the area is in the lower 45% of the screen (y 1364 or more upright),
its top 8 px below the clock corner's zone (173 upright at 300 dpi with the status bar off). Its tool row's centre is
90 px above the card's bottom: upright at the bottom y 2370, Search x 330, Choose from the pages... 930, Cancel 1530;
sideways at the bottom y 1750, x 433, 1239 and 2045. Choose from the pages... opens the Pages view to pick
(`editor.picking is new`); its Back comes back to Link to.... Search opens the Find sheet in pick mode
(`editor.find is pick`; a number's row reads `Link to page N`): a row or Done links there with no page change, and
its Cancel or a tap outside comes back to Link to.... Docked at the bottom its rows and keys are where Go mode has
them; at the top (`editor.kb_dock is top`) its keys sit 1307 px higher and the rows card starts 16 px under the
keyboard (y 1169, its first row's centre 1247). New page... opens a keyboard (`Name the new page`)
docked as Link to... is: at the top its keys sit 1307 px higher than the coordinates below (Done 1643 1058). A
scenario checks each raw tap on the strip and Link to... with `editor.ui_at.X.Y` first.

grimoire.log follows link making with a line at each change of its state (`link: armed -> lasso, the pen owns
lasso`; the states are off, armed, lasso, pick and naming), and says when a contact is ignored because another
owns the screen (`editor: a finger down ignored: the pen owns lasso`) or a lift was lost (`editor: a pen down
while the pen owned lasso: its lift was lost`), 40 such lines a minute at most. The pen's button taking the pen
over for a drag in the middle of the lasso (a drag bound to Pan, tests/fixtures/penbutton/pan) ends the lasso:
the strip stays for another, and the next stroke is the lasso again, never ink (g_gest_15_link_then_finger).

A finger held on a link (`touch down X Y`, `wait 800`, `touch up X Y`) shows its heavy border (four DU strips,
each its own `fast` present in `display.presents`) and then the long-press card beside it (one `ui` of its rect):
780 px wide, its rows 732 x 118 from 134 px below its top, 130 px apart (centres at top + 193, 323, 453, 583, x at
its left + 390). It goes right of the border, else left, below, above, never over the clock's corner zone, and off
the panel when it can. Its rows: Rename in the Index (Add page N to the Index... for an unnamed page; none for the
Index), Change where it goes..., Redraw the area and Delete link. Rename's keyboard (`Name in the Index`) docks at
the top when the border's centre is in the lower 45% of the screen (its keys 1307 px higher). A tap outside closes
the card; a tap in the clock's corner toggles the clock and the card stays (the same with Link to... up: nothing
is cancelled). The strip and Link to... docked at the bottom end 8 px above a clock corner at the bottom (z_link_8_corner).
Change where it goes...
opens Link to... with two tools, Search and Cancel (centres x 480 and 1380 upright), the current target's chip
reading ` current`.
A press held on a Links bar chip (`touch down` on it, `wait 800`, `touch up`) opens the same card for all of
that entry's links on the page (`editor.link_card.links`; no Redraw for two or more, and the line "To change one,
hold it on the page."), above the bar in the chip's column (`linkcard_place_above`), the bar staying under it; while
the bar looks at another page it first looks at its own again (`editor.bar.peek is 0`), the borders coming with the
page area's GL16 rather than DU strips.

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
- tests/l10n/templates.qs, in every language and style: the Templates sheet with templates in it
  (its `# copy:` line copies tests/fixtures/tpl-many into the data root first, as make scenarios
  does), over New notebook and over the editor's page, upright and sideways, with its pages of
  tiles (shots in `build/shots/l10n/templates-<lang>-<style>/`).

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
copy of `c_share` held open by the test driver until downloads finish (`make scenarios` runs the short one).
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

The eraser practice pad has probes `penset.pad_strokes` (remaining original sample strokes)
and `penset.pad_sample_rect` (`x y w h` of the centered sample). Precise erasing is measured with
`penset.pad_ink`, `pad_ink_near.X.Y` and `pad_lighter`. `z_8_eraser_targets.qs` checks hits and
misses, fast swept hits, partial erasing, both reset controls and both orientations. The sample
is the maintainer's embedded handwriting; the runtime does not open a user notebook for it.

`penset.param.PARAM` reads the current tab's value to four decimals; `penset.slider_pending`
indicates a value waiting for the throttled repaint. `z_9_slider_fine.qs` uses pen and touch to
hold, move away from the track, select 0.30, catch up and track directly within the knob-height
band, resume fine control on either side, release without a jump and check the saved value.

### Smart-shape probes

`settings.shapes` and `settings.shape_hold_ms` report the saved controls. `editor.shape_preview` reports the lowercase kind while
an in-progress stroke has a snapped preview, or `0` otherwise. `editor.stroke_shape` reports its lowercase recognized kind,
or `none`; `editor.undo_kind` and `redo_kind` also accept `replace`. `pentap panel shapes` toggles recognition.
`z_shape_circle` checks a held circle, one-step undo/redo and replay equivalence; `z_shape_move_on`
checks moving on from the preview restores the freehand stroke. `settings.shape_quads`,
`settings.shape_round`, `settings.shape_corner` and `settings.shape_min_mm` report the menu's other
controls. `editor.stroke_turn_deg` is the newest stroke's sharpest turn from one piece to the next,
in whole degrees, the turn through its start included when it is closed (90 at a snapped rectangle's
corners, a few where Corner roundness made them arcs); empty with no stroke. The Smart shapes menu's
named targets are `shapes.on`, `shapes.hold`, `shapes.quads`, `shapes.round`, `shapes.corner`,
`shapes.min`, `shapes.links` and `shapes.done` (`tap panel NAME`); a slider's target is the middle
of its row, whose hit takes the gaps beside it too. `z_shape_4_ellipses` to `z_shape_7_rounded` hold
generated hand-like strokes (ellipses as drawn, uneven quads, each Four sides mode, Corner
roundness); `z_shape_8_sliders` drags both sliders by finger and pen, upright and sideways, through
a palm's and the pen's cancels, and `z_shape_9_saved` finds its values in settings.ini.

`z_sideways_toolbar` checks rotation with the toolbar on either side of the page, then disables
`landscape_fit` and verifies the saved placement is retained. The full-page landscape gap is 24 px at 300 dpi.

The Index map is a full-screen menu (`editor.map` is `graph`, otherwise `0`). Its probes include
`nodes`, `edges`, `force`, `satellites`, `tray`, `orphans`, `selected` (page number), `mode`, `layout`,
`labels`, `zoom`, `node.PID` (tier, radius, screen x/y, visible), and `ui_at.X.Y`.
`tap map-node PID` resolves the current host's layout before tapping; scenarios never assume saved
coordinates from another machine. Header controls occupy y=20..170: Back x=20..170, List632..832,
All848..1148, Around1160..1460; the selection card's buttons occupy the bottom155pixels.
`GRIMOIRE_SIM_MAP_NOMEM=1` refuses map allocation and checks the Index-list fallback.
`editor.bar.incoming` counts distinct other pages linking to the open page.

Library folders are logical paths. `library.folder` reports the full current path; folder cards show
only their last component. `z_subfolders` checks child creation and parent navigation; storage tests
check subtree rename, empty descendants and refusing overlong destinations before any metadata changes.

`pen strokes FILE.qpg [N] [HOLD_MS]` replays the first N strokes (all by default) through the
current page viewport, preserving pressure/tilt and point spacing, and pausing before each lift.
Timestamp gaps within a stroke are capped at one second in the runner. An unreadable or read-only
file fails the run. `editor.shape_snaps` counts committed snaps and shape-last replacements in the
editor session; `z_shape_writing` holds every one of the bundled handwriting asset's32strokes
for700ms and requires this counter to remain zero. `z_shape_last` checks one Replace undo step
and both new action families. The test runner ticks during an explicit `wait` with the nib down.

`tap library NAME` and `touch down|move|up library NAME` resolve the Library control by its
`library.ui_at` name, including a binding row or a record-card button; `notebook.ID` and
`folder.PATH` are a card, `notebook-menu.ID` and `folder-menu.PATH` its menu button. They fail
if that control is not currently reachable. These commands make recording and controller tests
independent of menu spacing; unit checks still verify separate, on-screen hit targets.
