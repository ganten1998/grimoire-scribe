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

## The panel's own timing

The Scribe 2 keeps its waveform on a small FAT partition (`wfm`, mmcblk0p2): a gzipped table of
14 modes by 14 temperature ranges, each a list of 85 Hz frames (11.8 ms). Every frame says, for
each pair of 32 levels, whether to drive the pixel toward black, toward white, or not at all.
From the owner's device (`400_B212_HL4702_ED103KC1C6_TC`), at 24 to 27 °C (the driver's temp
zone 8, `/proc/hwtcon/temp`):

| Mode | Frames | Length |
| --- | ---: | ---: |
| DU | 16 | 188 ms |
| A2 | 8 | 94 ms |
| GL16 | 23 | 271 ms |
| GC16 | 29 | 341 ms |

The lengths grow as it gets colder: at 15 to 18 °C DU takes 25 frames (294 ms), below 3 °C 53.

DU starts a white pixel toward black on its first frame. From a gray it first drives the pixel
toward white, then to black:

| Shown gray (8 bit) | Ink starts darkening after | Frames toward white first |
| --- | ---: | ---: |
| 240 to 255 | 0 ms | 0 |
| 208 to 239 | 12 ms | 0 (one idle frame) |
| 192 to 207 | 12 ms | 1 |
| 176 to 191 | 24 ms | 2 |
| 144 to 175 | 47 ms | 4 |
| 128 to 143 | 59 ms | 4 (after an idle frame) |
| 32 to 127 | 59 ms | 5 |
| 16 to 31 | 71 ms | 5 (after an idle frame) |

Inverted, ink goes to white: from the dark grays of an inverted light page it starts after 35
to 59 ms, waiting rather than moving the wrong way first. Every other mode is slower from gray
(GL16, GC16 and their relatives reverse for longer) and A2 does not drive a gray pixel at all,
so DU stays the live waveform. On a page printed in mid gray (the cover of a book KOReader sent,
157) ink cannot start before 47 ms on this panel, against 0 ms on white paper, and it first
turns lighter: that difference is the panel's, and no update order removes it. A2 drives white
to black with the same frames as the first 7 of DU and stops there, so ink would not appear
sooner with it, only finish sooner.

## Pen mode

MTK pen mode is the driver's path for handwriting. FBInk documents a Bellatrix3 MDP bypass for
DU/DUNM updates smaller than 6000 pixels at framebuffer rotations 1 and 3 (the Scribe 2 runs at 1
or 3), and warns that pen-mode updates must never collide with standard ones.

Measured on the owner's Scribe 2 on 2026-10-26 with the latency report below and `latency-ab`
(about 40 strokes over a notebook page, a gray book cover normal and inverted, and a white music
page, panel at 25 °C):

| Frames sent as | Pen sample to refresh ioctl | Refresh to start on the panel | Waveform done |
| --- | ---: | ---: | ---: |
| plain DU | p50 0.2-0.3 ms | p50 1-114 ms, p90 up to 302 ms | about 190 ms, often never reported |
| pen mode, direct | p50 0.2-0.4 ms | p50 0.2-1.0 ms, max 6.7 ms | 184-190 ms |
| pen mode, padded | p50 0.2-0.3 ms | p50 1.0 ms, max 7.7 ms | 188-190 ms |

Plain DU frames that overlap ink still on the panel wait in the driver's queue: quick circles on
the inverted cover queued up to 302 ms, and note heads on the music page half the sampled frames
over 114 ms. Pen-mode frames never queued. Direct pen-mode ink landed in the right place at fb
rotate 1. The pen service's relay from the raw digitizer took p50 0.1 ms, so reading the raw
digitizer directly would gain nothing.

So pen mode is on by default on grayscale panels, in the direct form. The `pen-mode` flag's
first word overrides it: `off` for plain DU, `padded` to pad small updates to at least 6400
pixels, bounded by the panel, so the MDP takes them; anything else (an empty file too) is the
direct form. Colour panels, where Grimoire has never run, keep plain DU unless the flag asks.
Host checks cover the padding's containment and every edge in portrait and landscape.

A stroke enters pen mode only when no standard update can still be on the panel: a thread with
its own framebuffer fd waits for every standard update's end (in order for grayscale ones, whose
waveforms differ in length; the newest for DU ones). The driver then stays in pen mode until a
grayscale refresh is due, and that refresh first waits for the pen's last update. A stroke that
finds the panel busy keeps plain DU. In the test 7 of 24 pen strokes did: 2 right after a block
of plain DU strokes, the others most likely because the grayscale settle, then sent 450 ms after
a stroke, was still on the panel when the next word began. The settle now waits until the pen
leaves the page (150 ms gone), or 1.5 s with it still near, so writing does not meet it.

## Input smoothing

A pen's Smoothing (brush.c, `smoother_apply`) eases each sample toward the last smoothed one, a
little more for slow, careful motion. The drawn line trails the nib by a steady distance, which
reads as lag. With samples every 3 ms (about 330 a second, as the Scribe 2 reports them) the
line trails by, at smoothing 1.00 / 0.50 / 0.25:

| Writing speed | 1.00 | 0.50 | 0.25 |
| --- | ---: | ---: | ---: |
| 10 mm/s | 13 ms | 2.0 ms | 0.7 ms |
| 30 mm/s | 10 ms | 1.8 ms | 0.7 ms |
| 60 mm/s | 7.5 ms | 1.7 ms | 0.6 ms |
| 100 mm/s | 6 ms | 1.4 ms | 0.5 ms |

The ballpoint's default is 0.50 since 2.0.0-beta.4 (it was 1.00). The fineliner keeps 1.00, the
marker and calligraphy pen 0.50.

## The latency report

The optional empty `grimoire/ink-metrics` file enables aggregate average/max framebuffer
copy and refresh-submission timing, and the latency of live ink by how each frame went out
(plain DU, pen mode, padded pen mode), logged at most once every 10 seconds of fast refreshes:

- pen->submit: from the kernel time of the newest pen sample in the frame to the return of the
  refresh ioctl (both pens' events are switched to CLOCK_MONOTONIC for this reader only);
- oldest pen->submit: the same for the frame's oldest sample;
- raw->submit and the relay hop: from the raw digitizer's sample (WacomDigitizer), which the
  Kindle's pen service relays as the scaled pen Grimoire reads, matched to it by position;
- for one frame at a time, followed by a second thread: submit->start (the driver took it to
  the panel, MXCFB_WAIT_FOR_UPDATE_SUBMISSION) and submit->done (its waveform ended,
  MXCFB_WAIT_FOR_UPDATE_COMPLETE), also for frames within 600 ms of a grayscale refresh, with
  the panel temperature.

With it, an empty `grimoire/latency-ab` makes ink strokes take turns in blocks of 12 at plain
DU, direct and padded pen mode, so one session compares them on the same pages and hand. Both
files are read again every 2 seconds, so a test needs no restart. The report records no pen
coordinates or key values. Remove `ink-metrics` and restart to disable measurement. Both are
developer switches, not new controls in the writing interface.
