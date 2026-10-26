# Ink performance work, 2.0.0-beta.4

The default drawing path now skips deposits onto pixels already at or below the tool's ink
value, calculates spacing compensation once for a smooth dab's fully covered interior, and
skips tilt/pressure calculations whose results cannot affect that dab. It keeps the same
formulas for edges and texture, the same saved samples and parameters, and the same dirty
rectangles. This is not a change to the tuned pen defaults.

The main loop processes at most 64 events or 8 ms of work before ticking and refreshing.
It checks the time after each event, so one slow event can exceed 8 ms. Remaining events stay
queued in order. The previous loop could wait for a continuously replenished queue to empty
before showing any of its ink. The backlog scenario checks intermediate live frames and all
257 stored pen points, plus a pixel comparison with a fresh render.

## Measured CPU work

On a 2024 Scribe, firmware 5.20.1.0.2, the standalone offscreen benchmark ran at low priority
without opening the display or input devices. Both builds used the app's Cortex-A7, static
ARM, `-O2` configuration. Results below are median mean microseconds per input sample from
three alternating before/after runs, each drawing 800 samples with the current defaults.
The baseline brush is from 40fb17b (beta.3). These are rendering CPU times, not pen-to-screen
latency; the display driver and panel are outside the measurement.

| Tool | White, before / after | Gray image, before / after | Black/white image, before / after |
| --- | ---: | ---: | ---: |
| Pencil | 67.8 / 70.2 | 67.8 / 68.4 | 66.1 / 41.8 |
| Mechanical pencil | 39.7 / 39.2 | 39.8 / 39.0 | 38.9 / 25.2 |
| Ballpoint | 40.3 / 36.2 | 40.5 / 36.3 | 39.5 / 23.5 |
| Eraser | 85.5 / 83.6 | 85.5 / 83.6 | 54.6 / 53.8 |
| Highlighter | 128.8 / 113.7 | 128.6 / 78.2 | 127.8 / 63.8 |
| Fountain pen | 47.6 / 40.6 | 47.8 / 39.9 | 47.3 / 26.2 |
| Fineliner | 26.9 / 24.6 | 27.0 / 24.5 | 26.4 / 17.0 |
| Marker | 128.7 / 132.5 | 128.1 / 133.0 | 123.2 / 74.1 |
| Charcoal | 118.4 / 121.8 | 118.4 / 122.4 | 117.0 / 70.6 |
| Calligraphy | 88.6 / 78.0 | 88.8 / 77.5 | 88.0 / 45.6 |

The early pixel check costs a few microseconds for some textured tools on light paper; it
saves much more over black image regions. Gray-background gains depend on the tool. Every
ARM comparison produced identical pixel/geometry hashes. A host comparison of 240 cases
also matched, including changed grain, opacity, size, DPI, clipping and white ink. Existing
golden-render and eraser checks pass without changing their expected images.

Build the standalone benchmark after generating the catalogs with `make headless`:

```sh
clang -O2 -std=gnu11 -D_GNU_SOURCE -Isrc tools/bench-ink.c src/brush.c \
  src/canvas.c src/util.c src/i18n.c build/gen/l10n.c -lm -o build/bench-ink
build/bench-ink
build/bench-ink --verify
```

For the Kindle, replace `clang` with `zig cc -target arm-linux-musleabihf -mcpu=cortex_a7
-static`. Compare hashes against the previous brush compiled with the same toolchain before
comparing timings. The inputs are synthetic; no notebook or captured handwriting is needed.

## Gray paper: one transition per pixel

DU can only drive a pixel to black or white, and a pixel cannot start a new transition until
its current one has ended. Beta.3 dithered every changed pixel of a live frame to black or
white. On white paper, ink going down turns a pixel black once. On gray paper, the first light
deposit of a dab's edge sent most pixels to *white*, and the dab's center a few milliseconds
later sent them to black. That black had to wait for the white transition to finish, so on gray
PDF pages and templates a large share of a stroke appeared a whole waveform later than on white
paper, inside a brief white halo.

Live frames now move what the panel shows only toward each pixel's new value: a darkening pixel
goes black or stays as shown, a lightening pixel goes white or stays as shown. The 8x8 ordered
dither decides which, scaled to the shown gray so the average darkness stays right. The GL16
settle on pen-up still shows the exact grays. Where the panel shows black or white this is the
same dither as before, which `tests/test_fbpack.c` checks for every value and dither position.
The same test fails 119,024 checks against the beta.3 rule (white over gray, second transitions).

`tools/du-model.c` draws strokes with the real brush and default tools, one fast frame per pen
sample, and packs them with `fbpack_row` both ways. In its panel model a transition takes T ms
and waits for the pixel's previous one. With T = 150 ms and 240 samples a second (beta.3 rule /
current rule; "black p90": 90% of the pixels that end black have finished turning black within
this time of their first ink):

| Tool, paper gray, speed | Transitions per inked pixel | Went white over gray | Black p90 |
| --- | ---: | ---: | ---: |
| Ballpoint, white 255, 20 mm/s | 0.59 / 0.59 | 0% / 0% | 162 / 162 ms |
| Ballpoint, gray 195, 20 mm/s | 1.38 / 0.58 | 70% / 0% | 300 / 162 ms |
| Ballpoint, gray 195, 60 mm/s | 1.23 / 0.58 | 55% / 0% | 300 / 154 ms |
| Pencil, gray 225, 60 mm/s | 1.14 / 0.41 | 66% / 0% | 300 / 154 ms |
| Marker, gray 195, 60 mm/s | 1.36 / 0.58 | 68% / 0% | 300 / 158 ms |

Results are the same shape at 100 and 260 ms waveforms and at 120 to 360 samples a second.
On white paper the two rules give identical frames. This is a model of the panel, not a
measurement of it: the real waveform times and the driver's collision handling are unknown,
so it says which way the change goes and roughly how far, and the Kindle has the final word.

```sh
clang -O2 -std=gnu11 -D_GNU_SOURCE -Isrc tools/du-model.c src/brush.c src/canvas.c \
  src/fbpack.c src/util.c src/i18n.c build/gen/l10n.c -lm -o build/du-model
build/du-model 150 240
```

## Opt-in display experiment

MTK pen mode remains off by default. FBInk documents a Bellatrix3 MDP bypass for DU/DUNM
updates smaller than 6000 pixels at framebuffer rotations 1 and 3. The optional `pen-mode`
flag now pads those refresh rectangles to at least 6400 pixels, bounded by the panel.
It does not repaint extra pixels or change the gray-paper preservation cache. Host checks
cover containment and every edge in portrait and landscape, including smaller panels.

This is a hypothesis to test on hardware, not evidence of reduced visible latency. A
comparison must cover gray PDF backgrounds, ordinary pages, normal/inverted display, all
orientations, pen-up settling, sleep/wake and return to KOReader. Leave the flag absent on
devices that have not passed that comparison.

The optional empty `grimoire/ink-metrics` file enables aggregate average/max framebuffer
copy and refresh-submission timing, logged at most once every 10 seconds of fast refreshes.
It records no pen coordinates or key values. Remove it and restart to disable measurement.
These times still exclude the panel's physical response. Both flags are developer switches,
not new controls in the writing interface.
