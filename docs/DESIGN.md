# Grimoire: a homebrew notebook for the Kindle Scribe 2

Grimoire is a standalone notebook app for a jailbroken Kindle Scribe 2 (2024, "Pisco",
MTK Bellatrix3, 1860x2480 @ 300 ppi, firmware 5.19.x, hard-float userland).
Launched from a Library scriptlet (documents/Grimoire.sh). One static binary (musl, armhf), no runtime dependencies.
A desktop simulator (SDL3) and an SDL-free headless script runner exist for development.

## Requirements (from the owner)

1. Paper: narrow rule, medium (college) rule, wide rule, sheet music. (Blank is a bonus 5th.)
2. Library: see notebooks, create notebooks, come back to them later (reopens at the last page).
3. Native pen: Scribe pen over the Wacom EMR digitizer (evdev), pressure + tilt, eraser end.
4. Drawing tools (three at first, nine now: pencil, mechanical pencil, charcoal, ballpoint,
   fineliner, marker, fountain pen, calligraphy pen, highlighter). Each tool has its OWN
   tweakable pressure and tilt sensitivity (plus width/darkness ranges).
5. A floating side panel: draggable, collapses into a small box on a click, expands on a
   click. Holds tools, undo/redo, pages, paper, pen settings, back to library.

## Units and coordinates

- Screen = page. Portrait, origin top-left, pixels. `App.w x App.h` (1860x2480 on Scribe 2).
- Page files store the page size they were drawn at; a load at another size scales points.
- `dpi` is passed everywhere sizes are physical (`mm_to_px(mm, dpi)`), 300 on the Scribe.
- Gray: 8-bit, 0 = black, 255 = white. Everything renders into `Canvas` (8-bit gray).

## Layers / modules (src/)

| file | owns | depends on |
|---|---|---|
| common.h / util.c | Rect math, mm_to_px, clampf, time, logging, string helpers | - |
| canvas.h/.c | 8-bit gray canvas, clip, fill, AA lines/discs, ink deposit, blit, downsample | common |
| font.h/.c | stb_truetype text from embedded fonts; faces by role (regular, bold, title, caps, fixed) mapped per theme: Classic = Terminess Nerd Font Mono, Gothic = pixel faces Jersey 10 (text, labels; bold = overprinted one design pixel) and Jacquard 24 titles, drawn one bitmap pixel per design pixel and blown up to whole k x k blocks (no smoothing); FIXED faces (paper labels) are Terminess in both, so pages never change with the theme | canvas |
| paper.h/.c | page backgrounds (rules, staves) | canvas |
| brush.h/.c | tool params, pressure/tilt mapping, deterministic dab renderer, smoother | canvas |
| page.h/.c | Stroke, Page, hit-test, page render, thumbnail | brush, paper |
| history.h/.c | per-page undo/redo with stroke ownership | page |
| store.h/.c | notebooks on disk, page files, settings ini, thumbnails | page, brush |
| ui.h/.c | widgets (button, slider, sheet, toast, hit lists) in two themes; `ui_frame` draws any box in the theme's shape (rounded, or cut corners via `canvas_chamfer_rect`), plus gothic ornaments (rules, arch frieze, quatrefoil); icons are Terminess Nerd Font glyphs | canvas, font |
| keyboard.h/.c | on-screen keyboard modal (rename / name a notebook) | ui |
| app.h/.c | App struct, main loop glue, dirty-rect + refresh accumulation, screen switch | all screens |
| library.h/.c | library screen, new-notebook sheet (paper picker), rename/delete | app, store, ui, keyboard |
| editor.h/.c | the notebook editor, live ink, eraser, floating panel, page nav, autosave | app, page, history, store |
| pensettings.h/.c | per-tool sensitivity screen: sliders, curve graph, live readout, test pad | app, brush, ui |
| calibrate.h/.c | pen + touch calibration (tap crosshairs) | app, input |
| display.h + display_fbink.c | Kindle e-ink output through FBInk | FBInk |
| input.h + input_evdev.c | Kindle pen/touch/power evdev input, EVIOCGRAB, calibration mapping | linux |
| sim_sdl.c | desktop simulator implementing display.h + input.h (+ headless script mode) | SDL3 (window build only) |
| main.c | parse env/args, open display+input, app loop | everything |

## Rendering + refresh model

- `App.page_canvas` holds the rendered page (paper + ink). `App.screen` is what the panel
  shows: page_canvas plus overlays (floating panel, modals). Overlays are re-drawn on top of
  a blit from page_canvas whenever a region is recomposed.
- Screens never call display directly. They call `app_invalidate(app, rect, mode)` and the
  main loop calls `app_flush(app)` after draining all pending input events, so a burst of
  pen samples becomes one refresh.
- Refresh modes: `REFRESH_FAST` (DU, black/white, for live ink, ~120 ms), `REFRESH_UI`
  (GL16 partial, no flash, for UI and to settle the grays of a finished stroke),
  `REFRESH_FULL` (GC16 flashing, page turns / screen changes / every 30 UI refreshes).
  Accumulating different modes keeps the strongest (FULL > UI > FAST).
- Live stroke: each pen sample batch renders new dabs into page_canvas, recomposes the dirty
  rect, FAST refresh. On pen-up: UI refresh of the stroke bbox so pencil grays come back.

## Brush model (deterministic)

- Stored stroke = tool + a SNAPSHOT of its ToolParams + seed + points (already smoothed).
  Retuning a tool never changes old strokes.
