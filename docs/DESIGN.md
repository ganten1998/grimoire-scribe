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
| share.c | the Share screen: the QR code, the addresses, its own layout check; the server answers its routes only while it is open. Also Live view's pairing card | app, httpd, qr |
| live.h/.c, web/live.* | Live view: the shared copy of the screen, its 64 px tile marks, the viewer children that stream the tiles as PNGs; the browser's viewer | canvas |
| pair.h/.c | one-use pairing codes and QR tokens, and the browser sessions they give (memory only) | - |
| library.h/.c | library screen, new-notebook sheet (paper picker), rename/delete, Settings (Language among them) | app, store, ui, keyboard |
| editor.h/.c | the notebook editor, live ink, eraser, floating panel, page nav, autosave, read-only pages | app, page, history, store |
| pensettings.h/.c | per-tool sensitivity screen: sliders, curve graph, live readout, test pad | app, brush, ui |
| calibrate.h/.c, calib_fit.h/.c | pen + touch calibration (tap crosshairs); the fit, and scaling a calibration to another panel (`calib_to_panel`) | app, input |
| display.h + display_fbink.c | Kindle e-ink output through FBInk, the screen guard's readback, refresh health, live ink and the color panel's wait | FBInk, fbcheck |
| input.h + input_evdev.c | Kindle pen/touch/power evdev input, EVIOCGRAB, calibration mapping, the power service watcher | linux, power |
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
- Refresh modes: `REFRESH_FAST` (DU, black/white, for live ink, ~120 ms), `REFRESH_UI`
  (GL16 partial, no flash, for UI and to settle the grays of a finished stroke),
  `REFRESH_FULL` (GC16 flashing, page turns / screen changes / every 30 UI refreshes).
  Accumulating different modes keeps the strongest (FULL > UI > FAST).
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
  `font_has_glyph(face, cp)` tells whether a face has its own glyph, without the Terminess
  fallback.
- The keyboard's text is UTF-8 and its keys carry code points (`KB_KEY_*` below 32 for the named
  keys). There are two pages, letters and accents (`Keyboard.page`); `keyboard_upper()` is the
  shift case map (Latin-1 and Latin Extended-A), `keyboard_key_rect()` locates a key on the page
  showing (for tests and scripts), and `keyboard_open` keeps valid UTF-8 and drops control
  characters and broken bytes. A key types a whole character; backspace removes one. In Spanish
  the letters page's middle row ends with ñ after l (`k_row_es`), as Spanish keyboards have it, and
  is then as wide as the row above; the accents page keeps ñ in every language.

## Languages

- English is in the code, and every string a person sees goes through i18n.h: `_()` looks one up
  now, `N_()` marks one in a static table (looked up where it is drawn), `C_(ctx, s)` and
  `NC_(ctx, s)` add a context that tells two meanings apart ("paper", "tool", "month", "date",
  "key"...), and `NG_(one, many, n)` gives the plural form for n. Each language is
  `po/<lang>.po`, compiled by msgfmt to `po/<lang>.mo`, which is committed (building needs no
  gettext) and embedded like the fonts: `build/gen/l10n.c` holds every po/*.mo, named by its file
  (po/es.mo is "es"). The list of languages (code, name in the language itself, number of plural
  forms, plural rule) is `k_langs` in src/i18n.c, English first.
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
  s_passcode_once, t_passcode_left_off and t_toast_over_sheets, `GRIMOIRE_BG_DELAY_MS` in
  v_leave_from_editor, `GRIMOIRE_HOST` in y_koreader_hosted and y_koreader_not_hosted,
  `GRIMOIRE_LANG` in y_language and z_language_kept, `GRIMOIRE_SIM_SIZE` in y_pagesize_1_ksc,
  `GRIMOIRE_CHECK_FIT` and `GRIMOIRE_SIM_ADDRS` in y_all_screens), `# settings: key=value` lines
  go into its settings.ini, and a `# pdfs: N` line wants exactly N PDFs in its root's PDFs/ folder.
  The rule unsets the test hooks first, so nothing else of the caller's reaches them, and runs
  every scenario in English (`GRIMOIRE_LANG=en`, which ranks first) unless its `# env:` line names
  another language. With GRIMOIRE_TEST_PORT set every scenario's share server uses that port.
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
   socket: it takes its marks, compares the tiles with what it sent, sends the changed runs as gray
   PNGs and a batch end, at most about 30 batches a second, a ping after 15 s idle. A slow browser
   only lets the marks pile up; the pen loop never waits. While the pairing card is up the state is
   LIVE_PAIRING and nothing is sent: the card's code never reaches a stream (the viewer checks the
   state again after copying each run, and the app sets the state before the card is drawn). Asleep
   it is LIVE_ASLEEP (the sleep screen is never pushed); a turn bumps the epoch, and each viewer
   sends a new hello and every tile. Viewers end with a bye when Live view stops (Stop on the
   card, 10 minutes without a viewer, Grimoire leaving: `app_shutdown` stops them before anything
   else, so no export child inherits a socket and the headless runner never waits for one); on
   Linux they also have PR_SET_PDEATHSIG, and everywhere they end once their parent has gone.
   An icon-only chip in the top right corner (inside the 50 px margin) says Live view is on; it is
   overlaid at the present like the toast, and the screen guard skips it
   (`display_skip_overlay`). Auto-sleep waits while a computer watches. The log names no code,
   token or `/live` query; only the simulator logs the code and the viewers' pids, for
   tests/live_check.sh, which pairs as a browser, reads the stream (tests/live_stream.py, which
   decodes the PNGs itself), checks the browser's frame reader on the same bytes
   (tests/live_check.js), refuses a third viewer, and checks that no viewer outlives the app.
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

## Fast navigation and optional KOReader integration (1.1.0-beta.2)

The Pages screen (`pagesview.c`) draws a fixed screenful of numbered tiles, never `load_page` or
`page_render` for a preview. There is no preview cache. Selection is still by page index; a number
pad (keyboard page 2) validates a direct jump before the selected page is read. New page inserts
after the open page or the last selected page, using the notebook's paper. Copy page duplicates
the one selected page after itself, with a new page id and the original bytes. Image-paper copies
share their picture. Both keep the first page in place. A failed notebook-list save rolls the
insertion back and removes the new file. Book-page labels come from `source.ini`; a copy is a note
page and does not get the original's book-page mapping. The browser reader similarly
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
