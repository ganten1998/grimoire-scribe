# Grimoire: a homebrew notebook for the Kindle Scribe 2

Grimoire is a standalone notebook app for a jailbroken Kindle Scribe 2 (2024, "Pisco",
MTK Bellatrix3, 1860x2480 @ 300 ppi, firmware 5.19 and 5.20, hard-float userland). The 2022
Scribe has the same panel; the Scribe 3 and the Scribe Colorsoft have a 1986x2648 one, also at 300 ppi, and
notebooks keep their page size between the two.
Launched from a Library scriptlet (documents/Grimoire.sh), or from KOReader through its Grimoire
plugin. One static binary (musl, armhf), no runtime dependencies.
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

- Screen: portrait, origin top-left, pixels. `App.w x App.h` (1860x2480 on the Scribe 2).
- Page: pixels at the notebook's page size, `page_w x page_h` in notebook.ini, which is the screen
  it was made on and stays so wherever it is opened (docs/FORMAT.md, Page size). On a screen of
  that size, upright, page = screen; on the other Scribe size the editor fits the page to the
  screen (below). Every page file also stores the size it was written at; one of another size
  than its notebook's (an older version scaled it to its screen) has its points scaled back when
  it is read (`scale_page` in store.c).
- `dpi` is passed everywhere sizes are physical (`mm_to_px(mm, dpi)`), 300 on every Scribe.
- Gray: 8-bit, 0 = black, 255 = white. Everything renders into `Canvas` (8-bit gray).

## Layers / modules (src/)

