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
| font.h/.c | stb_truetype text from embedded fonts; faces by role (regular, bold, title, caps, fixed) mapped per theme: Classic = Terminess Nerd Font Mono, Gothic = pixel faces Jersey 10 (text, labels; bold = overprinted one design pixel, sparing accents: see Text and the keyboard) and Jacquard 24 titles, drawn one bitmap pixel per design pixel and blown up to whole k x k blocks (no smoothing); FIXED faces (paper labels) are Terminess in both, so pages never change with the theme | canvas |
| paper.h/.c | page backgrounds (rules, staves); a kind it does not know draws blank ("Newer paper") | canvas |
| brush.h/.c | tool params, pressure/tilt mapping, deterministic dab renderer, smoother | canvas |
| page.h/.c | Stroke, Page, hit-test, page render, thumbnail | brush, paper |
| history.h/.c | per-page undo/redo with stroke ownership | page |
| store.h/.c | notebooks on disk, page files (docs/FORMAT.md), settings ini, thumbnails | page, brush |
| power.h/.c | the power key (press, release, 4 s hold) and the power service's state, as pure logic | input |
| fbcheck.h/.c | framebuffer readback at 8 and 32 bpp, refresh health, the dpi choice (pure) | - |
| ui.h/.c | widgets (button, slider, sheet, toast, hit lists) in two themes; `ui_frame` draws any box in the theme's shape (rounded, or cut corners via `canvas_chamfer_rect`), plus gothic ornaments (rules, arch frieze, quatrefoil); icons are Terminess Nerd Font glyphs | canvas, font |
| keyboard.h/.c | on-screen keyboard modal (rename / name a notebook): UTF-8, a letters page and an accents page | ui |
| qr.h/.c | QR codes as whole-pixel modules (the Share screen's address), with third_party/qrcodegen | canvas |
| app.h/.c | App struct, main loop glue, dirty-rect + refresh accumulation, screen switch, toasts, power button and sleep | all screens |
| httpd.h/.c, web/studio.* | share server (pages and fonts in-process, renders in a forked child) and the browser's Effects studio | export, store |
| share.c | the Share screen: the QR code, the addresses, its own layout check; the server runs only while it is open | app, httpd, qr |
| library.h/.c | library screen, new-notebook sheet (paper picker), rename/delete | app, store, ui, keyboard |
| editor.h/.c | the notebook editor, live ink, eraser, floating panel, page nav, autosave, read-only pages | app, page, history, store |
| pensettings.h/.c | per-tool sensitivity screen: sliders, curve graph, live readout, test pad | app, brush, ui |
| calibrate.h/.c | pen + touch calibration (tap crosshairs) | app, input |
| display.h + display_fbink.c | Kindle e-ink output through FBInk, the screen guard's readback, refresh health | FBInk, fbcheck |
| input.h + input_evdev.c | Kindle pen/touch/power evdev input, EVIOCGRAB, calibration mapping, the power service watcher | linux, power |
| light.h + light_kindle.c | front light and the power service through lipc (sleep request, state, preventScreenSaver) | lipc |
| sim_sdl.c | desktop simulator implementing display.h + input.h (+ headless script mode with expect checks) | SDL3 (window build only) |
| main.c | parse env/args, startup diagnostics, open display+input, app loop, screen guard | everything |

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
  rect, FAST refresh. On pen-up: UI refresh of the stroke bbox so pencil grays come back. A
  press on the panel before that settle settles first, so Undo and Clear answer at once.
- Toasts are never stored in `App.screen`: `app_flush` overlays one just for the refresh and puts
  the pixels under it straight back. `App.toast_floor` keeps them clear of a screen's controls:
  a screen that keeps the bottom for controls sets it as it draws, and toasts sit above it. The
  Library sets it in `redraw` for an open sheet: the keyboard's top, or else the top of the
  control just under the first band of a toast's height that is clear of the sheet's controls,
  found going up from the toast's usual place (for Custom music, only that sheet's own controls
  count). `app_switch` clears the floor, and a toast already up moves at the next flush
  (`toast_follow`).
- Display safety (main.c, display_fbink.c): the screen guard reads the framebuffer back (8 and
  32 bpp) and repaints what another process drew; after 10 repaints in a row it turns itself off.
  If the first frame and the next 4 refreshes all fail (`display_broken`, retried once a second),
  Grimoire exits with status 1 so the launcher gives the Kindle its screen back. docs/KINDLE.md has
  the log lines.

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