- Rendering stamps dabs along the polyline at a spacing of max(0.5px, 0.15*radius) with
  carried remainder. Incremental (live) and full re-render use the same `brush_add`
  iterator, so a re-render is pixel-identical to what was drawn live.
- Pressure: raw 0..1 -> floor/ceiling window -> ^gamma -> 0..1. Tilt: angle from vertical
  (from tilt_x/tilt_y degrees) -> threshold/max window -> 0..1.
  width = lerp(size_min, size_max, p) * (1 + tilt_size * t);
  opacity = lerp(opacity_min, opacity_max, p) * (1 - tilt_opacity * t).
- Grain: a fixed paper-tooth noise field over absolute page coordinates. A dab's coverage
  at a pixel is multiplied by a tooth factor; light pressure shows more tooth. Pencil high,
  mechanical lower, ballpoint ~0.
- Nibs: the fountain pen stamps an ellipse at a fixed 40 degrees (4.5:1). The calligraphy pen
  stamps a flat square-cut nib (a superellipse, about 8:1) whose broad edge sits at 90 degrees
  to the pen's lean, read from tilt_x/tilt_y in the page frame (input_evdev.c turns the
  digitizer's tilt like positions: calib_tilt). Below its tilt window the lean is not trusted
  and the nib blends to the default 45 degrees (a right hand leaning bottom right); along the
  stroke the orientation is smoothed over ~1.5 mm, as part of the deterministic brush state.
- Charcoal: the graphite path with the tooth sampled at twice the scale (clumps), a rougher
  and softer edge, less valley filling at full pressure, and loose specks up to 0.4 r past the
  edge, hashed on page position and the stroke seed. brush_max_radius accounts for all of it.
- Blend: "deposit" darkens only: dst = dst - (dst - ink) * a, applied per dab.
- Eraser is a stroke eraser (removes whole strokes it touches); one pen-down..up is ONE
  undo step. The eraser end of the pen (BTN_TOOL_RUBBER) and holding the side button
  (BTN_STYLUS) erase regardless of the selected tool.

## On-disk layout (root = /mnt/us/grimoire on Kindle, $GRIMOIRE_ROOT or ./grimoire-data in sim)

```
root/settings.ini              tool params x4, current tool, panel pos/collapsed, calibration
root/notebooks/<id>/notebook.ini   name, paper, created, modified, last_page, pages=<id,id,...>
root/notebooks/<id>/p<pageid>.qpg  binary page (see store.h)
root/notebooks/<id>/thumb.pgm      binary PGM thumbnail of page 1 (P5)
root/trash/<id>/               deleted notebooks are moved here, never unlinked
```
All writes are atomic: write `<file>.tmp`, fsync, rename. Page saves happen ~1.5 s after the
last pen-up, on page change, on leaving the editor and on exit.

## Kindle integration

- Scriptlet `documents/Grimoire.sh` (hdnext jailbreaks; KUAL is obsolete there) detaches
  `/mnt/us/grimoire/app/grimoire.sh`, which: disables the screensaver
  (`lipc-set-prop com.lab126.powerd preventScreenSaver 1`), hides pillow (status bar),
  SIGSTOPs the framework processes so nothing else draws or eats input, runs the binary,
  and on exit restores everything (SIGCONT, pillow, screensaver) and asks for a full repaint.
- The binary EVIOCGRABs the pen and touch devices. The power button (not grabbed) is
  watched: pressing it saves and exits cleanly back to the Kindle UI.
- Signals SIGTERM/SIGINT/SIGHUP save and exit cleanly.

## Build

- `make sim` (macOS/Linux, SDL3) -> build/host/grimoire-sim
- `make headless` (no SDL) -> build/host/grimoire-headless; `make scenarios` runs tests/scenarios/*.qs
- `make check` -> everything verifiable without the device
- `make test` (host, ASan+UBSan) -> runs tests/test_*.c
- `make kindle` (zig cc -target arm-linux-musleabihf, static) -> build/kindle/grimoire
- `make package` -> build/dist/Grimoire.zip

## Getting notebooks onto a computer (three roads, all share export.c)

1. **Wi-Fi, zero setup** - Library > Share starts `httpd.c` on port 8765 and shows the URL
   (`http://<kindle-ip>:8765`) in big type. Any browser: index of folders and notebooks with
   thumbnails; per page PNG, per notebook PDF / zip, per folder zip, everything zip.
   `/api/list.json` makes it scriptable. The server only runs while the Share screen is open.
2. **SSH / rsync** - `tools/grimoire-pull` (Mac) rsyncs `root/notebooks` from the Kindle
   (USBNetwork or any sshd on the device, host from `$GRIMOIRE_HOST`, default `kindle`), then runs
   the host build of the same renderer, `grimoire-export`, to produce
   `~/Grimoire/<folder>/<notebook>.pdf` + `pages/NNN.png`. Incremental; renders only what changed.
   `grimoire-pull --http <ip>` uses the share server instead (curl), no ssh needed.
3. **USB cable** - with `mirror_pdf` on (default), closing a notebook writes
   `/mnt/us/grimoire/PDFs/<folder>/<name>.pdf` from a niced background process (bgjob.c), so
   the UI never waits on a render. Plug in; they're in grimoire/PDFs. (Not in documents/: the Kindle indexes every PDF there, and each one made its home screen redraw on top of Grimoire.)

## Folders

One level. A notebook's `folder` field ("" = top level). The library shows folders first as
cards; tapping one lists its notebooks (with a back button). New notebooks are created in
the open folder. Card menu: Rename, Move to folder (pick existing / new via keyboard / top
level), Delete.