| file | owns | depends on |
|---|---|---|
| common.h / util.c | Rect math, mm_to_px, clampf, time, logging, string helpers, dates (in the language in use) | - |
| i18n.h/.c | the language in use, lookups in the embedded catalogs (po/*.mo) with English fallback, plural rules, the language list (see Languages) | common |
| canvas.h/.c | 8-bit gray canvas, clip, fill, AA lines/discs, ink deposit, blit, downsample | common |
| font.h/.c | stb_truetype text from embedded fonts; faces by role (regular, bold, title, caps, fixed) mapped per theme: Classic = Terminess Nerd Font Mono, Gothic = pixel faces Jersey 10 (text, labels; bold = overprinted one design pixel, sparing accents: see Text and the keyboard) and Jacquard 24 titles, drawn one bitmap pixel per design pixel and blown up to whole k x k blocks (no smoothing); FIXED faces (paper labels) are Terminess in both, so pages never change with the theme. DotGothic16 is the Japanese fallback for kana and kanji in both themes | canvas |
| paper.h/.c | page backgrounds (rules, staves); a kind it does not know draws blank ("Newer paper") | canvas |
| brush.h/.c | tool params, pressure/tilt mapping, deterministic dab renderer, smoother | canvas |
| page.h/.c | Stroke, Page, hit-test, page render, thumbnail | brush, paper |
| history.h/.c | per-page undo/redo with stroke ownership | page |
| store.h/.c | notebooks on disk, page files (docs/FORMAT.md), settings ini, thumbnails | page, brush |
| power.h/.c | the power key (press, release, 4 s hold) and the power service's state, as pure logic | input |
| fbcheck.h/.c | framebuffer readback at 8 and 32 bpp, refresh health, the dpi choice (pure) | - |
| ui.h/.c | widgets (button, slider, sheet, toast, hit lists) in two themes; `ui_frame` draws any box in the theme's shape (rounded, or cut corners via `canvas_chamfer_rect`), plus gothic ornaments (rules, arch frieze, quatrefoil); icons are Terminess Nerd Font glyphs | canvas, font |
| keyboard.h/.c | on-screen keyboard modal (rename / name a notebook): UTF-8, a letters page, a symbols page and an accents page | ui |
| qr.h/.c | QR codes as whole-pixel modules (the Share screen's address), with third_party/qrcodegen | canvas |
| app.h/.c | App struct, main loop glue, dirty-rect + refresh accumulation, screen switch, toasts, power button and sleep | all screens |
| httpd.h/.c, web/studio.* | share server (pages and fonts in-process, renders in a forked child) and the browser's Effects studio | export, store |
| status.rs | (the Rust build only) the clock and battery corner: a small indicator in the top left, painted over the screen for each refresh like a toast; its tap zone, its refreshes at the minute and on a battery change, held back while the pen is near, its battery reading (the kernel's power supply, else the power service) | app, ui, light |
| share.c | the Share screen: the QR code, the addresses, its own layout check; the server answers its routes only while it is open. Also Live view's pairing card | app, httpd, qr |
| live.h/.c, web/live.* | Live view: the shared copy of the screen, its 64 px tile marks, the viewer children that stream the tiles as PNGs; the pen sample ring and the pen viewer children (fast ink), the presented watermark; the browser's viewer | canvas, input |
| pair.h/.c | one-use pairing codes and QR tokens, and the browser sessions they give (memory only) | - |
| library.h/.c | library screen, new-notebook sheet (paper picker), rename/delete, Settings (Language among them) | app, store, ui, keyboard |
| editor.h/.c | the notebook editor, live ink, eraser, floating panel, page nav, autosave, read-only pages | app, page, history, store |
| pensettings.h/.c | per-tool sensitivity screen: sliders, curve graph, live readout, test pad | app, brush, ui |
| calibrate.h/.c, calib_fit.h/.c | pen + touch calibration (tap crosshairs); the fit, and scaling a calibration to another panel (`calib_to_panel`) | app, input |
| display.h + display_fbink.c | Kindle e-ink output through FBInk, the screen guard's readback, refresh health, live ink and the color panel's wait | FBInk, fbcheck |
| input.h + input_evdev.c | Kindle pen/touch/power evdev input, EVIOCGRAB, calibration mapping, the power service watcher | linux, power, pensync |
| pensync.h/.c | the pen after the kernel dropped its events (the tail's frames in order, never the present position first) and samples' kernel times, as pure logic | - |
| light.h + light_kindle.c | front light and the power service through lipc (sleep request, state, preventScreenSaver) | lipc |
| sim_sdl.c | desktop simulator implementing display.h + input.h (+ headless script mode with expect checks) | SDL3 (window build only) |
| main.c | parse env/args, startup diagnostics, open display+input, app loop, screen guard | everything |

## Rendering + refresh model

- The editor's `page_canvas` holds the rendered page (paper + ink) at the notebook's page size,
  not the device's: `use_page_size`, as a notebook opens, makes it and the paper previews again
  when the size changes, and every page the editor reads comes through `load_page`, at that
  size. The view maps page to screen (zoom + pan): `view_reset` fits the page's width to the
  screen's (`zoom_default` = screen width / page width, 1.00 upright for a notebook made on this
  screen, 1.07 for an 1860x2480 notebook on a 1986x2648 screen), `zoom_min` shows the whole page,
  and a pinch zooms up to 4x the fit. The sleep screen fits the page to the screen the same way
  (`canvas_blit_fit`). `App.screen` is what the panel shows: the viewed page plus overlays
  (floating panel, modals). Overlays are re-drawn on top of the page whenever a region is
  recomposed.
- Screens never call display directly. They call `app_invalidate(app, rect, mode)` and the
  main loop calls `app_flush(app)` after draining all pending input events, so a burst of
  pen samples becomes one refresh.
- Refresh modes: `REFRESH_FAST` (DU, black/white, for live ink, ~188 ms measured, 294 ms cold), `REFRESH_UI`
  (GL16 partial, no flash, for UI and to settle the grays of a finished stroke),
  `REFRESH_FULL` (GC16 flashing, page turns / screen changes / every 30 UI refreshes), and in the
  Rust build `RefreshMode::Clean` (GC16 flashing over the given rect alone).
  Accumulating different modes keeps the strongest (FULL > CLEAN > UI > FAST).
- DU wear (issue #11, display.rs `DuWear`): DU turning a pixel from black to white leaves a faint trace
  of the black, and the GL16 settle after it does not reach that pixel (a partial update drives only the
  pixels whose value changed, and DU already wrote its final white). The toolbar drag's release cleans
  its whole trail with one Clean; the erasers and a dragged selection add the area they turned back to
  paper, and past a quarter of the screen the editor's tick sends one Clean of where it was at the next
  pause (as the settle waits: never under a stroke). Writing adds nothing (DU turning paper black leaves
  no trace that matters), and neither do pinch zoom frames, which settle without a flash by design.
- Live stroke: each pen sample batch renders new dabs into page_canvas, recomposes the dirty
  rect, FAST refresh. On pen-up: UI refresh of the stroke bbox so pencil grays come back. A
  press on the panel before that settle settles first, so Undo and Clear answer at once.
- The Kindle display keeps the previous logical frame in physical order (+4.6 MB at 1860x2480).
  `fbpack_row` dithers only changed pixels for a FAST refresh. Unchanged gray image paper stays
  gray in the framebuffer, which DU leaves alone; earlier dithered ink stays as it was until the
  grayscale settle. This avoids turning the paper around a stroke into black and white dots.
  Inversion or rotation invalidates the cache until a whole grayscale frame is copied. If the
  cache allocation fails, live drawing falls back to dithering the whole rectangle.
- Live ink and color panels: a color panel's driver reads the framebuffer for an update some time
  after it was sent, in tiles, so pixels changed again before then show half old, striped. On a
  color panel at 32 bpp (FBInk's `has_color_panel`) display_fbink.c therefore waits after every
  FAST refresh that is not live ink until the driver has read it (`MXCFB_WAIT_FOR_MDP_COMPLETION`,
  as KOReader does; a driver that refuses, or takes over a second, is not asked again that
  session). Live ink never waits, so pen reading never stalls: a stroke's frames only add ink the
  next one sends again, and erasing ends with a grayscale refresh of all it erased. Screens mark
  it with `display_live_ink(bool)` around every pen stroke (writing, erasing, the pen feel pad,
  the calibration's check line); `display_pen_mode(bool)`, the editor's ink only, marks live ink
  and asks for MTK pen mode too. Which FAST refreshes count is one rule in display.h (`LiveInk`,
  `live_ink_mark`, `live_ink_frame`: those sent while a stroke is drawn, and the first within 100
  ms of its end), shared by display_fbink.c and sim_sdl.c, which reports it to scripts
  (`display.last_fast`). A new display.h backend must define `display_live_ink`.
- Toasts are never stored in `App.screen`: `app_flush` overlays one just for the refresh and puts
  the pixels under it straight back. `App.toast_floor` keeps them clear of a screen's controls:
  a screen that keeps the bottom for controls sets it as it draws, and toasts sit above it. The
  Library sets it in `redraw` for an open sheet: the keyboard's top, or else the top of the
  control just under the first band of a toast's height that is clear of the sheet's controls,
  found going up from the toast's usual place (for Custom music, only that sheet's own controls
  count). The Pages view sets it to its bottom row of buttons, or to its keyboard's top while the
  keyboard is up. `app_switch` clears the floor, and a toast already up moves at the next flush
  (`toast_follow`).
- The clock and battery corner (status.rs, the Rust build only) is the third overlay, after the
  Live view chip and before the toast: `app_flush` paints it over `App.screen` when a refresh reaches
  its rect (widening the refresh to hold it whole), presents, and puts the pixels under it back, so
  it is never in a page, an export, a thumbnail, a sleep screen (`app_sleep` presents `App.screen`
  as it is) or a Live view frame (`live_push` comes before it). It is on the notebook page only
  (`wanted`: the setting is on, the editor is the screen and `editor_page_in_front` says no sheet, menu,
  bar, keyboard or Pages view is over the page); `status_tick` takes it down where it is when that stops
  being so, in a refresh of its own rect. It sits in the corner `status_corner` names, a box of the time, a
  battery in pixel art (units a tenth of the text's height: an outline with notched corners, a terminal, a
  bar of up to 12 lit columns, a bolt on a white halo while it charges) and the percent. Its own refreshes
  are one small GL16 of its rect, and of what it covered when its box shrinks, moves or it goes, each a
  refresh of its own (two corners are never one refresh across the screen: `pending` is a few rects, joined
  only where they touch): at each minute and when the battery or its charging changes (`status_tick`), and
  when it is turned on or off, resized or moved. They go through `display_present` with no wait of any
  kind, and a flush that has none of its own carries them (`status_flush`), which then writes the setting
  that changed (a save waits for the storage twice, so it follows the refresh). They are held back while
  the pen is near (any pen event until `IN_PEN_LEAVE`, or until a finger the touch panel lets through: that
  is never the pen's hand), a stroke or another gesture is in progress (`editor_busy`), or the driver is in
  pen mode (`display_pen_mode_on`: it stays so past a stroke until a grayscale refresh is due, and
  that refresh waits for the pen's last frame, so a refresh of the corner must not be the one that
  ends it). A finger tap in the 14 mm corner square, or on the box's place with 4 mm around it (shown or
  not: `status_hit`), turns it on and off: decided at the lift, with the
  finger down in the zone, on no control (`editor_control_at`: the panel, or a link, but
  not one in the corner), lifted in the square (or within 5 mm of where it landed) and under 800 ms, whatever the fingertip did between (the path is not asked: a hard press drifts); every decision is one `grimoire.log` line; the screen
  sees the touch as ever, and the corner only watches (`status_tapped` tells it at the lift, so no link is
  followed). `display_skip_status` leaves its rect out of the screen guard, as `display_skip_overlay` does
  the chip's.
- Live view's chip (`live_chip`, `live_chip_sync`) is the second: on a notebook page it is beside the
  editor's panel (`editor_live_chip`: left of it, level with its header, or right where the panel is at the
  screen's left edge) and follows it, with none while a menu is in front; on the other screens it is in
  the top right corner. A flush moves it (the pixels it left come back from `App.screen`).
- The panel is under every menu (`compose`: the page, the areas, the panel, then `draw_menus`: Paper,
  Templates, Display, Custom music, the tool picker, Find with its keyboard and the Links bar). What draws
  the panel alone (a tool picked, the panel dragged, folded or fitted, a settle) goes through
  `draw_panel_over`, which draws the menus over the part they cover again; `compose` draws the menus over the
  whole panel, not only over the rect it was asked for. The Pages view covers the screen and the panel is
  not drawn on it, and Live view's browser layer of the panel (`editor_live_panel`) is empty while any of
  them is up.
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
- The eraser removes whole strokes it touches, or with `eraser_mode` 1 (Precise) draws erase
  strokes (below); one pen-down..up is ONE undo step. The eraser end of the pen
  (BTN_TOOL_RUBBER) and holding the side button (BTN_STYLUS) erase regardless of the selected
  tool.

## Erase strokes (the engine for white ink)

The page format has erase strokes from 1.1 on (docs/FORMAT.md, Erase strokes), and the renderer
draws them. White ink and the precise eraser make them.

- Stored like any stroke, with tool 3 (`TOOL_ERASER`, `stroke_is_erase`). The seed's top byte
  holds the texture, the drawing tool whose look it has + 1; 0 is the round hard tip, and so is any
  other byte (4, the eraser's own, or any value past this version's tools). The brush hashes only
  the low 24 bits (`erase_seed` makes such a seed).
  `stroke_render_tool` gives the tool whose look and reach it draws with, from the seed, so the
  reader, `scale_page` and the editor know its bbox before its first point. White ink keeps
  everything of its tool but the color: width, nib, grain, pressure and tilt.
- Blend: besides the darken-only deposit there is a lighten-only restore toward the paper under
  each pixel (`canvas_restore_px`), never past it; a pixel a dab covers fully (coverage 0.999 or
  more) lands exactly on the paper. An erase stroke replays in stroke order like any other, so it
  takes away ink drawn before it, keeps the paper's lines (rules, staves, grid, dots) and leaves
  later ink alone.
- The paper comes with the brush: `brush_set_erase` takes the paper under the canvas (NULL is
  white, right only for a canvas of ink alone). `stroke_brush_begin` starts a stroke's brush for
  drawing live and for a replay alike, so live drawing still equals re-rendering, as long as the
  live brush restores toward the same paper.
- `page_render_layers` with both layers copies the freshly drawn paper only where erase strokes
  reach into the region (the union of their bboxes there) and replays the erase strokes clipped to
  that copy; out of memory, it renders the region in two halves, each with a smaller copy (a
  single pixel restores toward white, logged once). Without the paper layer (the share studio's
  `?layer=ink`) erase strokes restore toward white. Never draw a page in two passes (the paper
  layer, then the ink layer over it on the same canvas): the ink layer restores toward white and
  would wipe the lines under every erase stroke. src/page.h says so.
- One write helper, `put_px`, serves ink and erase. The ink path keeps its own copy of the dab
  loop (`stamp_as` with a constant `erase` flag), with no erase test in its pixel loop. A round,
  grainless, full-opacity erase dab restores the core of each row in one pass (`restore_span`),
  byte for byte the per-pixel path's: tests/test_erase.c compares the two on 1500 random dabs and
  on whole pages (`g_brush_erase_spans` turns the fast path off). Erase dabs skip pixels already
  at their paper before working out coverage, which changes no byte and makes a scrub over an
  erased area cheap.
- `BrushState.restored`: the stroke has lightened a pixel of the canvas it drew on, that canvas
  only. Over ink that the page's darker lines hide, an erase stroke can leave the page as it was and
  still change the ink layer, so whether a stroke may go is `page_droppable`'s to say.
  `BrushState.dry`: dabs report the rect they would draw into and draw nothing, the brush moving on
  exactly as when it draws.
- The stroke eraser draws the page again only where the strokes it took off drew (2.0.0-rc.12,
  `page_render_tiles`, page.rs). A pixel is its paper and the dabs over it, one after another, so
  taking strokes off changes the page only under their dabs: `stroke_mark_tiles` marks the 16 px
  tiles their dabs reach (a dry run, `TileMask` in canvas.rs), and their boxes are drawn again only
  in those tiles, over the page canvas, which keeps every other pixel. The paper goes down a run of
  marked tiles at a time; a stroke whose box meets no marked tile is left out; a brush given the mask
  (`BrushState.mask`) skips a dab in no marked tile and draws the others a run at a time, each pixel
  as it would without the mask. tests/test_erase_tiles.rs compares it with fresh drawings on dense
  random pages; `editor.erase_tiles` counts the tiles. On the owner's 1,090-stroke page (Mac
  timings) the largest marker stroke, whose box is the whole page, draws 13 times faster, the longest
  charcoal stroke 7 times, a sweep of the eraser 2.6 to 4.4 times. Every dab that draws nothing (out
  of the clip or the mask) also skips the work only its pixels need (a powf or two), and the spacing
  takes the dab's own radius instead of a second `brush_dab`.
- Compaction, `page_droppable` (page.h): erased ink stays in the page file, in the strokes before
  the erase stroke, until something leaves it out. It marks the strokes that can go without
  changing a pixel of the page or of its ink layer: ink that later erase strokes restore entirely,
  and erase strokes that change nothing, each checked with those marked before it already left
  out. A stroke erased only in part stays whole. The strokes are cut into pieces of about 16 mm,
  each with a brush checkpoint and the rect its dabs reach (measured dry); the page as it is is
  drawn once, in 16 px blocks, where a check first needs it; each candidate is checked by drawing
  only its rects without it, and the check stops at the first difference. Erase strokes are checked
  only where they meet earlier ink not marked, and the ink layer only where the paper has marks.
  The marks are exactly those of the rule checked the slow way, stroke by stroke over the whole
  page (tests/test_erase.c, `compaction_is_the_rule`). It only reads the page (the editor's undo
  history holds pointers to its strokes): leave the marked strokes out of a copy. Never call it on
  the UI thread: on the device a heavily erased page can take tens of seconds, past the watchdog's
  15 s. Its read-only contract suits a forked child, as bgjob.c runs exports.
- Memory: a render whose region erase strokes reach allocates a copy of the paper there, up to the
  region's size (4.6 MB for a whole 1860x2480 page, in an export's child too); `page_droppable`
  needs two page-size canvases (one on white paper) and the pieces of the strokes it draws.
- tests/test_erase.c holds `k_look`, CRCs of erase-stroke renders recorded from the 1.1 engine
  (`GRIMOIRE_GOLDEN_PRINT=1 build/test/test_erase` prints a fresh table). FORMAT.md freezes how an
  erase stroke draws from 1.1 on: re-record it only for a change meant to alter that look before
  1.1 ships, never after, and never to make another change pass.

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
  `font_face_has_glyph(face, cp)` tells whether a face has its own glyph, without the Terminess
  or DotGothic16 fallback; `font_has_glyph(face, cp)` also counts those fallbacks.
- The keyboard's text is UTF-8 and its keys carry code points (`KB_KEY_*` below 32 for the named
  keys, `KB_KEY_FLICK` + row for the kana pad's own keys). There are four pages of characters,
  letters, symbols, accents and kana (`Keyboard.page`); the first three have `-`, `'` and `.`
  beside the space bar and two page keys, #+= to the symbols page and áé to the accents page, each
  labelled abc on its own page to go back to the letters (Go to page's number pad is a fifth page).
  In Japanese the letters page's accents key is あ instead (`KB_KEY_KANA`: the accents page has
  nothing a Japanese name needs), and the kana page is a phone's flick pad: twelve keys in three
  columns, あ か さ / た な は / ま や ら / わ ゛小 ー, where a tap types the row's first kana and a
  swipe left, up, right or down the rest (`k_kana`: tap, left, up, right, down, 0 where the row has
  none, and a row with nothing that way types its first). や keeps a phone's brackets on its
  sides, ゆ above and よ below; the bottom row's 、 key holds 、。？！・ on its flicks. Its keys
  type on the lift, not on the touch-down, since which kana it is is the swipe: `Keyboard.flick`
  holds the way the pressed key is swiped (0 still on it), the key's label shows that kana while
  the finger moves, and the pen may end well off the key. One modifier key (`KB_KEY_KANA_MOD`, ゛小)
  rewrites the last character of the text instead of adding one (`change_last`, `kana_cycle`):
  unvoiced, voiced, half-voiced, small, round again (か が か; は ば ぱ は; つ づ っ つ; や ゃ や),
  which covers dakuten, handakuten and the small kana in one key. カ (`KB_KEY_KATA`) switches the
  pad to katakana and back (`Keyboard.kata`; the labels follow), and it types on the lift too.
  Every kana the pad types, both scripts, and 、。？！・ー゛小カ, is drawn in every theme:
  DotGothic16 covers what the pixel and Terminess faces lack. `keyboard_upper()` is the shift case map (Latin-1 and Latin
  Extended-A), `keyboard_key_rect()` locates a key on the page showing (for tests and scripts), and
  `keyboard_open` keeps valid UTF-8 and drops control characters and broken bytes. A key types a whole character; backspace removes one. In Spanish
  the letters page's middle row ends with ñ after l (`k_row_es`), as Spanish keyboards have it, and
  is then as wide as the row above; the accents page keeps ñ in every language.
- Every keyboard host (the Library's sheet, the Pages view's Page name and Go to page) hands its pen
  and finger events to `app_keyboard_pointer` and its tick to `app_keyboard_tick` (app.c), so a key
  costs the same over any screen. A sample that changes nothing (the pen resting on a key sends one
  at every digitizer frame) draws nothing; a key that types draws the keyboard once and refreshes
  the text line on the fast waveform, flushed on its own, then the key; 900 ms after the last key
  (`Keyboard.settle_at`) one GL16 of the keyboard cleans up the dithered keys. Nothing under the
  keyboard is drawn until it closes, and a settle the editor had due on the page (a stroke's GL16,
  the panel's) is dropped while the Pages view covers it: the view's opening refreshed the whole
  screen and its closing composes it again. The Pages view used to refresh the text line and the key
  as one fast rectangle from the text down to the key and to draw the whole keyboard at every
  sample: on the Kindle each key's refreshes then overlapped the one before and waited in the
  driver's queue for it (docs/INK-PERFORMANCE.md), and the text fell further behind with every key.
  tests/scenarios/j_pages_keyboard.qs holds both keyboards to the same refreshes and draws.

## Languages

- English is in the code, and every string a person sees goes through i18n.h: `_()` looks one up
  now, `N_()` marks one in a static table (looked up where it is drawn), `C_(ctx, s)` and
  `NC_(ctx, s)` add a context that tells two meanings apart ("paper", "tool", "month", "date",
  "key"...), and `NG_(one, many, n)` gives the plural form for n. Each language is
  `po/<lang>.po`, compiled by msgfmt to `po/<lang>.mo`, which is committed (building needs no
  gettext) and embedded like the fonts: `build/gen/l10n.c` holds every po/*.mo, named by its file
  (po/es.mo is "es", po/ja.mo is "ja"). The list of languages (code, name in the language itself,
  number of plural forms, plural rule) is `k_langs` in src/i18n.c, English first. Japanese has one
  plural form (`plural_zero`); its kana and kanji come from DotGothic16, the embedded fallback.
- Lookups (i18n.c) read a GNU .mo as msgfmt writes it, in either byte order: a binary search over
  the English strings, a context stored as `ctx\004English`, a plural's forms one after another.
  A string the catalog lacks, or leaves empty, gives the English: the very pointer the code passed.
  A catalog is checked whole when its language is set (`catalog_open`: the tables and every string
  inside the file, the English in order, or a search could miss), and one that does not read is
  refused whole: its language is English, logged as `i18n: the catalog for es is damaged; using
  English`.
- The compiler checks every printf against the English: the lookups are declared `format_arg`.
  Clang heeds one per function, so `NG_` is a `?:` of two lookups, one checked against each English
  form (`i18n_ng`, never 0 and extern, keeps the `?:` from being folded away). msgfmt --check (make
  l10n, make l10n-check) and tests/test_i18n.c check each translation against its English: the
  same arguments, of the same types, positions allowed (all numbered or none).
- Plural rules are written by hand in i18n.c (`k_langs`); a catalog's Plural-Forms line is only
  data. tests/test_i18n.c holds that line to `k_rules`, the test's own copy of each rule, kept in
  step with i18n.c by hand; the function in i18n.c is checked only by the `plurals` test's counts
  (Spanish at 0, 1 and 2), so a new language adds such checks of its own.
- Which language: settings.ini `language=`, `auto` by default (a code this version does not speak
  works as `auto` and goes back into the file as it was). For `auto`, `i18n_detect` at startup takes
  the first that is set of `GRIMOIRE_LANG`, on the Kindle the `LANG=` line of
  `/var/local/system/locale` (its `LC_ALL=` line without one), then `LC_ALL`, `LC_MESSAGES` and
  `LANG`; a language Grimoire does not speak is English. main.c sets the language before anything
  is drawn or said (`settings_language` reads the one line), and Settings > Language applies a
  pick at once (`i18n_apply`). docs/KINDLE.md has the log lines.
- What stays English: identifiers. `paper_name`, `tool_name` and `music_part_name` are names for
  the API and the log; the screens show `paper_label`, `tool_label` and `part_label`. English too:
  settings keys, tool keys, file and folder names, log lines, the studio's preset keys and the
  magick command, and everything drawn on a page: paper is the same in every language, Cornell
  headings included, so pages, PDFs, PNGs, thumbnails and grimoire-export's renders never depend
  on it (tests/test_i18n.c, `paper_is_the_same_in_every_language`). Names people give stay as
  typed.
- How strings are written: whole sentences, with %s for values (APP_NAME, the Kindle's folders,
  the pull command), never pieces glued together. A choice is one sentence per value (the pen
  button's rows, the sleep screen's rows). Dates go through `format_time_short` and
  `format_date_short` (util.c), whose formats are translated too (`C_("date", ...)`, reordered by
  position; the months are `C_("month", ...)`). TRANSLATORS comments say what a string is and how
  much room it has, as px at a size ("284 px at 34 px"). xgettext attaches a comment only to the
  first marked string after it, so a group's comment (tools, papers, music parts, Pen feel's
  sliders, months) sits on the group's first entry.
- Text that fits, since a translation can be longer than the English. `font_draw_wrapped` and
  `font_wrap_lines`: up to N lines broken at spaces (a word wider than the line is broken between
  its characters); two lines are balanced, unless a break right after punctuation makes the longer
  line at most half again as long ("Fino, sin margen" breaks after its comma). `font_fit_px`: the
  largest size that fits, among the sizes the face really has (Jersey 10 has whole steps only, 44
  px to 34 px being the one that counts). `ui_text`: one line, else smaller, else lines. Toasts
  take two lines where one would be cut (`ui_toast_rect`). `ui_chip_w`: a chip grows with its text
  and keeps it 43 px from Gothic's pointed ends (for a chip 100 px high). `ui_button_need_w`: a row of
  buttons widens to its widest need (the Library's top bar, up to 220 px; hosted, Back to KOReader
  takes only its own room). `ui_button_text_px` and `ui_button_text_at`: a grid of buttons draws
  every label at the smallest size any of them needs (Custom music's parts). In Gothic a label
  keeps 4 px inside a button's carved inner line (10 px in, 13 for a primary button, 2 px wide): a
  170 px button holds 138 px of label (132 primary), in Classic 154. `i18n_copy` copies translated
  text into a fixed buffer, cutting only between characters (toasts and calibration's notes).
- The fit check: `font_set_overflow_hook` hears of every text cut short (`font_draw_centered`,
  `font_draw_left`, `font_draw_wrapped`) and of a word broken between letters. Names are not
  reported: `font_draw_left_name` (a notebook's, a folder's, Settings' data path) shortens with
  "..." by design. With `GRIMOIRE_CHECK_FIT=1` the simulator and the headless runner list each cut
  (`fit[es]: "..." needs W px, has R`, once per text and room) and exit 3, and make l10n-check
  runs every screen so, in every language and style (docs/SIM.md). Text no scenario reaches (rare
  notices, the Share screen without an address, Pen feel's raw labels) is measured in
  tests/test_i18n.c.
- The share pages: httpd.c writes `<html lang=..>` and escapes every translation as text, as it
  does names (a catalog is third-party input, and none may add markup; share_check serves the pages
  from a catalog full of markup). /api/list.json keeps the English identifiers. The studio's words
  are web/studio.js's: `t()` for words shown, never for keys, file names or the magick command;
  `N_` marks the preset names, which are keys too; `tPages` gives the page-count plural. `make
  l10n` extracts them under the context "studio", and httpd.c sends that context to the page as
  `window.GRIMOIRE_L10N` (English to translation, through `json_str`), each plural in its form for
  the notebook's page count (`i18n_getnc`). Gothic's bold on the web is a text-shadow, which fills
  in accents, so letters past ASCII go in spans of class `na` without it (`html_esc_bold` in
  httpd.c, `setText` in studio.js).
- Improving a language or adding one: docs/TRANSLATING.md.

## On-disk layout (root = /mnt/us/grimoire on Kindle, $GRIMOIRE_ROOT or ./grimoire-data in sim)

```
root/settings.ini              current tool, panel, every tool's params, calibration and the
                               panel it was made on, sleep, style, language, port... (unknown
                               keys kept)
root/folders.ini               the folder names, one per line
root/sleep.pgm                 the pinned sleep screen (P5)
root/notebooks/<id>/notebook.ini   name, folder, paper, created, modified, last_page,
                                   next_page_id, pages=<id,id,...>, page_w, page_h (unknown
                                   lines kept)
root/notebooks/<id>/p<pageid>.qpg  binary page (see docs/FORMAT.md)
root/notebooks/<id>/p<pageid>.qpg.damaged-<time>   a page that could not be read, kept
root/notebooks/<id>/thumb.pgm      binary PGM thumbnail of page 1 (P5)
root/trash/<id>/               deleted notebooks are moved here, never unlinked
root/PDFs/<folder>/<name>.pdf  the PDF mirror (an export, not data)
root/.passcode-paused          legacy beta.1 recovery marker (new sessions never create it)
root/.passcode-left-off        how many starts were told a restart left the passcode off
root/.koplugin-installed       the build whose KOReader plugin went into koreader/plugins, or
                               "removed" (Kindle integration); a Library start that finds no
                               KOReader deletes it
```
All writes are atomic: write `<file>.tmp`, fsync, rename. Page saves happen about 2 s after the
last pen-up, at every press of the power button, on page change, on leaving the editor and on
exit. `store_add_page` never writes over an existing `p<id>.qpg`: it takes the next free id.

The page size: a new notebook gets the device's (`store_set_device`), and
`store_notebook_page_size` is what every reader and writer of its pages asks. A notebook.ini
without a size (or with one outside 100..6000) takes the size most of its page files have, by
their first 16 bytes, a tie going to the first in page order (`notebook_size_from_pages`, which
stops reading once one size has more than half the pages); with no readable page, the device's.
The next save writes it, and the Library's listing writes it once for a notebook whose size it had
to find. The editor reads pages with `store_load_page` at that size and exports with
`store_read_page`: a page file of another size has its points scaled to it (`scale_page`, logged
once per run), and a missing page is blank paper of it. The Custom music sheet counts systems for
the page it is for (`musicbuild_open_page` carries that page's size, and `music_max_systems` takes
its height). Calibrations follow the panel the same way: at startup `app_init` passes each through
`calib_to_panel` (calib_fit.c). It takes the record (`pen.panel`, `touch.panel`; none
means an 1860x2480 Scribe's) by pointer, decides by the calibration's own scale which panel it was
made on (the first it spans of the recorded one, this one, 1860x2480 and 1986x2648; spanning none,
the recorded one), scales it to this panel if that was another, sets the record to the panel it
took, and says what it did (`CALIB_SAME`, `CALIB_SCALED`, `CALIB_MADE_HERE`). app.c logs that and
records this panel, written at the next settings save. docs/FORMAT.md has the rules,
docs/KINDLE.md the log lines.

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
  1.1's keys are of that kind: without `page_w`/`page_h` a notebook takes the size most of its
  pages have, without `pen.panel`/`touch.panel` a calibration counts as an 1860x2480 Scribe's
  (and its own scale has the last word), and without `language` the language is automatic.
- The stroke parameter ranges (`k_params`) are frozen in both directions while 1.0.0-beta or
  1.1 is in use: a wider value travels in the ext bytes or a critical chunk. A safe
  (lowercase) chunk must never depend on page coordinates or the page size.

## Kindle integration

- Scriptlet `documents/Grimoire.sh` (hdnext jailbreaks; KUAL is obsolete there) carries the app:
  it unpacks it into `/mnt/us/grimoire/app` when its BUILD_ID differs, then detaches
  `app/grimoire.sh`, which: sets `preventScreenSaver 1` while Grimoire starts (Grimoire turns it
  off again unless Sleep after is never), hides pillow (status bar), SIGSTOPs the framework
  processes so nothing else draws or eats input, preserves the Device Passcode (below), runs the
  binary, and on exit restores everything (SIGCONT, pillow, screensaver) and starts the
  home booklet. On a locked Kindle (power service state `screenSaver`, `readyToSuspend` or
  `suspended`) the restore does not write the screen saved at the start back: it would cover the
  screensaver or the passcode prompt. A start from the Library maintains an explicitly installed Grimoire plugin
  (`root/.koplugin-installed`). Fresh installations leave KOReader untouched; Settings > KOReader
  opts in. Deleted plugins stay deleted. A KOReader reinstall needs consent again.
- Both launchers refuse a plain start inside KOReader (exit 3): `KOREADER_DIR` set, or `pidof
  reader.lua` succeeding. KOReader holds the pen and the touch screen, and Grimoire's exit would
  wake the Kindle UI underneath it. KOReader's way in is its Grimoire plugin
  (kindle/koreader/grimoire.koplugin), which hands over in KOReader's own exit path. It wraps
  `Device.exit` for one exit and asks for KOReader's restart. KOReader's `Device:exit()` is the
  only place it lets go of the pen, the touch screen and the screen; after it, the wrapper runs
  `app/grimoire.sh --hosted koreader` in the foreground, and reader.lua's exit code 85 then makes
  `koreader.sh` start KOReader again in the same session, whose own start reopens the book.
  koreader.sh keeps the Kindle UI paused across the restart, so nothing cycles the framework: no
  home screen shows in between, and the passcode stays paused. It fails safe: the wrapper runs in a
  pcall, so a failure still ends in code 85, and a launch marker that outlives the hand-over tells
  the user at KOReader's next start. Hosted mode refuses unless reader.lua, started by koreader.sh,
  is among the launcher's parents, with koreader.sh among its ancestors, and has let go of
  `/dev/input`. If KOReader left the native UI running, Grimoire owns its pause/restore. It keeps the launcher's lock, priority, firewall hole, watchdog and log, and leaves the screen
  dump, pillow, the home booklet, volumd, lab126_gui and the passcode to KOReader. In the app,
  `GRIMOIRE_HOST=koreader` makes Exit read Back to KOReader and sleep mode 2 show the page.
  `Grimoire.sh --unpack-only` and the `# Capabilities:` lines (`hosted` among them) are for the
  plugin; docs/KINDLE.md has the flags, the exit statuses, the hand-over step by step, the
  plugin's install and how a tool reads the Capabilities lines.
- The native Device Passcode remains enabled on a Library start. On sleep the app yields
  pen/touch grabs and resumes the native UI; only `outOfScreenSaver` takes it back. Input received
  while yielded is discarded before coordinate logging. Reacquisition drains stale events and
  resets contacts, and the UI commands have an ordered three-second bound. Native PIN forces
  the stock sleep/lock screen and prevents stop-framework mode. Old beta.1 recovery markers are
  still consumed without guessing a missing PIN flag. A KOReader session normally already disabled
  PIN; Grimoire inherits its host's policy. See KINDLE.md for the boundaries and device test status.
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
- Hosted by KOReader, sleep mode 2 (the Kindle's own screensaver) needs the Kindle UI, which
  KOReader keeps paused and the launcher never wakes, so `sleep_mode` in app.c turns it into mode
  0, the page (logged once). The sleep screen shows any page fitted to the screen, whatever its
  notebook's page size.
- The pure logic and its tests: src/power.c (tests/test_power.c) and src/fbcheck.c
  (tests/test_fbcheck.c).

## Build

- `make sim` (macOS/Linux, SDL3) -> build/host/grimoire-sim
- `make headless` (no SDL) -> build/host/grimoire-headless; `make scenarios` runs every
  tests/scenarios/*.qs. Scenarios check the app's state with `expect` lines (docs/SIM.md): a
  failed check, an unknown script line or a leftover `*.tmp` fails the build once every scenario
  has run, and a headless run waits for the PDF exports it started. A `# root: NAME` line starts a
  scenario from another scenario's data (d_persist, x_turn_calibrated and y_pagesize_1_ksc use
  b_library's, s_passcode_once r_passcode's, y_pagesize_2_back y_pagesize_1_ksc's, z_language_kept
  y_language's), `# env: K=V` lines set its environment (`GRIMOIRE_PASSCODE` in r_passcode,
  s_passcode_once, t_passcode_left_off and z_annotate_f_passcode, `GRIMOIRE_BG_DELAY_MS` in
  v_leave_from_editor, `GRIMOIRE_HOST` in y_koreader_hosted and y_koreader_not_hosted,
  `GRIMOIRE_LANG` in y_language and z_language_kept, `GRIMOIRE_SIM_SIZE` in y_pagesize_1_ksc,
  `GRIMOIRE_CHECK_FIT` and `GRIMOIRE_SIM_ADDRS` in y_all_screens), `# settings: key=value` lines
  go into its settings.ini, and a `# pdfs: N` line wants exactly N PDFs in its root's PDFs/ folder.
  The rule unsets the test hooks first, so nothing else of the caller's reaches them, and runs
  every scenario in English (`GRIMOIRE_LANG=en`, which ranks first) unless its `# env:` line names
  another language. Every scenario's share server gets a port of its own: GRIMOIRE_TEST_PORT (else 8765) plus its job's slot.
  tests/selftest/pass.qs and fail.qs check the runner itself.
- `make l10n` (for the maintainer, with GNU gettext): xgettext collects every marked string of
  src/*.c, with the TRANSLATORS comment above it, and web/studio.js's (`t`, `tPages`, `N_`) under
  the context "studio", into po/grimoire.pot; msgmerge brings each po/<lang>.po up to date with it,
  and `msgfmt --check --no-hash --endianness=little` compiles each to po/<lang>.mo (the same .po
  makes the same .mo on any machine). `make l10n-check`, part of `make check`: each committed .mo
  must be what msgfmt makes of its .po now (skipped, and said so, without msgfmt), then
  tests/scenarios/y_all_screens.qs runs in every language (English and each po/*.mo) and both
  styles with the fit check, then tests/l10n/long_root.qs (Settings from a data root longer than
  the sheet is wide) in each language, then tests/l10n/hosted.qs (the top bar as KOReader opens
  it) and tests/l10n/templates.qs (the Templates sheet with templates in it, its `# copy:` line
  honoured) in each language and style. docs/TRANSLATING.md and docs/SIM.md have the rest.
- Dependencies: HDRS holds the Makefile, VERSION and third_party/qrcodegen/qrcodegen.h besides
  src/*.h, so a change of flags rebuilds every program (FBInk's objects follow only their own
  .c files); the test binaries also depend on tests/*.h (beta_format.h among them), and the zip on
  the Makefile. TEST_FLAGS pin `-ffp-contract=on`, because tests/test_golden.c's table holds only
  with contraction on (clang's default, under which it was recorded).
- `make check` -> everything verifiable without the device: the unit tests, the scenarios, the
  share server (tests/share_check.sh), the host exporter, the ARM build, the zip: package-check
  (tests/package_check.sh, what the zip holds, and tests/koplugin_check.sh on the plugin the app
  carries) and launcher-check (tests/launcher_check.sh: both launchers run against stand-ins for
  the Kindle's tools in tests/shims, under each POSIX shell it finds), and l10n-check.
- `make test` (host, ASan+UBSan) -> runs tests/test_*.c. The test builds, the simulator and the
  headless runner stop at the first undefined behaviour (`-fno-sanitize-recover=undefined`).
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
  export_mirror_pdf; spanish_letters_have_enye holds the Spanish letters page.
- tests/test_power.c and tests/test_fbcheck.c test src/power.c and src/fbcheck.c, tests/test_qr.c
  the QR drawing, and tests/test_probe.c that the probes' tool names match settings.ini.
- tests/test_i18n.c: lookups, contexts and plurals in the embedded catalogs; every translation
  against its English (the same printf arguments by type, the language's number of plural forms,
  only glyphs that Jersey 10, Jacquard 24 and Terminess regular and bold all have) and the
  catalog's Plural-Forms against `k_rules`; damaged catalogs refused whole; the choice of language
  (settings.ini, then `GRIMOIRE_LANG`, the Kindle's locale file, `LC_ALL`, `LC_MESSAGES` and
  `LANG`); dates; paper that never changes with the language; the layout helpers (the overflow
  hook, wrapping, fit sizes, button labels inside Gothic's carved line, button grids, chips,
  two-line toasts); and the widths of text the every-screen scenario cannot reach (rare notices in
  both styles, the Share screen without an address, Pen feel's raw labels).
- tests/test_pagesize.c: notebooks keep their page size, pages an older version scaled come back
  to it, the size found from the page files (the size most have) and recorded by a listing,
  calibration scaling and the panel records in settings.ini, music layouts at both page heights,
  and which fast refreshes are live ink. tests/test_erase.c is the erase-stroke engine's (Erase
  strokes above).
- tests/koplugin_check.sh (run by package_check on the plugin the app carries): every Lua file
  parses (luajit, else luac), and with luajit tests/koplugin_test.lua plays the plugin against
  stand-ins for the KOReader modules it uses, down to reader.lua's `Device:exit()` and KOReader's
  next start; luacheck has its say when installed.
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
  still runs. It runs hosted starts under tests/shims/fake-koreader (a reader.lua whose parent is
  koreader.sh, in a scratch `/proc`) and each of their refusals, another Grimoire's lock (exit 1,
  awesome stopped again first), `Grimoire.sh --hosted` inside and outside KOReader, and the plugin's
  install, update, removal by the user and KOReader's reinstall.
- tests/share_check.sh also serves the pages in Spanish, and from a catalog of markup
  (tests/l10n_markup.py puts tags, quotes and an entity before every translation of po/es.mo,
  read through `GRIMOIRE_L10N_DIR`): the index, a notebook's page and the studio must show it as
  text. tests/studio_copy_check.js (with node) tries the studio's Copy command where the browser
  copies and where it will not.

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
   Kindle's style (served from the binary, cached a year) and the language in use (Languages
   above);
   every page is rendered at its notebook's page size (`store_read_page`), whichever Kindle
   renders it, and `X-Page-Size` says that size. The
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
   **Live view** (2.0.0-beta.5) is the same server's other owner (`httpd_acquire`/`httpd_release`:
   the server runs while the Share screen or Live view keeps it; every route but `/live` and the
   fonts needs Share, and answers 403 "Open Share on the Kindle to download." while only Live view
   runs). Share's Live view button turns it on and shows the pairing card (the Share screen in its
   live mode): the address of `/live`, a QR code of `/live/pair?t=<token>` that pairs the phone that
   scans it, and a six-digit code a computer types (pair.c: single use, a new one after five wrong
   tries and a 30 s lock, sessions of 128 random bits in an HttpOnly SameSite=Strict cookie, all
   forgotten when Live view stops). The card closes by itself once a browser pairs.
   While Live view is on, `app_flush` and `app_press_feedback` copy each rect they present into a
   shared frame (`live_push`: a memcpy of the rect, before the toast is drawn, so neither the toast
   nor the chip is ever streamed) and mark its 64 px tiles, for each viewer slot. A paired
   `GET /live/stream` is handed to a viewer child (two at most; `live_take_viewer`), which owns the
   socket: it takes its marks, compares the tiles with what it sent, sends each stretch of changed
   tiles in a marked run as a gray PNG (a row redrawn whole for one tile sends that tile) and a
   batch end, at most about 30 batches a second, a ping after 15 s idle. A slow browser only lets
   the marks pile up; the pen loop never waits. The socket itself holds little, so a busy link
   never fills with pictures already out of date: a 64 KiB send buffer, at most 32 KiB not yet
   sent (TCP_NOTSENT_LOWAT), and a batch starts only while the socket is writable and holds under
   48 KiB, sent or not, unacknowledged (SIOCOUTQ; SO_NWRITE in the simulator). Until then the marks
   wait and gather, and the batch that goes reads the frame then: the newest pixels go, the ones
   between never. A browser that takes none of it for 10 s is gone. While the pairing card is up
   the state is LIVE_PAIRING and nothing is sent: the card's code never reaches a stream (the
   viewer checks the state again after copying each run, and the app sets the state before the
   card is drawn). Asleep
   it is LIVE_ASLEEP (the sleep screen is never pushed); a turn bumps the screen's epoch, and each
   viewer sends a new hello and every tile. The state frame also says whether Grimoire's colors are
   inverted (`live_set_invert`, from settings.invert at every tick: the frame holds the screen as
   drawn, since the inversion is applied as it presents) and the front light's warmth
   (`live_set_warmth`: `light_warmth()`, as read once at Grimoire's start or as the Display sheet
   last set it, at every tick, so the main loop never waits on lipc for it; 255 without warm light);
   every stream starts with one. The browser's picture follows both until its own Invert colors
   switch or Warmth slider says otherwise: inverted, the whole page is black; warmth tints the
   picture's light (the paper, or the ink when inverted) from white at 0 to 2400 K at 24, in even
   steps of mireds, in the pixels so a recording has it. Its Brightness slider (10% to 100%,
   remembered by the browser in localStorage, never sent to the Kindle) dims that light the same
   way, warm or not: black stays black, and on transparent paper only inverted ink has light to
   dim. A slider's step only asks for a frame, which draws the whole picture once however many
   steps came, and nothing sets a slider a pointer holds: a state frame meanwhile changes what the
   Kindle has, not the thumb, and a warmth let go on the Kindle's follows the Kindle's again. The page itself is dark, as every share
   page is (httpd.c's :root colors), around a screen drawn as the Kindle draws it. The canvas on
   the page has a pixel for each of the screen's and is filled by averaging (web/live.js present():
   each pixel the alpha-weighted mean of the area of the full-size picture it covers, for the rect
   that changed): the browser's own scaling, CSS or drawImage at any quality, picks pixels, and
   1 px ruled lines came out bright, dim or gone (measured in Chrome at 567 x 757: each line's ink
   0 to 255 that way, 155 to 156 averaged, as the exact average's 156). The pen is drawn over the
   picture, never in it. Each pen event's place (away, hovering, touching; never asleep) goes to
   `live_pen` as it is read (`live_capture`, below), one atomic word in the shared frame (state in
   2 bits, y and x in 15 each), so a sample costs a store; only the pen coming into range writes to
   the tile viewers' wake pipes, and while it is in range each viewer looks again every 33 ms
   itself. On the Kindle the pen service relays the pen coming and going, and where it touches, but
   not where it hovers (no X or Y between the lift and the leaving, in the raw pen log of
   2026-10-26), so while Live view is on the app is handed the raw digitizer's place at each of its
   frames while the pen is near and not touching (`input_hover_hook`, and with fast ink's
   `input_sample_hook` set an IN_PEN_HOVER sample that is never queued, which needs no hover hook):
   the raw axes scaled onto the screen as the pen service scales them (8.4 units to a pixel), each
   as it is until contact frames of the same moment (within 4 ms) show it reversed, and none while
   even the best orientation is more than 8 px from the pen on average (pensync.h); grimoire.log
   names the orientation once it is decided. A tile viewer reads the word with the state, before
   the marks, and sends it after the tiles (frame 6: u16 x, y, u8 state), only when it changed and
   at most every 33 ms, so a place is never ahead of a push before it; a hello or any state but
   awake takes it away (asleep, the pairing card, a turn). It is the legacy cursor now: the page
   follows it only while it has no `/live/pen`, whose samples say sooner where the pen is. The page
   draws a red dot, see-through (55%) while it hovers and smaller and solid while it touches,
   inside a light edge and a dark one so it shows on white, gray, black and inverted pages, on a
   canvas over the averaged one (Show pen, on by default; the Mac simulator never says the pen
   left, as its mouse leaving the window is no IN_PEN_LEAVE, so there the dot stays where it was
   last). Record takes a third canvas, film: the full-size picture with the pen's mark drawn on
   it, kept as tiles and pen frames come (not by animation frames), so recording continues in a
   hidden tab. Picture in picture does not use film (see below).

   **Fast ink** shows what the pen writes in the browser before the tiles can: its samples come on
   a stream of their own (`GET /live/pen`, paired as `/live/stream` is), the page draws a quick
   line from them at once, and the tiles' real ink takes its place as it comes. A pen viewer child
   (two at most, `live_take_pen_viewer`; a browser with both streams counts once in
   `live_viewers`) makes no PNG, keeps a 16 KiB send buffer and a 1 s send timeout, marks its packets low delay
   (DSCP EF; on the Kindle SO_PRIORITY 5 too), which mac80211 and most Wi-Fi drivers send in WMM's
   video queue ahead of the tiles' best effort, runs at the ordinary priority where the tile
   viewers are niced, and sends what came as soon as it wakes: no
   PNG being made, full tile socket or 33 ms between batches is ever on its way. live.h
   (rust/src/live.rs's head) has the version 3 protocol byte by byte. `input_sample_hook` hands
   the app each pen event as it is read, before it is queued, the reads `work_pump` makes during
   long work too, and the raw hover above. `live_capture` (app.c) stores it with its ink style in
   a ring of 512 in the shared frame (`live_sample`: its sequence, the kernel's time, place,
   pressure, tilt, buttons and style; each record stamped zero while it is written and with its
   sequence after, its words atomic, so a viewer that falls a whole ring behind rejects a torn one
   and the writer never waits), which is all the pen path pays: no allocation, no log line, and one
   byte down each pen viewer's wake pipe per read of the pens (`live_wake_samples`). A viewer packs
   every sample that came since its last wake into one frame (64 at most; a backlog goes as a
   few), a style frame before a sample whose style changed.

   That ring waits for the main loop: a sample reaches it, and the viewer's wake, only when the
   loop reads the pens, so a heavy tool (a textured brush whose ink frames take tens of
   milliseconds) held the samples back and let them go in bursts (the owner's readout in Firefox
   Nightly: pen age at paint p50 6.5 ms, p90 64.5 ms, max 203.5 ms; a skipping dot). So where the
   Kindle has the raw digitizer (WacomDigitizer, read without a grab), a pen viewer opens that node
   itself (`raw_open`: never grabbed, each evdev reader has its own queue, so neither the main
   loop's reader nor the pen service misses anything; EVIOCSCLOCKID for the main loop's
   CLOCK_MONOTONIC kernel times; `st_rdev` checked) and sends each frame the moment the node has
   it. The main loop publishes, in the shared frame, what the viewer cannot read itself: the map
   from raw units to screen pixels (`input_raw_pen`: the affine map raw_hover and hover_from_raw
   apply, worked out from three places inside the raw ranges, pressure and tilt ranges, and the
   tilt's turn from calib_tilt; none while the hover orientation is not trusted, the times are not
   CLOCK_MONOTONIC or the pen is the Kindle's), at each tick and as Live view starts
   (`live_set_raw_map`); the capture state after every event and tick and wherever it changes
   (`live_set_capture`: whether quick ink may draw at all, the editor's style for each eraser and
   side button state, and the newest pen event handled); and whether the loop is in UI work
   (`live_set_blocked`, LIVE_CAPTURE_BLOCKED's twin). The viewer decides each frame's ink as
   live_capture does (`live_raw_gate`): a pen-down on the panel, or where nothing draws, is UI,
   and nothing draws after it until the main loop has handled an event at or after it. But the
   viewer runs ahead of the main loop, often by a stroke or more, so a pen-down that comes while
   the loop is in UI work (it is, for a moment, at every event it handles, and for the whole of a
   pen-up's commit) or behind a UI pen-down it has not handled yet cannot be judged when it comes:
   its samples are held (`LIVE_RAW_HOLD`, about half a second) until it can, then go drawn from
   their pen-down or undrawn as UI. Judged as they came, such a stroke lost all its quick ink: the
   page ends a stroke at its first undrawn sample and starts one only at a pen-down. For the same
   reason a stroke once drawing goes on drawing through the loop's UI work, and one broken by a gap
   starts again at the sample after it; the capture state a read finds half written is the one
   before. Without the node, or while the map is withdrawn, the viewer reads the ring as before.

   A sample is known by its identity: the kernel's time of its frame in microseconds (u32,
   wrapping), the same for the raw node's frame the viewer reads and for the main loop's own event
   of it (InEvent.k_us; the simulator's scripted events, which have no kernel time, take the time
   they are captured, kept increasing). The scaled pen that the main loop follows is relayed from
   the raw frames about 0.1 ms after them, well inside a frame's 2 to 5 ms, so its event stamped t
   covers every raw frame stamped t or before and no later one: the tiles' watermark, an identity
   now, retires exactly the quick ink of the raw frames whose ink the tiles hold. Measured headless
   (tests/live_pen_stall.py: a feeder writes a frame every 4 ms to both stand-in nodes while the
   main loop is held 60 ms after each read), frame to browser: the ring p50 61 to 70 ms, p90 113 to
   115 ms, max 128 to 129 ms; the viewer's own reader p50 0.1 ms, p90 0.2 ms, max 0.4 ms, every
   frame delivered.

   The style is what the editor would draw (`editor_live_style`): the tool, its width range on the
   screen at the zoom, its pressure curve and opacity, black or white ink, the stroke eraser (which
   shows only its pen: the samples cannot say which strokes it takes), the page's clip and the
   panel, where no quick ink goes. None while a sheet, the Pages view, the Library or any screen but
   the editor is up, on a read-only page, while adding a link or pinching; none for a pen-down on
   the panel or a menu and the rest of its gesture (a queued UI contact holds the samples after it
   off until the app has handled it); and none for what a pump reads while a key, the power button,
   sleep, waking or a screen's own UI work is handled, as that work may turn the page.

   Two epochs. The screen's moves on at a start and a turn, the only times the tile viewers send a
   hello and the picture starts again from white. The ink epoch moves on whenever ink drawn before
   can no longer join what comes (`live_reset`): a turn, a page turned (`go_to_page`), a screen
   changed (`app_switch`), a pinch begun, sleep, waking, and the pairing card coming or going. The
   pen viewers say so with a hello, and the page clears its quick ink; a page turn costs only the
   tiles that differ, as before fast ink. `app_flush` hands `live_presented` the newest pen event a
   screen has finished handling (`App.live_processed`), with the ink epoch, once every pixel it
   dirtied is in the frame; never the ring's head, which a pump reads ahead during long work. A
   tile viewer reads that watermark before it takes its marks and sends it with its batch end (in
   one of its own when no pixel changed), so every pixel it vouches for is in that batch or an
   earlier one. A batch cut short (the state or the screen's epoch moved on, or a PNG could not be
   made) vouches for nothing, and a PNG with no memory for it puts its marks back for the next wake,
   at the batches' pace, never at once. The page draws quick ink only over a picture of its ink
   epoch (the newest painted batch's), so a page turned shows none over the old page, and drops a
   stroke's quick ink up to the sample a batch names once that batch's tiles are painted, in the
   same animation frame, by identity, never by the millisecond (several samples share one). A gap
   the kernel reported, a ring overrun or a reconnect breaks a stroke, and only a pen-down starts
   one: a browser that joins mid-stroke gets the newest place without its pen-down. The
   history is 2048 segments at most while the tiles fall behind; there is no prediction. A wake
   costs a byte only when the viewer has none on its way (a flag in the shared frame, which it
   clears after draining its pipe), and each kind is woken only for its own news: the tile viewers
   for pixels, the watermark, the cursor, the colors and the light; the pen viewers for samples
   and the ink epoch; both for the state and a turn.

   The quick line is round, its width and opacity following the pressure curve, in the tool's own
   shade (brush.c's look_of: the highlighter's gray, not black) as the picture's mapping shows it
   (inverted, the warm light, transparent paper); a see-through stroke is drawn whole and laid over
   the page once at its opacity, so its overlapping segments never add up to black over the writing
   it highlights. White ink and the precise eraser draw the paper's color (black while inverted;
   on transparent paper they take quick ink away). Nibs, grain, smoothing, tilt, the paper white
   ink brings back and the stroke eraser are the tiles'. Record takes the picture and the pen's
   dot, never the quick ink. Document PiP and the pop-out keep the same quick-ink canvas as the
   main view; video PiP draws the quick ink and the dot into its own frames.

   The page plays the samples out at the pace the Kindle read them (Playout): each shows at its own
   time on the samples' timeline (the identities' microseconds unwrapped) plus an offset, the least
   this computer's clock has run ahead of the samples over the last 4 s (the quickest path from the
   pen), plus a delay: the 90th percentile of how late the last 120 came and one sample's spacing
   more (so the sample after the clock is there to glide to), at most 20 ms, rising at once and
   falling slowly. A pen moving 1 px every 4 ms whose samples come in bursts every 20 ms steps
   from frame to frame at 60 Hz with a spread of 1.86 px drawn as they come, and of 0.00 px played
   out, 20 ms behind (tests/live_check.js). A burst within it draws as a smooth line, the
   segment that the clock falls inside cut where it falls, and the dot glides between the two
   samples on either side of the clock (only between samples of the same state, close in time: no
   glide across a lift or a gap). The latency readout shows the delay (Playout delay), and a
   sample's age counts as it shows. In a hidden tab (no animation frames) a recording and Picture
   in picture take the newest place at once. The dot's color (five presets and the browser's color
   picker) and size (small, the default and smaller than before, medium, the old size, or large)
   are remembered in the browser (localStorage, failures ignored).

   The editor's panel and the clock and battery corner are layers of their own (frame 9: u8 which,
   its rect, its PNG; w 0 when it is not on the screen), shown over the page in the browser by
   default and switched off each by its own switch (Show toolbar, Show clock and battery,
   remembered). The frame holds the page under the panel: app_flush pushes the rect a.screen
   presents, then composes the part of it under the panel again without the panel
   (`editor_live_under_panel`: compose with the panel left out, pushed, the pixels put back) and
   copies the panel into its layer (`live_layer`); the corner, an overlay never in a.screen, is
   copied as status.rs paints it, and taken away when it goes. The tile viewer sends a layer that
   changed after the batch's tiles, whole (Seq: odd while it is written), and every layer again
   after a hello; a turn takes them away until the turned screen's flush tells them. The page keeps
   the tiles on a canvas of their own and draws raw as the page with the layers that show; the quick
   ink keeps off the panel only while the panel shows. The Kindle's own screen is unchanged.

   The page works per animation frame: a batch read only unions its rect into what waits, and the
   next frame composes it once (a burst after a Wi-Fi stall is one compose), shows it, draws the
   changed rect where the picture is and retires the quick ink its batches vouch for. A hidden tab
   has no frames, so its work waits and runs once on return; only a recording composes at once.
   The canvases read back each batch (page, raw, shown) are `willReadFrequently`, so they stay in
   memory and nothing goes to the GPU and back; areaAverage keeps its span tables and buffer
   between calls. A resize lays out once a frame, makes no canvas again while the size holds,
   scales the picture with the browser while the resize lasts and averages it exactly 150 ms
   after. The size engine (Fit, Fit width, or k device pixels to each Kindle pixel, 25% to 200%,
   remembered in localStorage) never makes a canvas larger than the Kindle's screen: above 100%
   the browser enlarges it with `image-rendering: pixelated`.

   Picture in picture takes the best tier the browser has. Document Picture-in-Picture (a secure
   page only) and, without any, a pop-out adopt the live picture alone (its stage) into their
   window and copy only styles; the original tab keeps a placeholder, the controls, the streams and
   the JavaScript, and draws nothing: animation frames, size and pixel ratio follow that window,
   which renders 1:1 at its own device pixels. Closing it returns the same picture. Video PiP
   (Safari, Chromium on plain http; Firefox there through its own button on the video) floats a
   video of a canvas of its own sized to the PiP window's pixels: the picture averaged into it, the
   quick ink and the dot drawn over it, and `captureStream(0)` with `requestFrame` only when
   something changed. The page's view is off meanwhile and frames come from a Worker's timer as
   well as animation frames, since a hidden tab has none; the held eraser's lease renewals use that
   timer too. While the tab is hidden (behind the floating window, or another tab in front) a frame
   is also drawn as soon as a network read asks for it, in a microtask, since Firefox Nightly 161
   slows every timer of a hidden tab, a Worker's too, to once a second (measured in headless
   Nightly: the picture changed once a second and the ink came 0.45 s late at the median, 0.9 s at
   p90; now 1 ms and 2 ms, and the video shows 59 frames a second instead of 1, with
   `tests/live_pip_bench.js`). A tile batch, a new stroke or a change of the pen's state draws at
   once; the pen merely moving on draws once 8 ms at most and waits for its next packet otherwise
   (it keeps sending while it is near). Those frames draw all the ink that has come and the dot at
   the newest sample, not the playout's paced clock, as there are no frames between packets to
   pace. A frame never asks for the next this way, so it cannot spin. With the tab shown, animation
   frames draw video PiP as they draw the page, and the ticker stands in only when none has come
   for 30 ms (it fires after 34): before 2.0.0-beta.2 it added a frame between every two, so PiP
   drew and captured about 120 frames a second to the page's 60. Each frame draws again only what
   changed (where the last frame laid the quick ink and the dot, what of the picture changed since,
   where they go now; all of it after a refit or a new window) and asks the capture for a frame
   only then; out, base and the see-through ink's scratch canvas are in memory
   (willReadFrequently), so a captured frame is never read back from the GPU. The owner measured
   the pen 15 to 23 ms later in a large PiP window than on the page, growing with the window: at
   the Kindle's 1860 x 2480 every frame filled 4.6 million pixels and the capture took them all,
   120 times a second. Headless Firefox draws canvases in software, where that cost 2 ms a frame
   (now under 1, at 60 a second), and there PiP's pen age at paint is within 2 ms of the page's at
   every size; a piecemeal frame matches the same picture drawn whole pixel for pixel
   (`tests/live_pip_bench.js`). No second client, stream or watcher is made in any tier. A
   recording keeps its own size independently.

   Monitor mode (M, or its button) fits the page, fades the controls rail until the pointer moves
   or keyboard focus reaches it, and asks for full screen when chosen; the browser remembers it (storage that
   fails is ignored). Latency (its switch in View settings) shows two ages at the animation frame that draws them,
   p50, p90 and the largest of the latest 1000: the pen's quick line, from its sample's Kindle
   time, and the page image, from the time of the newest sample its painted batch vouches for (the
   Kindle's own handling, rendering, PNGs and the network all in it); with the clocks'
   uncertainty: of eight paired `GET /live/time` requests the one with the smallest round trip is
   taken, half of it the uncertainty, again every 30 s. These are software estimates, not when a
   monitor lights a pixel: a 240 fps video of the pen and the monitor together is the real check.
   The pen stream follows the tile stream: it starts once a tile hello says the Kindle streams,
   retries as the tile stream does (500 ms, doubling to 8 s), and after a 404 (Live view off, or a
   Grimoire without fast ink) or a bye waits for the next tile hello. A page of another protocol
   version than the Kindle's (a tab left open across an update) loads itself again, once a minute
   at most.

   Record makes an MP4 wherever the browser can (the owner: "absolutely an mp4"). MediaRecorder's
   own first (H.264, the first of avc1.42E01E, avc1 and plain video/mp4 that isTypeSupported takes;
   Chrome raises the level itself, 5.0 at 1860x2480, measured in Chrome 156; Safari takes plain
   video/mp4). Firefox's MediaRecorder writes only WebM and Ogg (Nightly 161's XUL has no MP4
   writer), but its WebCodecs VideoEncoder makes H.264 (macOS: VideoToolbox), so there the page
   makes the MP4 itself (CodecRecorder): Baseline H.264 (no B frames, so no reordering table),
   asked of VideoEncoder.isConfigSupported at each hello's size, through filmChoices: the screen's
   own size, then smaller even ones (a hardware encoder may take no side over 2304 or 1920 px; the
   owner's Firefox Nightly 161 took none of the old probe's three Baseline strings at the screen's
   size in the length-prefixed form, and said Record (WebM)), each with the
   length-prefixed form and Annex B (whose parameter sets go to avcC, and whose NAL units become
   length-prefixed samples), the browser's own choice of hardware, then software and hardware
   outright, Constrained Baseline before Baseline at levels 5.1, 5.0, 4.2 and 3.1; an encoder that
   fails once it began is struck off and the probe goes on past it. The button says which it
   makes, Record (MP4) or Record (WebM), plain Record while the probe is out, and its tooltip the
   codec and size; a frame taken from film
   once the code that changed it has run (a microtask, not an animation frame, so it goes on while
   the tab is hidden), at most 15 a second, a key frame when 2 s have passed since the last; at Stop
   the encoder is flushed and mp4Boxes writes ftyp, mdat (the chunks as they came) and moov (stts
   from the frames' own times, 30000 ticks a second, each frame lasting until the next, stss, stsz,
   stco, co64 past 4 GB). Only a frame per change, so a still screen is one long frame. Measured in
   Firefox 158 headless at 1860x2480: Constrained Baseline at level 5.0, every frame decoded by
   ffmpeg and by AVFoundation (what QuickTime plays with). WebM only where neither can (the button
   says "Record (WebM)" and why). Transparent paper records as an MP4 too, its paper drawn in: an
   MP4 has no alpha (Chrome's clear paper came out black without it). film keeps its first size for
   the whole recording, a turned screen fit whole inside it: Chrome's MP4 frames after a size change
   came out corrupt (its WebM changed size mid-file). Viewers end with a bye when Live view stops
   (Stop on the card, 10 minutes without a viewer, Grimoire leaving: `app_shutdown` stops them
   before anything else, so no export child inherits a socket and the headless runner never waits
   for one); on Linux they also have PR_SET_PDEATHSIG, and everywhere they end once their parent
   has gone.
   An icon-only chip says Live view is on: in the top right corner (inside the 50 px margin) on the Library,
   Share and the other screens, and beside the editor's panel on a notebook page (and none while a menu is
   in front of it); it is overlaid at the present like the toast, and the screen guard skips it
   (`display_skip_overlay`, moved with the chip). Auto-sleep waits while a computer watches. The log names no code,
   token or `/live` query; only the simulator logs the code and the viewers' pids, for
   tests/live_check.sh, which pairs as a browser, reads the stream (tests/live_stream.py, which
   decodes the PNGs itself, and the pen frame for the place its script left the pen hovering),
   refuses a third viewer, then reads the pen stream while its script (tests/live_fast.qs, which
   the runner's `hold PATH` holds until the pen stream is read) opens a notebook, draws a stroke and
   taps the panel and a menu: `/live/time` and `/live/pen` need pairing, the pen stream's hello and
   reset, the stroke's samples with pressure and its pen-down, none of quick ink on the panel or a
   menu, each frame in the ink epoch of the hello before it, no tiles on it, and a batch end that
   vouches for the last sample in that epoch. It checks the browser's frame reader on the same bytes
   (tests/live_check.js, with the page's pen, film and format math, the quick ink's model, colors
   and drawing, mp4Boxes read back, CodecRecorder on a stand-in encoder, and, where ffmpeg has
   libx264, a real H.264 stream put in an MP4 that ffmpeg must decode to the stream's own frames at
   their times), runs the page itself on DOM and canvas stand-ins (tests/live_browser_check.js: the
   quick ink painted while PNGs decode and retired by the batch that holds it, the ink epochs in
   either order, the pen stream's states, Monitor mode, how the pen stream retries, and the Warmth
   and Brightness sliders: never moved while held, one draw a frame, brightness composed with
   warmth, inverted colors and transparent paper and remembered), and checks
   that no viewer outlives the app. test_live (rust/tests/test_live.rs) checks the ring, the frames
   and the watermark against the viewer children over socket pairs.
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
level), Delete. A folder's menu: Rename folder, Move folder (the same list, less the folder itself
and the folders inside it; it goes with all it holds, and its notebooks' mirror PDFs follow),
Remove folder (only an empty one). Folders sort by name with accents folded (`cmp_name` in store.c: Latin-1 and
Latin Extended-A letters compare as their base letters, ß as ss, æ as ae, œ as oe, ĳ as ij, þ as
th, combining accents dropped), so Álgebra sorts among the A's; names that fold equal go by their
bytes. A name, folder or folders.ini line longer than 95 bytes is cut before the UTF-8 character
that would cross the limit (`clean_line`).

## Fast navigation and optional KOReader integration (1.1.0-beta.2)

The Pages screen (`pagesview.c`) draws a fixed screenful of numbered tiles, never `load_page` or
`page_render` for a preview. There is no preview cache. Selection is still by page index; a number
pad (keyboard page 2) validates a direct jump before the selected page is read. New page inserts
after the open page or the last selected page, using the notebook's paper. Copy page duplicates
the one selected page after itself, with a new page id and the original bytes. Image-paper copies
share their picture. Both keep the first page in place. A failed notebook-list save rolls the
insertion back and removes the new file. Book-page labels come from `source.ini`; a copy is a note
page and does not get the original's book-page mapping. Since 2.0.0-beta.2 a held page's card and the
bottom row (with pages ticked) also have Move to... and Copy to notebook.... Move to... reuses the number
pad (`PagesView.moving`): the number typed is the page the ticked pages go before, one past the last (or
an empty pad) the end; `store_move_pages` rewrites only `notebook.ini`'s order, and the editor follows the
open page by id. A move is one undo step of the notebook's own, not the page history's (history stays per page,
so the editor's Undo button never undoes a move): `store_move_pages_undo` gives a `PageMoveUndo` (the order
before and after), kept in `PagesView.move_undo` and dropped as the notebook closes. While it applies (the
same notebook, its pages still in the order the move left: an add, delete, copy or another move changes
that; a rename does not) and nothing is ticked, the bottom row's Undo stands in Move to...'s place, and
`store_undo_move_pages` writes the order back through the same `reorder_pages` path (notebook.ini only).
A tile held until its card opens and moved past the slop is dragged (`PageDrag`): the view as drawn is kept,
a hollow outline of the tile (two frames for several pages) and a bar in the gap (`drop_at`: the half of the
tile under the contact says before or after it; past the last tile, the end) are drawn over it inside the
grid's band and refreshed fast, only where they were and are; 700 ms over the top bar or the bottom row
turns the screen (the band alone refreshed). The lift asks `store_move_order` whether the gap changes the
order: then the carried pages are ticked and it is Move to...'s path (one undo step, one Full redraw);
else, and for a lift over the bars or a cancel, one Ui redraw and no undo step. Copy to
notebook... opens `NotebookPicker::start_copy` over the Pages view, without the open notebook; a page tile
there (the copies go before it) or At the end gives `NotebookPick::Place`. `store_copy_pages_to` copies the
files (docs/FORMAT.md, Moving and copying pages), then `nav_copy_pages_across` the names and links, every
target through `nav_target_across`, the one rule for a link crossing notebooks (lasso paste shares it).
The destination's compaction stops first (`bg_cancel_notebook`), as when a notebook opens; its card is
drawn again when the copies come first, its PDF mirror is queued, and so is its compaction when a copy
holds erase strokes. The browser reader similarly
uses one image at a time and a GET page field at the top (`?p=N`), clamped to the notebook's range.

`src/koreader.c` discovers the hosted directory or three common installations and manages the
plugin and optional patches. Different patch contents at a managed filename are preserved.
Terminess fonts are copied from the binary's embedded font data. No KOReader settings Lua is
executed. The plugin's custom wallpaper export is a bounded PNG committed by write-then-rename,
read by `store_load_koreader_sleep`; it is cached only when used and freed at shutdown.

The handover wrapper retains KOReader's frameworkStarted LIPC service across Device:exit when
its native framework was stopped. Input/screen teardown still occurs before Grimoire. The handle
closes after Grimoire returns, before koreader.sh starts a new reader process. App exit statuses
propagate to the plugin so failed launches keep the diagnostic marker.

## Page previews: the Pages view and the page pickers (2.0.0-beta.2)

The Pages view's tiles and the destination picker's (Copy to notebook..., Link to...'s other notebooks)
show the small previews of `.page-previews/` (docs/FORMAT.md), which the UI only decodes. A screenful's
missing previews are asked for before its tiles are drawn (`bg_request_page_previews_but`), as one batch
for the background job (bgjob.rs, `store_build_page_previews`), which draws them in a forked child, so
the UI never draws a page for a tile.

- **Each tile as its preview lands.** The job makes the batch's missing previews cheapest first (pages
  with a current kept drawing, then by the size of their files: a dense page last) and tells an open view
  of each one as it is written (the `updated` token). While a batch of the notebook shown is being made the
  editor looks every 100 ms (`PREVIEWS_POLL_MS`, else every 500 ms) and draws only the tiles that show
  something else now (`PreviewCache::changed`, against what each tile was drawn showing), in one GL16 of
  them: never the whole screen, never a flash, never twice for the same preview. Previews that land within
  one look come in one refresh.
- **A tile still waiting shows an hourglass** (`ui_tile_loading`, the md-timer_sand glyph, black, a third of
  the tile), drawn once with the tile and left until the preview is drawn in its place. A tile whose batch
  ended without its preview (a page that does not read, one from a newer Grimoire) shows its number alone,
  as every tile did before; it is asked for again when the view opens again.
- **A batch asked for while another runs** (the view turned to another screen while a dense page is drawn)
  is queued, put before the other queued previews of that notebook (`bg_prefer`), and the batches of screens
  turned away from that still wait in the queue are dropped (asked for again when they show). With three
  cores or more (`std::thread::available_parallelism`), a batch a view waits on starts at once in a second
  child beside the first (bgjob.rs `extra_slot`: one such child at most, previews only; each child holds a
  page canvas and a page at a time).
- **Never two children on the same pages** (bgjob.rs `next_job`): a previews batch does not start while the
  other child runs one of the same notebook over any of the same page indexes (the same batch, or one that
  meets it, as the 24 tiles of Choose from the pages... meet the warm-up's 9); it waits for that one, then
  finds its pages made. The same batch queued already (the idle warm-up's, a view's from before) is the one a view waits on:
  it reads the notebook's page list as it starts. One already running that the view did not ask for read the
  list before, and the pages may have moved since (a page deleted, pages added), so the view queues its own,
  which runs after it and makes only what it did not. A second child that cannot be forked (the memory short)
  puts its batch back at the head of the queue, for the first child, and none is tried again until the first
  is free. When the last batch of the notebook shown is gone, ended or dropped unstarted, the tiles still
  waiting are drawn again (their numbers), so no hourglass stays.
- **The open page's tile** is made from its canvas as the view opens (it is drawn already), kept while the
  view is open (`retain_screen`), and left out of the job's batch, so a dense open page is never drawn twice.
  It is left out as well when its preview is current, as the idle warm-up leaves it out: the two then ask for
  the same batch. Its preview on disk is written by the idle save, or by the job once another page is open.
- **Dense pages.** A page with a current kept drawing (`p<id>.full`) gets its preview from that drawing,
  read back, not drawn; a page the job drew in `store_keep_render_ms()` (1 s) or more of its own processor
  time keeps its drawing too (`store_save_page_render_at`, only while the page file is still the one it
  read), so the page then opens quickly in the editor as well. A stale kept drawing is never used, nor counted
  as a quick page when the batch is sorted (only one whose whole stamp is current: the page file, the size and
  the renderer's fingerprint). A page drawn with its picture paper white (its picture missing or unreadable in
  the child) keeps no drawing and gets no preview, as in the editor: its tile shows its number, and the next
  view that opens asks for it again.
- **Opening a dense page** (a page turn, a jump, a notebook opening on it, the page left after a deletion)
  that has no current kept drawing and draws slowly (`store_page_dense`: a page file of 512 KB or more, or a
  page that kept a drawing before) says "Loading page..." (`render_page_open`): the toast is put on the
  panel (app_flush) before the drawing starts, and taken down once the page is drawn. Never for a page that
  draws quickly, nor while a pen held on an action holds the panel.
- **The log.** One line per batch, from the job: `previews: nb=ID pages=N cached=N kept=N drawn=N keep=N
  ms=N first=N slow=pID:N cores=N` (pages in the batch, previews already made, kept drawings read back,
  pages drawn, drawings kept, the batch's time, the first preview's time, the slowest page, the processor's
  cores), beside bgjob's `bg: page previews of ID (pid N)` and `done` lines (`beside pid N` for the second
  child).

Drawing previews straight at their size (the page's strokes scaled to 300 x 400) was measured and not
adopted: about 8 times faster on a dense page, but not the same picture (thin strokes and hatching come out
darker or lighter: a mean difference of 13 to 81 gray levels on the owner's dense pages, against the shrink
of the full drawing; drawn at twice the size and then shrunk, still 3 to 9 levels and up to 6% of the pixels
more than 32 levels apart).

## Touch gestures, the Links bar, Find and the link areas (2.0.0-beta.5, the Rust build)

**Touch.** Fingers reach the editor through `touchgest.rs`, a pure recognizer that input_evdev.rs
(the Kindle) and sim_sdl.rs (the runner) each feed contact frames and the pen's state, with kernel
time (`wrapping_sub`, never `now_ms()` inside a batch). It keeps the one-finger tap/link-hold
stream and recognizes ZOOM, PAN, SWIPE and TAP. Phase 3 selects provisional `TG_TUNE`:
a 16 mm swipe within 1200 ms and a tap within 400 ms, with the other gates in
[TOUCH-STUDY.md](index-hook/TOUCH-STUDY.md). `TG_TUNE_V1` retains the older behavior for replay.
Three-finger patterns require at least three reported slots; two-slot panels suppress them.
Pen contact wins; nearby hover, resting palms and held contacts suppress actions. A fresh
hover far from all fingers can permit a swipe, and pinch can start immediately after writing.
One finger is held as the writing hand only on the notebook's ink (the page with nothing over it,
off the panel and the link strip), by the same near and far rules; on the Library, a sheet, a
keyboard or the panel a finger always counts. On a keyboard's keys a second finger landing before
the first lifts is the next key (roll-over), never a two-finger session.
A swipe refused only for the pen says so once in ten minutes. The editor tells
touchgest what its screen allows (`touch_context`: whether the page pans, zooms, may start a gesture)
after every event, so a sheet, the bar or a keyboard makes a drag a swipe, which the mirror rule closes
the menu with. What a gesture does is `gesturemap.rs`'s (the built-ins, then `gestures.ini`), through
the same `editor_shortcut_at` a key uses; a page turned left by a gesture on the last page adds a page as the next-page key does, unless `gesture_new_page` is off (then it stays on the last page, with no toast). The corner's
clock tap (status.rs) decides at the lift and every finger tap that phase adds checks it first.
`grimoire.log` has one line per touch session and the owner's opt-in `touch-trace.txt` has every
contact frame (docs/FORMAT.md, docs/KINDLE.md).

**Sideways.** With `landscape_fit` (on by default) a page turned sideways rests whole, at the height of
the screen (`view_rest`): nothing scrolls, so two-finger swipes work anywhere, and a pinch zooms in from
there. Off, the width fills the screen as upright and a vertical two-finger drag pans.

**The Links bar** (linkbar.rs, hosted by the editor as dispmenu.rs is) is a card along the bottom with one
chip per place the open page's links go (the Index, a named page, a numbered page or a page that is
gone), in reading order and counted when two links share a page, with no number (the page's link
areas have none); a tool row has Name page, Go to page (the Find sheet) and Areas. It opens and closes on one Ui refresh of its card (no flash). Tapping a chip
peeks: the page above the bar is that page, drawn from a small canvas cache (the origin's own canvas
is never touched, so nothing autosaves, undoes or follows a link on the peeked page), and the view
is never saved. Staying (a finger tap on the page, a pen down outside, the chip again) commits it: the
origin goes on the back list, the cached canvas is swapped in (no second read or render) and one Full
refresh clears the peeks' ghosting. Cancel (X, two fingers down, Close menu, the Links button) puts
the page back as it was left, zoom and all. The bar and Find draw over the panel, and a press on
either is a DU of its own (`app_press_feedback`).

**Find** (findsheet.rs) is the Pages view's number pad done again for the whole notebook: a split
keyboard host puts the rows card above the keyboard, a number offers "Go to page N", letters search the
Index's names (accents and capitals ignored; names that begin with them first, then a word, then
anywhere inside). A result goes through `go_to_page` and one Full.

**The eye** (`link_areas`) draws a dotted box (canvas_dash_rect) and a numbered tag over each link's box
(from 2.0.0-beta.6 its area, or its ink for a link that has none) at compose time. It is never drawn into `page_canvas`, so it is never in a thumbnail, a PDF, a share
page or the sleep screen. The tags carry the bar's entry numbers, from the same grouping function.

**Refresh choices.** DU is 188 ms (294 cold), GL16 271 ms and GC16 341 ms. Screens only `app_invalidate`
and `app_flush` presents once per loop pass with the strongest mode over one union; nothing here adds a
display call or a wait.

| Surface | Waveform | Rect |
|---|---|---|
| Two-finger gesture while moving | none: the action's own refresh is the feedback | |
| Pan or zoom | Fast at most every 180 ms of the event clock, then Ui at the end if it moved | whole screen |
| Panel press cancelled by a second finger | Fast | the button |
| A bar chip or button press | DU at once (`app_press_feedback`), outside the accumulator | the chip |
| Bar open or close | Ui | the card |
| Peek | Ui, one present | the page area united with the card |
| Commit | Full (resets the every-30th counter) | whole screen |
| Cancel | Ui, or Full after three or more peeks | whole screen |
| Back chip, pager | Ui | the page area, the chip grid |
| Eye toggle | Ui of the button, a flush, then Ui of the boxes' union | the button, then the union |
| Find open from the bar or a key | Ui | rows card and keyboard |
| Find open from the Pages view | Full (the view closes in the same refresh) | whole screen |
| Find typing | Fast field, rows and key, each flushed; one Ui of keyboard and rows 900 ms after a pause | |
| Find result | Full (`go_to_page`) | whole screen |
| A gesture's page turn past either end of the notebook | none: the page stays, with no toast (2.0.0-beta.2) | |
| `landscape_fit` toggled sideways | Full | whole screen |

No GL16 and no toast ever run while the pen is down for ink: a pen down during a peek commits it at the
PenDown, and the stroke is swallowed (`draw_begin` never runs, so pen mode is never on for it). The
every-30th-Ui promotion to Full stays.


### Clock corner after RC1 feedback

The status strip is retired: `status_reserved` always returns zero and the corner overlays the
unmodified page viewport. Defaults are on, small, top left. Loading legacy Bar settings migrates to
those defaults; legacy Off stays off. Display has no title-row mode button, and no Status display
assignment is advertised. `status-mode` remains a compatibility alias for the corner toggle.

## Links between notebooks: backlinks, anchors, broken links (2.0.0-beta.2)

What links INTO a notebook is spread over every other notebook's `links.ini`, so `backlinks.rs` keeps
it turned around in `root/backlinks.ini` (docs/FORMAT.md, Backlinks cache): a cache, rebuilt by stamps.
A save of `links.ini` (nav_save_links) updates its notebook in memory at once; opening a notebook asks
for a check of every notebook, which the editor's idle ticks do a few stamps at a time with at most one
`links.ini` read a tick (512 KiB at most), so the page is never held up; the file itself is read on the first
idle tick, never by a draw (a query sees what is in memory). Other notebooks' page names, for the Linked
from rows and the anchors, come from `backlinks_book`: their `notebook.ini` and `nav.ini` only (never their
`links.ini`), kept between builds for the 16 most recently used and read again only when one of the two
files changed. The Links bar lists Linked from
after the page's own entries (its own notebook's sources from the open Nav, always current, then the
cache's); a bar that is up is built again only when its rows changed. A Linked from row has no hold: a
long press on it is its tap. The Graph boxes other notebooks'
pages beside the plot (anchors), never as layout nodes, so the map keeps its shape and `graph.ini` its
`gen`. Each line is solid where that page links out to the box and dashed where it only has links in from
it, and an arrow at the box's start says which way its links go (two arrows: both ways); a page linked only from elsewhere stays hidden there (its box names it). A target whose page or
notebook was deleted (`nav_target_state`: a notebook folder that is not there is gone; one whose
`notebook.ini` does not read is only unavailable) is broken: dashed wherever it shows, and a tap opens
its link's card, which leads with what is gone. Following a link into another notebook pushes the
origin's page, view (zoom and offset, for its screen size) and back list; Back restores all three.

## Links without ink and the long-press card (2.0.0-beta.6, Index phase 2)

A link is (page, seed, target, area) in `links.ini` (docs/FORMAT.md). The area is screen-only: the eye
draws it at compose time and nothing puts it in `page_canvas`, so it is never in a thumbnail, a PDF, a
share page or the sleep screen. A link's box is its area, or for a beta.5 link with no area yet the box of
its ink; the smallest box under a finger wins. beta.5 links migrate at page load and on idle ticks (one
page per tick, read through `store_load_page_pumped`), and their ink stays as ordinary ink. Making,
deleting, retargeting and redrawing a link are undo steps (`ActKind::Link`, the editor applying the
records); a step whose save fails is dropped. Copy page copies the page's links.

**The strip.** Add a link (the bar's New link, the add-link action) shows a strip along the bottom
("Draw around what links.", a Cancel at its right). It is not a sheet: the page still pans, zooms,
turns and follows links while it is up (a page change cancels it silently). The next pen contact on the
page, whatever the tool, is the lasso.

**The lasso** (`Owner::Lasso`) is dashes drawn straight into the screen with `canvas_dash_poly`, one DU
per input batch, never pen mode or the live ink style: it is not ink, and touches neither the page, the
history nor the PDF. At the lift its box (page px) is the area; a box under 7 mm both ways says "Too
small" and the strip stays. A dotted rectangle (`canvas_dash_rect`) marks the area until the pick.

**Link to...** is a second `LinkBar` in pick mode (`EditorState.lt`): New page..., The Index, up to four
recent targets (this session's picks, then the back list), then the Index's entries, six chips a screen
upright, with a pager; a tool row of Search, Choose from the pages... (a new link only) and Cancel. It
docks at the top, below the clock corner, when the area's centre is in the lower 45% of the screen, and
at the bottom otherwise. New page... names a page added at the end and links to it (the open page
stays). Search is Find in PICK mode: its digits row reads "Link to page N", a row links instead of going
there, and its keyboard docks with Link to... (`keyboard_set_dock`). Choose from the pages... is the
Pages view picking, with no buttons beside its tabs and no Go to page.... The first link ever made toasts
a tip about tap and hold once (`hints` bit 1) instead of "Linked to ...".

**The card** (linkcard.rs). A finger held 600 ms on a link draws a heavy dotted border around its box
and opens a card beside it: Rename in the Index (Add page N to the Index... for an unnamed page),
Change where it goes..., Redraw the area and Delete link, Delete last; a read-only page's card has no
rows. The card goes right of the border, else left, below, above, never over the border, the panel
(when it can) or the clock corner. A Links bar row held 600 ms opens the same card above the bar for
every link of that row on the page: a ×N row has Rename, Change where they go... and Delete N links,
each one undo step for all N. Rename's keyboard docks away from the border; Rename keeps the card open,
every other row closes it. A tap outside closes the card and does nothing else. The Pages view's old
hold path (its picker, Remove link, The Index) is gone. Deleting a link never deletes ink.

**Refresh choices** (no new display call, no new wait):

| Surface | Waveform | Rect |
|---|---|---|
| Strip opens or closes | Ui | the strip |
| Lasso while drawing | Fast per input batch | each segment's box |
| Lasso too small | Ui at the lift, then the toast | the lasso's rect |
| Lasso lift | Ui, its own flush, then Link to...'s Ui | lasso, strip and rectangle; then Link to... |
| Link to... opens or closes, its pager | Ui | its card, the chip grid |
| A chip, row or button press | DU at once (`app_press_feedback`) | the chip or row |
| Find PICK typing | as Find: Fast field, rows and key, each flushed; one Ui after a pause | |
| Choose from the pages... and back | Full | whole screen |
| Pick done | Ui | the rectangle's band and Link to... (and the bar coming back) |
| Card opens from the page | four Fast strips of the border, each flushed on its own, then Ui | the strips, then the card |
| Card opens from a peeking bar | Ui of the page area with its borders, its own flush, then Ui | the page area, then card and chip |
| Card opens from the bar, not peeking | four Fast strips per box (at most three boxes), else one Ui of their bounds; then Ui | |
| Card closes | Ui | card and strips |
| Link undo or redo, Areas on | Ui | the records' boxes, before and after |
| Link undo or redo, Areas off | none: no toast (2.0.0-beta.2); one left up goes | its rect |
| Migration | nothing | |

The border's strips hold only black and white pixels and never meet, so DU shows them exactly; one union
of the four would be the whole box, and DU would posterize the gray ink inside it. No GL16 and no toast
while the pen is down: the lasso's GL16 comes at the lift.

The Index map is an editor-hosted full-screen menu. Its model reads notebook metadata and links,
without loading page files. The current page has two rings, selection a third; labels avoid both
other labels and other dots. Select/deselect refreshes the labels and incident edges as well as
the node and card, since a card-only update would leave stale bold edges and label knockouts.
Pan/pinch previews are throttled to180ms and settle on release. Layouts over300force nodes run
in a background child; a finished result is swapped in after500ms of idle time, fitted unless the
user has moved the view (then the dot under the plot's centre, or the selected one, stays put).

Only linked pages are in the map's model: every other page is hidden, takes no room in the
layout and no part in graph.ini's gen, so naming it or adding plain pages moves no dot. graph.ini
keeps the layout. On opening, kept dots stay put, a few new pages go beside their neighbours, and
a new link between far pages moves only its two ends and their neighbours (graph_relax_stressed);
a map that changed a lot (over 10% new pages, or too many stretched links) is laid out again from
where its dots were (graph_layout_warm), so it keeps its shape. The relaxation stops once the
map's shape settles, usually well before its 300 steps. The view button reads View: All (the whole
linked map) or View: Adjacent (the rings around a page), the view it shows now, and a press always
switches to the other one, fitted (2.0.0-beta.2: there is no "Fit:" label any more; the fit-page key
fits again). Only All and Adjacent are saved (graph.ini still writes
mode=around for Adjacent).

Nested Library folders are logical slash-separated paths stored in notebook metadata and
folders.ini. Notebook IDs and physical directories stay fixed. A subtree rename validates all
new paths and collisions before writing, retaining original metadata for rollback on write failure.

## Settings > Gestures and page finger shapes (Index phase 3)

The Settings bottom row has five equal cells: KOReader, Controller, Gestures, Updates, Done.
Controller, Gestures and Updates words have a 248 px budget at 34 px caps on the upright panel,
in English and Spanish, in both themes. Coordinates remain pending main-scenario verification.
Buttons and Gestures share the seven registry groups and row descriptors from `actions.rs`.
Source masks expose every action to PEN. Hold eraser is pen-only in Gestures (hold/drag slots),
and pan/zoom are PEN-only Pages actions 82/83 (drag slots). Keys/pads cannot learn their own
Enable shortcuts or Bluetooth-controller switches; calibration stays outside the registry.

The Gestures sheet has a master on/off row, an ideas line, seven group rows, Reset and Done.
Groups filter for TOUCH, SHAPE or PEN, show the first available binding in that order and a
96 px medoid thumbnail for a recorded shape. `+N` counts other action/source bindings once
per source, including both members of a pair. Unsupported three-finger bindings on a two-slot
panel and unsupported pen buttons do not appear. Paired rows learn one member at a time.
A hold clears touch, shape and pen sources for the row (both members of a pair); Reset restores built-ins and confirms first when shapes exist. It also
recovers from an unreadable file. Learn/clear use copy-save-swap: failed persistence leaves the
live map and record card intact.

The record card keeps three sample slots, Use, Try again and Cancel. First input selects a
pattern, one-finger swipe, shape or pen-button gesture. Paired rows continue to the second member after saving the first. Patterns can be accepted with Use after a valid attempt;
shapes require two consistent samples, with an optional third. Conflict and accidental-slip
checks run before saving, with progress/refusal text in the line's own rectangle. Moving a
binding names the losing action; replacing a carried newer shape requires an explicit second
Use after the warning. Try again resets the attempt, Cancel returns without saving, and no
attempt within 10 s closes the idle card. Pen taps on either surface are UI input, never ink.

On a notebook page, capture arms only in the eligible idle view with gestures enabled and at
least a shape or one-finger binding. A stationary clock-corner tap stays Pending and belongs to
the clock; deliberate motion can become a shape. Pen arrival, another contact, sleep or a
screen/layout change aborts capture. The dotted, outlined black/white trail is drawn only on
the screen canvas; it never enters the page, undo stack, exported PNG/PDF or autosave. The page
backing restores the trail, including over the Links bar. Refusals fade; a first unknown shape
can show the one-time tip (`hints` bit 7). Match dispatch uses the common action path.

| Surface/event | Refresh | Contract |
| --- | --- | --- |
| Two-finger swipe/tap | Action's own refresh | Nothing while moving |
| Pan/zoom, late zoom, re-grab | Fast at most every 180 ms of event time; Ui at PinchEnd | Re-grab delays the end by at most `grace_ms` |
| Finger-shape trail | Fast, own flush, at most 25 Hz | Never pen mode/live ink; do not merge a trail frame into Ui |
| Match | Ui of trail rectangle, with action refresh | One GL16 per shape, counts toward every-30th Full |
| Reject | Fast thinning, then Ui after 250 ms | No Full flash |
| Abort by pen | Fast restore, then writing settle | Never GL16 while pen is down |
| Record reason/progress | Ui of that line only | No whole-card redraw for a changed reason |
| Gestures open/group/close | Ui, as Buttons | Existing sheet behavior |
| Invert/theme/language | Full | Deliberate screen changes |
| Other actions | Existing action/kind refresh | No device read or new wait in input dispatch |

`TG_TUNE` remains provisional without the owner trace. Synthetic replay exercises transitions
and persistence; it does not establish real-hand success rates, false-fire rates or e-ink
appearance. The pending study and the shape freeze gate are in [TOUCH-STUDY.md](index-hook/TOUCH-STUDY.md).

A refusal without a trail or previously recognized pattern uses `draw_record_line` (line-only
Ui). A candidate/sample change or removal of a trail redraws the card's dynamic area to restore
its pixels and button hit regions. Neither path demonstrates a measured device refresh.


## Pen-button recording and dispatch (Index phase 6)

`penbutton.rs` is the pure, allocation-free recognizer shared by editor and Library. Each owns
its own `PenButtons`. `App.pb_config` derives waits from live double/triple bindings through
`app_gestures_sync` after load/save/reset; this recomputation performs no I/O. Event timestamps
use wrapping u32 milliseconds. A click is at most 450 ms with no nib contact, multi-click gaps
are at most 350 ms from release to next press, and the result resolves 350 ms after final release.
A single click fires immediately only when neither double nor triple is bound. Hovering holds
begin once at 1200 ms. The recorder always waits for a possible multi-click, even when none is currently bound; runtime
waits still depend on bindings. Clear page defaults only to hold, with the duplicate RC1
triple/hold pair migrated to hold. Both owners tick only after the input queue is drained, so a queued release
cannot be mistaken for a hold. Proximity loss cancels a pending sequence and ends capture.

While the card is active, PenButton, proximity, contact and lift feed only its own recognizer.
Fire/HoldBegin/DragBegin select a pen key; endings only end recognition. No editor action fires
and no stroke enters the page. Drag recognition updates state at contact but defers its
card/line Ui (GL16) display until lift or proximity loss. A nib tap starting on Use/Try again/Cancel uses normal Library
pointer handling; a button drag cannot activate them. All ten Button/Button2 labels are translated
and wrap at 64 px in the existing pattern area. The new card body mentions the pen; hold-eraser
and pan/zoom refusals fit the same bilingual two-line status area. Save still uses
`gesturemap_set` on a clone, atomic persistence, then map swap and timing sync. Existing shape
first-Use checks, reassignment confirmation and newer-version replacement remain intact.

In the editor, a bound pen drag owns `Owner::PenDrag` and raises capture-blocked, preventing
page or Live view ink. Unbound drag uses Settings' side-button ink. Pan/zoom use Fast refreshes
at most every 180 ms and Ui at the end; actions use their own refresh. Hold eraser starts at
press for either its hold or drag binding and ends at release, coexisting with held keys.
No GL16 or toast is submitted while the nib is down; deferred feedback appears after lift.
Hardware BTN_STYLUS2 capability gates Button2; Eraser slot lines are always carried, never live.

Map layout results carry their notebook ID. A closed or different map ignores the completion;
a matching open map adopts it only when idle. Failed map opens discard their partial state.
Seed positions stay in memory, so neither closing a map nor changing its view can publish seeds
as a completed cache. A short `graph.ini.lock` transaction serializes preference changes with
the child's final read/merge/write; layout computation happens before that lock is acquired.
Reopening a map retains its running or queued layout job, so the completed layout is still adopted
and an older cached view cannot overwrite it on close.

Clear page saves removal of the current page's links before touching its ink, then records one
`ActKind::Clear` carrying the removed strokes and `LinkEdit`. A link-only page still gets this
step. Undo/Redo applies both parts; a failed link save reverses the tentative ink operation and
keeps the Clear step for retry. Initial Clear save failure leaves ink and history unchanged.

## Lasso select and scoped erasing (2.0.0-beta.2)

The binding design is docs/beta2/LASSO-DESIGN.md; this is where it lives in the code.

- `rust/src/lasso.rs` (pure): the lasso as a crossing table with non-zero winding; enclosure judged by
  each ink stroke's visible spine (samples no later erase stroke in scope covers with its hard core): half
  of what shows inside, or the middle of what shows for strokes under 1.5 mm; links that come along; the
  erase strokes that go with selected ink (after it, in scope, meeting its tiles); the group, with each
  distinct floor of those erase strokes encoded as a marker; seeds; where a paste goes; `lasso_place`,
  which appends a group at the end of a page drawing it onto the page canvas as it goes and decides each
  floor plain (its snapshot is the paper under its strokes' tiles: no marker, exact in older versions) or
  scoped (a free layer).
- `rust/src/page.rs` `render_from`: the renderer's floors. A page with no marker takes the old path
  unchanged; at a marker the canvas over the union of its scoped strokes' boxes is copied, and those
  strokes restore toward it. `page_render_from` draws appended strokes onto a canvas of the page before
  them (a paste or a move draws only the group).
- `rust/src/clipboard.rs`: `root/clipboard.qpg` (FORMAT.md, The clipboard).
- `rust/src/history.rs` `ActKind::Lasso`: the ink taken off plus an Add of the strokes appended, with
  the carried links' records, one step; undo and redo swap the step's canvas patch (instant on dense
  pages). `undo_redo_link` swaps the patch for lasso steps with links, and rolls the ink back if
  links.ini cannot be saved.
- `rust/src/editor/select.rs`: Select mode (`SelState` Off, Armed, Lasso, Selected, Dragging), its bar,
  the gestures (Move, Copy, Cut, Delete, Paste) and the scripts' probes (`editor.select`, `selected`,
  `sel_links`, `sel_box`, `clipboard`, `layered`, `page_links`, `lasso_ms`; `tap panel select.<button>`,
  `tap panel picker.select`). Moved ink goes to the end of the stroke order, on top of what is there.
  While Select mode is on the panel's tool button shows the lasso, selected (no pen is), and the early
  Live ink hook draws nothing (`editor_pen_not_ink`). `sel_hold` is `select` held on a pen button's hold
  or drag (`pb_out`: the drag's contact is not claimed, so it reaches `sel_down` as the lasso);
  `sel_tick` and `sel_finger_hold` paste on a long press (PASTE_HOLD_MS, the link card's hold). Cut
  copies, then deletes; a delete refused after the copy puts the clipboard back (`clipboard_snapshot`,
  `clipboard_restore`). A lasso step's dirty rect leaves out floor markers (history.rs `bbox_of`), so its
  undo refreshes only the group's place.
- `rust/src/lasso_live.rs` (pure): the live selection preview (docs/beta2/PIXEL-LASSO.md 2). Each sample a
  stroke is judged by (`judged_samples`, shared with the lift; lengths in fixed point, 1/64 px) keeps its
  winding as the integer sum of the lasso's edges' contributions, computed with the one crossing expression
  `Lasso::new` uses (`edge_rows`, `edge_cross`). Folding points pn+1..pm subtracts the old closing edge's
  contributions and adds the new stretch's and closing edge's: only samples between the first and last
  crossing of that small polygon in their row can change, found through a 64 px grid of the ink's boxes
  and 32-sample chunks. A full lasso (every other point dropped) and one taller than the table's 8192 rows
  answer from a fresh crossing table. The lift takes the preview's answer (`lasso_select_from`) when the
  page is unchanged (`holds`), and debug builds assert it equals `lasso_enclosed`. select.rs folds a batch
  PREVIEW_MS (120 ms of input time) after the first point waiting, or at once when the pen pauses 50 ms
  (`preview_tick`); the strokes whose halo flipped are drawn again in at most 6 merged fast refreshes
  (DU wear counted), and the lift's Ui redraw covers every one. The halo (`sel_draw_halo`, in compose
  under the panel) is a dither band 3 ui px wider than the ink each side with the page's pixels put back
  over it by a min, so ink stays on top; it stays on the selection. Probes: `editor.halo`,
  `preview_batches`, `preview_refreshes`, `preview_reused`, `preview_rect`, `lift_rect`,
  `lift_covers_preview`.
- Exact area (docs/beta2/PIXEL-LASSO.md 1): `stroke_dabs` runs a stroke's brush dry with
  `BrushState.dab_log` (brush.rs: the dry branch pushes each dab's sample, radius and calligraphy nib) and
  `stroke_cut` splits it into runs of dabs on one side of the lasso: a later piece starts at its first
  dab's own sample (its spacing from there walks the stroke's line), an earlier one ends half a spacing
  past its last dab, new points stored as the file stores them (`sample_as_stored`). A calligraphy piece
  starts with a point leaning so its nib starts at the stroke's smoothed nib (`brush_tilt_for_nib`) and a
  second point at the same place with the pen's tilt; one whose tilt widens or lightens it is not cut
  (`stroke_cuttable`). `lasso_cut_plan` takes ink whole when its box is inside (`Lasso::rect_inside`) or
  every dab is, cuts what crosses, judges beta.5 linked ink and short strokes (under `LASSO_SHORT_MM` of
  what shows, as the preview's halo shows them) by the whole-strokes rule, and areas on the cut view. select.rs keeps the plan (crossing strokes by identity, their pieces) in `SelMode.plan` and the
  lifted lasso in `kept` (the mode button, `SelBtn::Mode`, judges it again); Move, Cut and Delete splice it
  in inside their step (`plan_splice`, the outside pieces at the original's place, so floors and erase
  strokes keep their order), and the step is `history_push_lasso_at` (the originals and the whole ink
  removed, the outside pieces and the placed group added, not contiguous); the old place is drawn again in
  the tiles of the ink taken and of the outside pieces. Copy splices in and out at once (`plan_unsplice`).
  The mode is a static for this run (`EXACT`); Shift or Alt held as a lasso begins (`hidkeys_mods_held`,
  which input_evdev.rs keeps as every attached keyboard's modifiers together, after each of their key
  events, resyncs and detaches) flips it for that lasso; `select-exact` (85) enters
  Select mode in it or toggles it. The live preview in exact mode (`LassoLive::set_exact`, `halo_list`)
  shows a crossing stroke's inside runs of judged samples and a solid tick halfway between an inside and an
  outside sample. Probes: `editor.sel_exact`, `sel_cut`, `page_sig_keep`, `page_sig_same`;
  `tap panel select.mode`.