## Text and the keyboard

- The Gothic bold is the pixel glyph overprinted one design pixel to the right. Above U+007F,
  `pixel_bold` builds each glyph's bold once (cached with the glyph): the letter takes the
  overprint, and a mark that stands apart from it (wholly above or below its largest part) is
  drawn as designed where the overprint would close a one-pixel gap in it (the dots of ü, a
  tilde, a hat, a caron, a ring, a double acute). Acutes, graves, the dots of ¡ and ¿, and the
  two-pixel-gap marks of î ï ÿ keep the overprint. In ß, ø and Ø (`bold_keeps_gaps`) each
  one-pixel gap moves one design pixel right instead of closing; otherwise ß reads as B and ø/Ø
  as a theta. `pixel_drop` sets Jersey 10's inverted exclamation mark (U+00A1) two design pixels
  lower in every Gothic weight: the font draws it exactly as its i, and two pixels is the depth of
  the face's descenders. ASCII never takes these paths and draws as it always has.
  `font_has_glyph(face, cp)` tells whether a face has its own glyph, without the Terminess
  fallback.
- The keyboard's text is UTF-8 and its keys carry code points (`KB_KEY_*` below 32 for the named
  keys). There are two pages, letters and accents (`Keyboard.page`); `keyboard_upper()` is the
  shift case map (Latin-1 and Latin Extended-A), `keyboard_key_rect()` locates a key on the page
  showing (for tests and scripts), and `keyboard_open` keeps valid UTF-8 and drops control
  characters and broken bytes. A key types a whole character; backspace removes one.

## On-disk layout (root = /mnt/us/grimoire on Kindle, $GRIMOIRE_ROOT or ./grimoire-data in sim)

```
root/settings.ini              current tool, panel, every tool's params, calibration, sleep,
                               style, port... (unknown keys kept)
root/folders.ini               the folder names, one per line
root/sleep.pgm                 the pinned sleep screen (P5)
root/notebooks/<id>/notebook.ini   name, folder, paper, created, modified, last_page,
                                   next_page_id, pages=<id,id,...> (unknown lines kept)
root/notebooks/<id>/p<pageid>.qpg  binary page (see docs/FORMAT.md)
root/notebooks/<id>/p<pageid>.qpg.damaged-<time>   a page that could not be read, kept
root/notebooks/<id>/thumb.pgm      binary PGM thumbnail of page 1 (P5)
root/trash/<id>/               deleted notebooks are moved here, never unlinked
root/PDFs/<folder>/<name>.pdf  the PDF mirror (an export, not data)
root/.passcode-paused          holds "paused" while the passcode is paused (Kindle integration)
root/.passcode-left-off        how many starts were told a restart left the passcode off
```
All writes are atomic: write `<file>.tmp`, fsync, rename. Page saves happen about 2 s after the
last pen-up, at every press of the power button, on page change, on leaving the editor and on
exit. `store_add_page` never writes over an existing `p<id>.qpg`: it takes the next free id.

## Forward compatibility

The rule is "carry what you do not understand" (docs/FORMAT.md has every byte and rule). Going
back to an older Grimoire is a one-file copy, and 1.0.0-beta and rc.1 can never be fixed.

- In memory: `Page.readonly` (a page this build cannot fully read: shown as far as it can be,
  never changed or written; the editor refuses writing, erasing, clearing, paper changes and
  undo/redo on it with a toast), `Page.tail` (the chunks after the strokes, written back
  verbatim), `Stroke.ext` (the record's three spare bytes) and unknown paper kinds (kept in
  `Page.paper`, drawn blank as "Newer paper"). store.c keeps unknown settings.ini and notebook.ini
  lines, and a known setting's value it cannot use, as they were read.
- `store_recover_pages` (from `editor_open_notebook`) brings back a `p<id>.qpg.damaged-<time>`
  copy that opens now, for a page still in the page list whose `p<id>.qpg` is missing (both are
  kept when it is there); real damage stays aside.
- New page data goes in the ext bytes, the erase-stroke seed bits, paper values or chunks;
  notebook-wide data goes in files of its own; notebook.ini and settings.ini hold only what
  1.0.0-beta may forget. Never bump the page version or add a tool id while 1.0.0-beta is in use.
- The stroke parameter ranges (`k_params`) are frozen in both directions while 1.0.0-beta or
  1.0.0-beta.2 is in use: a wider value travels in the ext bytes or a critical chunk. A safe
  (lowercase) chunk must never depend on page coordinates or the page size.

