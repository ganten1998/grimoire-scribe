# Desktop simulator

`make sim && ./build/host/quire-sim` opens a window showing the 1860x2480 page scaled to fit,
with refreshes emulated (DU regions show as pure black/white until a later GL16 settles them).
Data goes to `./quire-data` (or `$QUIRE_ROOT`).

| input | does |
|---|---|
| left mouse | pen (down / drag / up) |
| right mouse | finger |
| `1`..`9`, `0` | pen pressure 0.1..0.9, 1.0 (also `QUIRE_SIM_PRESSURE`) |
| arrow keys | tilt x / y in 5 degree steps (shown in the title bar) |
| hold `E` | eraser end |
| hold `S` | side button |
| `P` | power button (save and exit) |
| `Esc`, Cmd+Q | quit |

`QUIRE_SIM_SIZE=WxH` changes the virtual panel; `QUIRE_SIM_STATS=1` prints refresh counts.

## Headless scripts

`make headless` builds `quire-headless`, which links no SDL and never opens a window. It runs a
script and can dump the panel to PNG; `make scenarios` runs every `tests/scenarios/*.qs`.

```
QUIRE_ROOT=/tmp/q QUIRE_SCRIPT=tests/scenarios/a_notebook.qs QUIRE_SHOTS=/tmp/shots build/host/quire-headless
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
| `drag pen|touch X0 Y0 X1 Y1 STEPS` | a slow drag (100 ms per step) |
| `wait MS` | let timers run (autosave, toasts, the share server) |
| `shot NAME` | write what the panel shows to `$QUIRE_SHOTS/NAME.png` |
| `power`, `quit` | power button / quit (end of script also quits) |

Each command line before a `wait`, `shot` or tap is delivered as one batch, like a burst of
digitizer samples between two screen refreshes on the device.
