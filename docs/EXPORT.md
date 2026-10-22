# Getting notebooks onto a computer

Three roads. All of them use the same renderer (`src/export.c`), so a page looks identical
on the Kindle, in a PNG, and in a PDF.

## 1. Wi-Fi: nothing to install

On the Kindle: turn Wi-Fi on, then **Library > Share**. It shows a QR code and, in big type
under it, the same address, like `http://192.168.1.23:8765`. Point a phone's camera at the code,
or open the address in a browser on any computer or phone on the same network:

- every folder and notebook with a thumbnail
- per notebook: **PDF**, **ZIP** (PDF + one PNG per page + the raw files), **Effects** (below),
  and a page view where each page is a full-resolution PNG
- per folder: a zip; at the top: **Download everything**

The code holds the Kindle's Wi-Fi (wlan0) address, the only one the launcher opens the share port
on. Other addresses, such as USBNetwork's usb0, are listed under it as small "Also:" lines and are
not in the code; with USBNetwork only there is no code, and the Wi-Fi icon stays. Turned
sideways, the screen puts the code on the left and the text on the right. With Invert on, the code
is drawn as its negative, so the panel still shows it dark on light.

Keep the Share screen open while downloading; leaving it stops the server. Turning the Kindle, or
waking it on the Share screen, keeps the server and its downloads going. While the Kindle sleeps
on the Share screen the server answers nothing new: a page asked for then waits, and loads after
the wake if the browser has not given up. The sleep screen then shows "Asleep" and "Wake the
Kindle to share again." instead of the code and the address (unless a pinned page or the Kindle's
own screensaver is the sleep screen). The pages use Grimoire's own fonts in the style picked on
the Kindle (Gothic: Jacquard 24 titles and Jersey 10 text, sized to stay pixel-sharp; Classic:
Terminess).

The pages are in the language Grimoire is in (Settings > Language, English or Spanish), and their
`<html lang>` says which: the index's line ("2 cuadernos en tu Kindle"), its headings, dates ("10
oct, 14:32") and links, each notebook's paper, a notebook's page and the Effects studio. So are the
plain-text answers a person can meet in a browser: not found, no such notebook, export failed, and
"use the address shown on the Kindle"; the ones only a script or a hand-made address gets stay
English. The download file names, the endpoints and `/api/list.json` (whose `paper` stays the
English name) are the same in every language.