## Kindle integration

- Scriptlet `documents/Grimoire.sh` (hdnext jailbreaks; KUAL is obsolete there) carries the app:
  it unpacks it into `/mnt/us/grimoire/app` when its BUILD_ID differs, then detaches
  `app/grimoire.sh`, which: sets `preventScreenSaver 1` while Grimoire starts (Grimoire turns it
  off again unless Sleep after is never), hides pillow (status bar), SIGSTOPs the framework
  processes so nothing else draws or eats input, pauses the Device Passcode (below), runs the
  binary, and on exit restores everything (SIGCONT, pillow, screensaver, passcode) and starts the
  home booklet. On a locked Kindle (power service state `screenSaver`, `readyToSuspend` or
  `suspended`) the restore does not write the screen saved at the start back: it would cover the
  screensaver or the passcode prompt.
- Both launchers refuse to start inside KOReader (exit 3): `KOREADER_DIR` set, or `pidof
  reader.lua` succeeding. KOReader holds the pen and the touch screen, and Grimoire's exit would
  wake the Kindle UI underneath it. `Grimoire.sh --unpack-only` (unpack, start nothing) and the
  `# Capabilities:` lines are for the planned KOReader plugin; docs/KINDLE.md has the flags, the
  exit statuses and how a tool reads the Capabilities lines.
- The Device Passcode: with one set, the Kindle holds back `outOfScreenSaver` until the passcode
  is typed, and its keypad cannot take a touch while Grimoire grabs the touch screen, so a locked
  Kindle could not be unlocked. Like KOReader, `app/grimoire.sh` removes
  `/var/local/system/userpasswdenabled` for the session and touches it back at the end of
  `restore()`, after the home booklet starts (in stop-framework mode before `start lab126_gui`,
  which reads it). The pause needs its marker, `grimoire/.passcode-paused` (holding `paused`): it
  is written first, with echo (a failed redirection on `:`, a special built-in, would end the
  script), and a marker that cannot be written means no pause. A marker found with no flag at the
  next start means a session ended without `restore()` (the Kindle restarted under Grimoire): the
  passcode is then not turned back on, since it may have been changed or turned off since.
  `grimoire/.passcode-left-off` counts the starts that tell the user so (that one and the next two
  while the flag is still missing) and goes as soon as the passcode is back, or after the third.
  The binary learns which through `GRIMOIRE_PASSCODE` (`paused`, `left-off` or empty) and says so:
  "Your Kindle passcode is paused while Grimoire is open." once (settings.ini `passcode_told`), or
  "Your Kindle passcode was left off. Turn it back on in Kindle Settings." in place of any other
  start notice.
- The binary EVIOCGRABs the pen and touch devices. The power button (not grabbed) is watched: a
  press saves, the power service sleeps and wakes the Kindle inside Grimoire (next section), and a
  4 s hold saves and exits. Signals SIGTERM/SIGINT/SIGHUP save and exit cleanly. Whichever way
  Grimoire ends, the launcher's EXIT trap restores the Kindle UI.

## Sleep and the power button

The power service (`com.lab126.powerd`) sleeps and wakes the Kindle, and Grimoire follows it. A
shell loop of one-shot `lipc-wait-event -s 0 com.lab126.powerd goingToScreenSaver,outOfScreenSaver`
(behind a TERM trap, so it dies with Grimoire and takes lipc-wait-event along) reports its
announcements: goingToScreenSaver draws the sleep screen, outOfScreenSaver is the wake.

- A press (IN_POWER) saves and cancels any fallback the press before it left armed. Its release
  (IN_POWER_UP, value = ms held, -1 if the release was missed) decides in `power_up_action` which
  fallback, if any, the press still needs: a short press while awake waits 2.5 s for
  goingToScreenSaver; a short press on the sleep screen waits 3 s for outOfScreenSaver. A sleep or
  wake announcement that arrives while the key is down answers the press, so its release arms
  nothing; the same goes for a release on a different screen (asleep or awake) than its press.
- Both fallbacks first read `lipc-get-prop -s com.lab126.powerd state` (1.5 s limit). In
  `screenSaver`, `readyToSuspend` or `suspended`, Grimoire sleeps without sending powerButton, or
  stays asleep: the Kindle is locked or asleep, and Grimoire never draws over it. The second wake
  fallback in a row that finds it so saves and leaves Grimoire (`power_wake_fallback`); any wake
  resets the count. In `active` or unknown it requests sleep, or wakes.
