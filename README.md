# Grimoire

**A handwriting notebook for the jailbroken Kindle Scribe.** Lined, dotted, graph and Cornell
paper, sheet music and guitar or bass tab you can mix and stack yourself, nine pens tuned to the
Scribe's pressure and tilt, folders, and three easy ways to get your pages onto a computer as PDF
or PNG. Everything stays on the Kindle in open files: no account, no cloud, no tracking.

It comes dressed like a castle in an old console game: pixel blackletter titles, pixel type,
carved-stone frames and pointed arches
(a plain **Classic** style is one tap away in Settings).

Free software under the GPL-3.0. Not affiliated with or endorsed by Amazon.

<p>
<img src="docs/screens/library.png" width="32%" alt="Library">
<img src="docs/screens/pens.png" width="32%" alt="Charcoal and the calligraphy pen">
<img src="docs/screens/music.png" width="32%" alt="Custom music paper builder">
</p>

> **Pre-release.** 1.0.0-rc.1 is in daily use on one Kindle Scribe (2024). It autosaves every
> stroke and keeps damaged files instead of overwriting them, but treat it as a release candidate:
> copy your notebooks off now and then (Library > Share makes that one click).

## Will it run on my Kindle?

You need a **Kindle Scribe** with a jailbreak that runs scriptlets (`.sh` files tapped from the
Library). Current jailbreaks for the Scribe (Vera, SpiderCat, WinterBreak, Sanctuary) all do;
see [kindlemodding.org](https://kindlemodding.org) to jailbreak.

| Kindle | Status |
|---|---|
| Kindle Scribe (2024, 2nd gen), firmware 5.17 to 5.19 | **Works.** This is the tested device (5.19.6, Vera). |
| Kindle Scribe (2022, 1st gen), firmware 5.16.3 or newer | **Should work.** Same platform. If the screen is upside down, see Troubleshooting. |
| Kindle Scribe (2022) on firmware older than 5.16.3 | Should run, but there is no Library launcher for those older jailbreaks yet. |
| Kindle Scribe 3 and Scribe Colorsoft (2025) | **Untested.** No public jailbreak yet. |
| Paperwhite, Oasis, Basic and other Kindles without a pen | Not useful: you cannot write without the pen. |

Firmware 5.20.1 is the newest as of October 2026. Check that your jailbreak survives it before
updating; most people keep OTA updates blocked.

## Install

1. Download `Grimoire-<version>.zip` from the
   [Releases](https://github.com/ganten1998/grimoire-scribe/releases) page and unzip it. You get a `documents` folder with two files.
2. Connect the Kindle to your computer. The Scribe uses MTP: on a Mac, use
   [OpenMTP](https://openmtp.ganeshrvel.com) (or `brew install --cask openmtp`); on Windows and
   Linux it shows up as a device.
3. Copy **both files** into the Kindle's `documents` folder:
   - `Grimoire.sh`, the app (it carries everything inside it)
   - `Grimoire - reset pen calibration.sh`, a rescue switch you will hopefully never need
4. Eject the Kindle. **Grimoire** appears in your Library with a gothic cover (indexing can take
   a minute). Tap it.

The first launch takes a few seconds while it unpacks itself into a `grimoire` folder. To
**update**, copy the new `Grimoire.sh` over the old one; your notebooks and settings stay.

**Coming from Quire** (Grimoire's old name)? Copy the new files and tap Grimoire once. It moves
your notebooks and settings from `quire/` to `grimoire/` and removes the old Quire launcher.

## Using it

- **Library**: notebooks as covers with thumbnails, one level of folders. Tap the `...` on a
  notebook to rename, move or delete it (deleted notebooks go to `grimoire/trash`, nothing is
  ever erased outright).
- **Paper**: narrow, medium and wide rule (with or without margin), Cornell, dot grid, graph,
  blank, sheet music, grand staff, guitar tab, 4 to 7 string bass tab, and **Custom music**:
  stack treble, bass, alto, tenor, drum and plain staves, tabs and lyric lines, and choose how
  many systems fit on a page. Each page can have its own paper.
- **Pens**: pencil, mechanical pencil, charcoal, ballpoint, fineliner, marker, fountain pen,
  calligraphy pen, highlighter. Every pen's pressure and tilt response can be tuned
  (Settings > Pen sensitivity). The eraser removes whole strokes; the eraser end of the Premium
  Pen works too.
  - **Charcoal** is grainy and crumbly, darker the harder you press; lay the pen low and it
    shades broad and soft with the side of the stick.
  - **The calligraphy pen** is a broad-edge nib that sits at 90 degrees to the way the pen
    leans, like a real one: lean toward the bottom right (a right hand) and you get the classic
    45 degree italic, thick going down to the right and hairline going up; lean toward the bottom
    left (a left hand) and the nib mirrors. Hold the pen upright and it keeps the 45 degree nib.
- **The floating panel** holds tools, undo/redo, clear page, pages, paper and the Library. Drag
  it anywhere; tap the corner arrows to fold it into a small box.
- **Pen button**: hold it to highlight (or erase; Settings), click it three times to clear the page.
- **Pages**: tap the page counter for a grid of previews; jump, or tick pages and delete them.
- **Pinch** to zoom up to 4x. Grimoire turns with the Kindle (switchable).
- **Sleep**: the power button sleeps the Kindle *inside* Grimoire, showing your page (or a pinned
  page, or the Kindle's own screensaver). Waking brings you straight back. Quit with
  **Library > Exit**.

## Getting pages off the Kindle

Details in [docs/EXPORT.md](docs/EXPORT.md).

1. **Wi-Fi**: turn on Wi-Fi, open **Library > Share**, and visit the address it shows from any
   browser on the same network. Download a notebook as PDF or zip, single pages as PNG, or
   everything at once. Only devices on your network can reach it, and only while the Share
   screen is open.
2. **USB**: every notebook you close is also saved as a PDF in `grimoire/PDFs/<folder>/`.
3. **SSH** (for the terminal-inclined): `tools/grimoire-pull` syncs everything and renders PDFs
   on your computer.

## Troubleshooting

- **Strokes land away from the pen tip**: Settings > Calibrate pen and touch. If you cannot reach
  it, tap **Grimoire - reset pen calibration** in the Library.
- **Everything is upside down** (possible on the 2022 Scribe): put an empty file named
  `screen-flip` or `screen-noflip` in the `grimoire` folder and relaunch.
- **Bits of the Kindle's own screen show through**: Grimoire repaints over them on its own. If
  it keeps happening, put an empty file named `stop-framework` in the `grimoire` folder; Grimoire
  then stops the Kindle interface completely while it runs (it takes a few seconds longer to
  come back after Exit).
- **Nothing happens when you tap Grimoire**: look at `grimoire/grimoire.log`. Please attach it
  to any bug report.

## Uninstall

Delete `Grimoire.sh` and `Grimoire - reset pen calibration.sh` from `documents`, then the
`grimoire` folder (copy `grimoire/notebooks` or `grimoire/PDFs` somewhere first if you want to
keep your pages).

## Building from source

Needs `zig` (the Kindle cross build), `clang`, and SDL3 for the desktop simulator.

```
make check     # unit tests, scripted UI scenarios, share server, exporter, ARM build
make sim       # desktop simulator window (docs/SIM.md)
make kindle    # build/kindle/grimoire: static ARMv7 hard-float (musl) binary
make package   # build/dist/Grimoire.zip
make tools     # build/host/grimoire-export (used by tools/grimoire-pull)
```

[docs/DESIGN.md](docs/DESIGN.md) describes the architecture, the refresh model, the brush model
and the file formats; [docs/KINDLE.md](docs/KINDLE.md) covers how it fits into the Kindle.

## License and credits

Grimoire is free software: GPL-3.0-or-later (see `LICENSE`), the same license as
[FBInk](https://github.com/NiLuJe/FBInk) by NiLuJe, which it uses to drive the e-ink screen.
Much of what Grimoire knows about the Kindle comes from [KOReader](https://koreader.rocks) and
[kindlemodding.org](https://kindlemodding.org).

- Terminess Nerd Font, Jacquard 24 and Jersey 10: SIL Open Font License 1.1
  (`assets/*-OFL-1.1.txt`)
- stb_truetype and stb_image_write by Sean Barrett: public domain / MIT

Kindle and Kindle Scribe are trademarks of Amazon.com, Inc. or its affiliates. Grimoire is an
independent project and is not affiliated with, endorsed by or supported by Amazon.