The raw files in a zip are the notebook's `notebook.ini`, its `source.ini` when it annotates a book
(KOReader's Annotate this page), its `p<id>.qpg` pages and its pictures (`bg/`), whose format
is in [FORMAT.md](FORMAT.md). PNG, PDF and Share renders show a page from a newer Grimoire (one
that opens read-only on the Kindle) as far as they can: a newer page format shows blank, and
strokes with tools this version does not know are left out.

Every export has the notebook's page size, whichever Kindle makes it: 1860x2480 for a notebook
made on a 2022 or 2024 Scribe, 1986x2648 for one made on a Scribe 3 or a Scribe Colorsoft. At 300
dpi their PDF pages are 6.20x8.27 in and 6.62x8.83 in. A page an older version stretched to
another Scribe's screen (FORMAT.md, Page size) exports at its notebook's size too, and a missing
page as blank paper of that size.

### Effects

**Effects** (on every notebook, and under each page) opens a studio that makes new pictures of
your pages. The Kindle sends each page as two layers, the empty paper and the ink alone, and the
browser puts them back together your way:

- **Background**: white, transparent, a color, or a gradient (two colors and an angle)
- **Paper lines**: shown or hidden, in any color and strength (music staves are paper too)
- **Ink**: as written, one color, or a gradient across the writing (or across the whole page)
- **Look**: a tone (gray, duotone, invert), warmth (cool to warm), a tint, brightness, contrast
- **Crop and size**: the whole page or trimmed to the ink plus a margin; 100%, 50% or 25%
- **Presets** to start from: Transparent, Signature (transparent, gradient ink, trimmed),
  Warm, Sepia, Parchment, Blueprint, Chalkboard, Night, Sunset ink, Aurora

Download the page you are looking at as a PNG, or every page as a zip of PNGs or as one PDF.
Transparent PNGs keep the ink's soft edges (pencil grain, light pressure) as partial
transparency, so they sit cleanly on any background. The browser remembers your last settings.

Every control is one ImageMagick operation, and the studio prints the `magick` command (ImageMagick 7,
macOS or Linux shell) that makes the same picture from the page's two layer files, linked right
above it: handy for scripts and batch work. `tests/studio_check.js` holds the studio's pixels
against ImageMagick's for a few dozen settings; they agree to within one color level.

The browser does the image work; the Kindle only renders the layers, two at a time. Previews
use them at half size, every page with the same paper shares one paper layer, and flipping
through pages only loads the page you stop on (←/→ or type a page number).

Security: the server has no password. It only listens while the Share screen is open, the
firewall hole is only on the Wi-Fi interface, it is read-only (GET), and it refuses requests
whose `Host` is not an IP address (so a web page cannot reach it through a rebound hostname).
Do not open Share on a network you do not trust. PDF, zip and PNG downloads are rendered by a
separate process, so the Kindle stays responsive during a big download; it renders two at a
time and the rest wait their turn.

Scriptable endpoints:

| path | returns |
|---|---|
| `/api/list.json` | listing (below) |
| `/nb/<id>.pdf` | notebook PDF |
| `/nb/<id>.zip` | notebook zip |
| `/nb/<id>/<n>.png` | page n (1-based), a grayscale PNG at the notebook's page size (1860x2480 or 1986x2648) |
| `/nb/<id>/<n>.png?layer=ink` | page n's ink alone, on white |
| `/nb/<id>/<n>.png?layer=paper` | page n's paper alone, without the ink |
| `/nb/<id>/<n>.png?f=2` | any of the above shrunk 2x (`f` = 2 to 8); headers `X-Page-Size` (the full size, the notebook's), `X-Paper` (paper kind) and, for the ink, `X-Ink-Box` (`x,y,w,h` at full size, or `none`) |
| `/nb/<id>/studio` | the Effects studio (`?p=<n>` opens page n) |
| `/nb/<id>/thumb.png` | thumbnail |
| `/folder/<name>.zip` | every notebook in a folder (PDF + raw) |
| `/all.zip` | everything, grouped by folder (PDF + raw) |

```json
{"device": "Grimoire", "version": "1.0.0-beta.2",
 "notebooks": [{"id": "20261010-183859-1fdf", "name": "Lecture notes", "folder": "School",
                "pages": 3, "paper": "Medium rule", "modified": 1791672139,
                "pdf": "/nb/20261010-183859-1fdf.pdf", "zip": "/nb/20261010-183859-1fdf.zip",
                "pngs": ["/nb/20261010-183859-1fdf/1.png", "..."]}],
 "folders": ["School"]}
```

## 2. From a terminal: `tools/grimoire-pull`

```
tools/grimoire-pull --http 192.168.1.23     # over Wi-Fi (Share screen open), no SSH needed
tools/grimoire-pull                         # over SSH: rsync + render here
tools/grimoire-pull --png --open            # also PNG pages; open the folder when done
```

Output: `~/Grimoire/<folder>/<notebook>.pdf` (and `<notebook>/page-NNN.png`), `$GRIMOIRE_OUT` to change.

SSH mode rsyncs `/mnt/us/grimoire/notebooks/` from `$GRIMOIRE_HOST` (default `kindle`) into
`~/Library/Caches/grimoire`, then runs `build/host/grimoire-export` (`make tools`). It is incremental:
only notebooks changed since their last export are rendered again. You need an SSH server on
the Kindle (an SSH / USBNetwork package for your jailbreak; look in KPM or KindleForge; over USB
networking the Kindle typically answers at 192.168.15.244). If you don't have one, the Wi-Fi mode
above needs nothing extra. A `~/.ssh/config` entry keeps it short:

```
Host kindle
    HostName 192.168.15.244
    User root
```

`grimoire-export [--png] [--zip] [--force] <grimoire-root> <out-dir>` also works on any copy of the
`grimoire` folder, e.g. one copied off over USB.

Build `grimoire-export` from the same release as the Grimoire on your Kindle (`make tools` again
after an update): an older one, 1.0.0-beta's say, shows pages in a newer format blank. From 1.1
on it renders every page at its notebook's page size, as the Kindle does; an older one renders
each page at the size its file has (and a missing page at 1860x2480).

## 3. USB cable

With **Settings > Mirror PDFs to grimoire/PDFs** on (the default), closing a notebook writes
`grimoire/PDFs/<folder>/<name>.pdf` in the background. Renaming, moving or deleting a notebook moves
or removes its PDF too. Plug the Kindle in and copy them. (Not in documents/: the Kindle indexes every PDF there, and each one made its home screen redraw on top of Grimoire.)
The raw notebooks are in the `grimoire` folder if you want everything.

KOReader opens these PDFs with Tools > Grimoire > Notebook PDFs (with Grimoire's KOReader plugin:
docs/KINDLE.md), or from its file browser. Grimoire writes a notebook's PDF again whenever the
notebook closes after a change, and renames, moves or deletes it with the notebook, so highlights
or notes KOReader writes into it are lost: read these, and annotate copies.