- A held press (IN_POWER_HELD at INPUT_POWER_HOLD_MS = 4000, confirmed with EVIOCGKEY) saves and
  exits from any state, and so does a release held 4 s or more whose hold never reached Grimoire.
  The power key is timed by the kernel's event stamps (EVIOCSCLOCKID CLOCK_MONOTONIC).
- `app_leave` (the hold, the second locked wake fallback) never drops a page unannounced:
  `editor_save_now` returns whether the save worked. When it failed, awake, the first try stays
  once (`App.leave_unsaved_at`) with a toast, and another within 60 s leaves without the page;
  asleep it leaves at once. The autosave of a page that cannot be written tries again 2 s later,
  not at every tick.
- The sleep request sends `preventScreenSaver 0` and `powerButton 1` from one ordered shell, and
  the sleep policy is applied again at every wake, so "Sleep after: never" gets preventScreenSaver
  1 back.
- While asleep, pen and touch, side-button presses and pinch begin/move are dropped
  (`power_asleep_drops`), but button releases and pinch ends pass, so one that began before the
  sleep finishes (a button left down would clear the page by its hold right after the wake).
- Asleep on the Share screen, `share_sleep` stops watching the server's listening socket: no
  screen ticks while asleep, so a connection waiting there would make `input_poll` return at once
  and the main loop spin until the wake. It waits in the backlog instead, and `share_enter` (the
  wake's relayout) watches the socket again. With no page to show (sleep mode 0, or mode 1 with
  no pinned page), the sleep screen is `share_draw_asleep`'s card, without the code and the
  address.
- The pure logic and its tests: src/power.c (tests/test_power.c) and src/fbcheck.c
  (tests/test_fbcheck.c).

## Build

