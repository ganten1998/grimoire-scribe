# Quire

A homebrew notebook for the jailbroken **Kindle Scribe 2** (2024): narrow, medium and wide
rule plus Cornell, dot grid, graph, sheet music, grand staff, guitar and 4/5/6/7-string bass tab, custom music layouts, a library of notebooks and folders, seven pens tuned for the Scribe's
pressure + tilt digitizer, a floating tool panel, and three easy ways to get pages onto a
computer. One static binary, launched from a Library scriptlet. Typeset in Terminess.

![tools](docs/tools.png)

## What it does

- **Paper**: narrow (5.5 mm), medium (7.1 mm), wide (9 mm) rule, each with or without the
  margin line, Cornell notes (cue column + summary), 5 mm dot grid, 5 mm graph paper, roomy 8-staff
  sheet music, grand staff (treble + bass with brace), 6-line guitar tab, 4/5/6/7-string bass tab,
  blank, and **Custom music**: stack treble / bass / alto / tenor / drum / plain staves, 4-7 line
  tabs and lyric lines into a system (up to 6 parts), reorder them, choose how many systems go on a
  page (it shows how many fit), with a live page preview. Treble directly over bass gets a piano brace. Each page can change paper on its own (undoable).
- **Library**: notebooks as cards with thumbnails, one level of folders, rename / move / delete
  (delete goes to `quire/trash`, never unlinked). Notebooks reopen on the page you left.
- **Pens**: pencil (graphite with paper grain, shades when tilted), mechanical pencil (thin,
  even), ballpoint (solid, quick to full ink), fountain pen (italic nib: thick and thin by
  direction), fineliner (crisp, constant width), marker (broad, dark), highlighter (broad gray that never hides the
  writing). The eraser removes whole strokes; the Premium Pen's eraser end erases with any tool.
- **Pen button**: hold it while writing to highlight (or erase, in Settings); click it 3 times,
  or hold it about a second without touching the screen, to clear the page. The toolbar's broom clears too.
- **Pages**: tap the page counter on the toolbar for a grid of page previews; tick pages and
  Delete (they go to `quire/trash`), or tap a preview to jump there.
- **Sleep**: the power button (or "Sleep after" N minutes without use, in Display) puts the
  Kindle to sleep showing the page you're on, or a page you pinned, or the Kindle's own
  screensaver; waking returns straight into Quire. Quit with Library > Exit.
- **Display**: the Kindle's front light brightness and warmth (system levels 0..24, the same
  ones the stock UI uses), and an invert option that shows Quire white on black.
- **Zoom and rotation**: pinch with two fingers to zoom 1x..4x and pan; a chip resets to 100%.
  Quire turns with the Kindle (portrait, upside down, both landscapes; can be switched off). Strokes keep the settings they were
  drawn with, so retuning a pen never changes old pages.
- **Pen feel**: per tool, 13 sliders: min/max width, light/full darkness, pressure deadzone,
  "full at" and curve, tilt start/full, how much tilt widens and lightens, grain, smoothing.
  A live pressure-curve graph, raw-to-mapped readout and a scratch pad to try them.
- **Floating panel**: drag it anywhere by its grip; tap the corner arrows to collapse it into a
  small box showing the current tool, tap the box to expand, drag the box too. Holds the tool button
  (tap it for a picker of all seven pens), the eraser, undo / redo, clear, prev / page count / next (or new page), paper, pen feel, library.
- **Getting pages off the Kindle** (see [docs/EXPORT.md](docs/EXPORT.md)):
  1. *Wi-Fi*: Library > Share shows `http://<kindle-ip>:8765`; open it in any browser for
     PNG pages, PDF / zip notebooks, folder and everything zips.
  2. *SSH*: `tools/quire-pull` rsyncs and renders PDFs on the Mac with the same renderer.
  3. *USB*: closing a notebook writes its PDF to `documents/Quire/<folder>/`.

## Install

See [docs/KINDLE.md](docs/KINDLE.md). Short version: `make package`, unzip `build/dist/Quire.zip`,
copy the two files in `documents/` into the Kindle's `documents` folder (OpenMTP on a Mac), then
tap **Quire** in the Library. `Quire.sh` is self-contained and unpacks the app on first run.
(No KUAL: on Vera-era jailbreaks it is obsolete.)

## Build

Needs `zig` (Kindle cross build), `clang`, and SDL3 for the desktop simulator.

```
make check     # unit tests, scripted UI scenarios, share server, exporter, ARM build
make sim       # desktop simulator window (docs/SIM.md)
make kindle    # build/kindle/quire: static ARMv7 hard-float (musl) binary
make package   # build/dist/Quire.zip
make tools     # build/host/quire-export (used by tools/quire-pull)
```

## Layout

`docs/DESIGN.md` is the architecture: modules, refresh model, brush model, file formats.

## Licenses

Quire's code: unlicensed so far (your call; MIT would suit). Terminess Nerd Font: SIL OFL 1.1 (`assets/Terminess-OFL-1.1.txt`).
FBInk: GPL-3.0-or-later (linked into the Kindle binary, so that binary is GPLv3).
stb_truetype / stb_image_write: public domain / MIT.