- `make sim` (macOS/Linux, SDL3) -> build/host/grimoire-sim
- `make headless` (no SDL) -> build/host/grimoire-headless; `make scenarios` runs every
  tests/scenarios/*.qs. Scenarios check the app's state with `expect` lines (docs/SIM.md): a
  failed check, an unknown script line or a leftover `*.tmp` fails the build once every scenario
  has run, and a headless run waits for the PDF exports it started. A `# root: NAME` line starts a
  scenario from another scenario's data (d_persist and x_turn_calibrated both use b_library's,
  s_passcode_once r_passcode's), `# env: K=V` lines set its environment (`GRIMOIRE_PASSCODE` in
  r_passcode, s_passcode_once, t_passcode_left_off and t_toast_over_sheets, `GRIMOIRE_BG_DELAY_MS`
  in v_leave_from_editor), and a `# pdfs: N` line wants exactly N PDFs in its root's PDFs/ folder.
  The rule unsets the test hooks first, so nothing else of the caller's reaches them. With
  GRIMOIRE_TEST_PORT set every scenario's share server uses that port. tests/selftest/pass.qs and
  fail.qs check the runner itself.
- Dependencies: HDRS holds the Makefile, VERSION and third_party/qrcodegen/qrcodegen.h besides
  src/*.h, so a change of flags rebuilds every program (FBInk's objects follow only their own
  .c files); the test binaries also depend on tests/*.h (beta_format.h among them), and the zip on
  the Makefile. TEST_FLAGS pin `-ffp-contract=on`, because tests/test_golden.c's table holds only
  with contraction on (clang's default, under which it was recorded).
- `make check` -> everything verifiable without the device: the unit tests, the scenarios, the
  share server (tests/share_check.sh), the host exporter, the ARM build, and the zip:
  package-check (tests/package_check.sh, what the zip holds) and launcher-check
  (tests/launcher_check.sh: both launchers run against stand-ins for the Kindle's tools in
  tests/shims, under each POSIX shell it finds).
- `make test` (host, ASan+UBSan) -> runs tests/test_*.c
- `make kindle` (zig cc -target arm-linux-musleabihf, static) -> build/kindle/grimoire
- `make package` -> build/dist/Grimoire.zip. The licenses in its app/licenses: the GPL; the font
  licenses, which follow FONTS in the Makefile (each embedded font needs an
  assets/<family>-OFL-1.1.txt whose family begins the font's file name); every
  third_party/<dir>/LICENSE* or COPYING* as <dir>-LICENSE.txt and CREDITS* as <dir>-CREDITS.txt;
  the stb headers' license, extracted once into stb-LICENSE.txt; musl's COPYRIGHT, from `zig env`'s
  lib_dir; zig's LICENSE, found beside that lib_dir, as zig-compiler-rt-LICENSE.txt (zig cc links
  its compiler-rt into the binary: 64-bit division, float conversions, memcpy and more); and for
  the two parts of FBInk's tree with notices of their own (FBINK_SRC), cutef8-LICENSE.txt and
  dfa-LICENSE.txt, each the head of its source (cutef8/utf8.c, cutef8/dfa.c) and its entry in
  FBInk's CREDITS, the only text the tree has for them. The recipe fails when it cannot find zig's
  LICENSE or musl's COPYRIGHT, or one of those notices. A vendored directory with no license file,
  a font without its OFL text, or a missing notice fails package-check, and so does a change to
  FBINK_SRC until its notice is added.

## Tests

- tests/test_golden.c holds renders recorded from 1.0.0-beta: every paper it knows, a stroke of
  every tool, and a page's bytes. A page someone already wrote must look exactly the same in every
  later version, so it is never re-recorded to make a change pass. It compares on the recording
  host only (clang on arm64 macOS).
- tests/test_format.c checks pages against tests/beta_format.h, a frozen copy of the 1.0.0-beta
  page reader and writer (the "Moving between versions" table in docs/FORMAT.md).
- tests/test_font.c pins text renders by CRC, recorded at 56d931c: all printable ASCII in every
  face, both themes and 13 sizes, plus the never-overprinted faces with accents (its accents
  string leaves out ¡, which Jersey 10 now sets lower, and includes the Catalan middle dot). After
  a deliberate rendering change, `GRIMOIRE_FONT_CRC=print build/test/test_font` prints a fresh
  table. Terminess (anti-aliased) entries are compared only with clang on arm64, where they were
  recorded. bold_letters_keep_their_gaps and inverted_exclamation_is_not_an_i guard the Gothic
  bold.
- tests/test_keyboard.c drives the keyboard API: one_character_keys_at_title_size compares each
  key with font_draw_centered at the title or text size, and typed_name_is_stored_whole sends the
  keyboard's text through store_rename, store_set_folder, export_mirror_path and
  export_mirror_pdf.
- tests/test_power.c and tests/test_fbcheck.c test src/power.c and src/fbcheck.c, tests/test_qr.c
  the QR drawing, and tests/test_probe.c that the probes' tool names match settings.ini.
- The launchers' test hooks: `GRIMOIRE_US`, `GRIMOIRE_TMP`, `GRIMOIRE_SYSROOT` and
  `GRIMOIRE_WD_TICK`/`FIRST`/`STALE` exist only for tests/launcher_check.sh and are never set on
  the Kindle. package_check pins every `GRIMOIRE_*` default in the three shipped scripts (the exact
  lines, and no other read of a `GRIMOIRE_*` variable), and that the watchdog watches
  `/tmp/grimoire.alive`, where the binary beats. launcher_check runs one case per shell with no
  `GRIMOIRE_WD_*` set, so the device's 2, 25 and 15 s are used there; its watchdog fakes keep
  running after TERM, the way a stuck Grimoire does, so the check proves TERM and then KILL
  (status 137). The shells run side by side; the check takes about 20 s.
- package_check also wants each third_party CREDITS file in licenses/ (FBInk-CREDITS.txt), and
  builds both power-service watcher command lines from WATCH_EVENTS in src/input_evdev.c: the
  binary must contain them, and `restore()`'s `pkill -f` pattern must match both. launcher_check
  covers the passcode: it comes back only after the home booklet starts (the `lipc-set-prop`
  stand-in notes whether it is set at each appmgrd call), the left-off notice comes at three starts
  and not a fourth, and a marker that cannot be written leaves the passcode on while Grimoire
  still runs.

## Getting notebooks onto a computer (three roads, all share export.c)

1. **Wi-Fi, zero setup** - Library > Share starts `httpd.c` on port 8765 and shows the URL
   (`http://<kindle-ip>:8765`) in big type, under a QR code of it. Any browser: index of folders
   and notebooks with thumbnails; per page PNG, per notebook PDF / zip, per folder zip,
   everything zip. `/api/list.json` makes it scriptable. The server only runs while the Share
   screen is open; turning the Kindle and the redraw after waking keep it (`share_leave` returns
   early while `App.relayout` is set), with its downloads and queue.
   Up to 16 connections at once, each with its own request buffer (a shared one once answered a
   new connection from the previous request); renders fork, two at a time, the rest queue
   oldest first and are dropped when their browser hangs up. The pages use the fonts of the
   Kindle's style (served from the binary, cached a year). The
   Effects studio (`web/studio.js` + `.css`, embedded at build time) rebuilds a page in the
   browser from its `?layer=paper` and `?layer=ink` renders: background, then lines and ink
   composited over it with each layer's darkness as alpha, then tone, warmth, tint,
   brightness/contrast, crop and scale. Each step is one ImageMagick 7 operation computed with
   ImageMagick's own math, and the page prints the equivalent `magick` command;
   `tests/studio_check.js` (run by `make check` when node and magick exist) compares the two.
   The QR code (`ui_qr` in src/qr.c, in UI_SRC so mkicon's link is untouched) uses ECC M, raised
   when that costs no size, and at least version 3, so every IPv4 share URL is the same 29x29
   symbol. Modules are whole pixels (10 to 16 px in the Share layout, 16 px on the Scribe) in a
   4-module quiet zone, pure 0 and 255; under Invert it is drawn as its negative, because the
   inversion happens at display time. It holds the first wlan* address (`net_ipv4_list` orders
   wlan*, then en*/eth*, then others, then usb*), the very string shown in big type: the launcher
   opens the port on wlan0 only. The other addresses get an "Also:" line each, and with no Wi-Fi
   address the Wi-Fi icon stands in for the code. Landscape puts the code on the left and the
   text, counter and buttons on the right. The screen checks its own layout as it draws:
   `draw_line` logs "share: layout: a line is cut to fit (W px wide, room for R)" when a line is
   wider than its column (font_draw_centered would shorten it with '...'; only the simulator adds
   the line's text, which may be the address), and `check_layout` logs any part (the code or the
   icon, the text, the counter, Done, Retry) that leaves the card or overlaps another, with both
   rects. tests/share_check.sh fails on any such line over 1, 2 and 3 addresses, USB only and
   none, in both styles and both orientations.
   grimoire.log goes to public bug reports, so the Share screen never logs the address, only the
   interface and the port ("share: QR code for the wlan0 address, port 8765"), and httpd logs a
   folder zip's request as `/folder/<name>.zip`. Only the simulator adds "share: (simulator)
   the code holds http://...", which share_check compares with the code it reads back. For every
   made-up address list, in both styles, share_check also checks that no other line holds an
   address, and its download run checks that a folder zip's request leaves the folder's name out.
2. **SSH / rsync** - `tools/grimoire-pull` (Mac) rsyncs `root/notebooks` from the Kindle
   (USBNetwork or any sshd on the device, host from `$GRIMOIRE_HOST`, default `kindle`), then runs
   the host build of the same renderer, `grimoire-export`, to produce
   `~/Grimoire/<folder>/<notebook>.pdf` + `pages/NNN.png`. Incremental; renders only what changed.
   `grimoire-pull --http <ip>` uses the share server instead (curl), no ssh needed.
3. **USB cable** - with `mirror_pdf` on (default), closing a notebook writes
   `/mnt/us/grimoire/PDFs/<folder>/<name>.pdf` from a niced background process (bgjob.c), so
   the UI never waits on a render. Plug in; they're in grimoire/PDFs. (Not in documents/: the Kindle indexes every PDF there, and each one made its home screen redraw on top of Grimoire.)
   Exports still queued at exit go to a detached child (`bg_detach_rest`), which waits for the
   running one and writes the rest after Grimoire has gone. `app_shutdown` calls it after the
   screens' `free_`, because `editor_free` closes the open notebook and so queues its PDF (leaving
   straight from the editor: the 4 s hold, a Kindle that stays locked, a signal).

## Folders

One level. A notebook's `folder` field ("" = top level). The library shows folders first as
cards; tapping one lists its notebooks (with a back button). New notebooks are created in
the open folder. Card menu: Rename, Move to folder (pick existing / new via keyboard / top
level), Delete. Folders sort by name with accents folded (`cmp_name` in store.c: Latin-1 and
Latin Extended-A letters compare as their base letters, ß as ss, æ as ae, œ as oe, ĳ as ij, þ as
th, combining accents dropped), so Álgebra sorts among the A's; names that fold equal go by their
bytes. A name, folder or folders.ini line longer than 95 bytes is cut before the UTF-8 character
that would cross the limit (`clean_line`).
