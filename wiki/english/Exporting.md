**English** | [日本語](Exporting-ja) | [Polski](Exporting-pl)

# Getting Pages Off the Kindle

Three roads. All of them use the same renderer, so a page looks identical on the Kindle, in a PNG,
and in a PDF.

## Wi-Fi: nothing to install

On the Kindle: turn Wi-Fi on (**Settings > Wi-Fi** also picks the network), then **Library >
Share**. It shows a QR code and, in big type under it, the same address, like
`http://192.168.1.23:8765`. Point a phone's camera at the code, or open the address in a browser
on any computer or phone on the same network.

- every folder and notebook with a thumbnail
- per notebook: **PDF**, **ZIP** (PDF + one PNG per page + the raw files), **Effects** (below),
  and a reader with one page at a time, a page counter and direct number entry at the top
- per folder: a zip; at the top: **Download everything**

Keep the Share screen open while downloading; leaving it stops the server. While the Kindle
sleeps on the Share screen the server answers nothing new: a page asked for then waits, and loads
after the wake if the browser has not given up.

The pages are in the language Grimoire is in. Download file names and the scriptable endpoints
are the same in every language. The raw files in a zip are the notebook's `notebook.ini`, its
`source.ini` when it annotates a book, its `p<id>.qpg` pages and its pictures (`bg/`).

Every export has the notebook's page size (1860x2480 or 1986x2648), whichever Kindle makes it.

### Effects

**Effects** (on every notebook, and under each page) opens a studio that makes new pictures of
your pages. The Kindle sends each page as two layers, the empty paper and the ink alone, and the
browser puts them back together your way:

- **Background**: white, transparent, a color, or a gradient
- **Paper lines**: shown or hidden, in any color and strength
- **Ink**: as written, one color, or a gradient across the writing
- **Look**: a tone, warmth, a tint, brightness, contrast
- **Crop and size**: the whole page or trimmed to the ink plus a margin; 100%, 50% or 25%
- **Presets**: Transparent, Signature, Warm, Sepia, Parchment, Blueprint, Chalkboard, Night,
  Sunset ink, Aurora

Download the page you are looking at as a PNG, or every page as a zip of PNGs or as one PDF.
Transparent PNGs keep the ink's soft edges as partial transparency. The browser remembers your
last settings. Every control is one ImageMagick operation, and the studio prints the `magick`
command that makes the same picture.

Security: the server has no password. It only listens while the Share screen is open, the
firewall hole is only on the Wi-Fi interface, it is read-only (GET), and it refuses requests whose
`Host` is not an IP address. Do not open Share on a network you do not trust.

## USB cable

With **Settings > Mirror PDFs to grimoire/PDFs** on (the default), closing a notebook writes
`grimoire/PDFs/<folder>/<name>.pdf` in the background. Renaming, moving or deleting a notebook
moves or removes its PDF too. Plug the Kindle in and copy them.

The raw notebooks are in the `grimoire` folder if you want everything. The PDFs are kept out of
`documents/` so the Kindle never indexes them.

## From a terminal

```sh
tools/grimoire-pull --http 192.168.1.23     # over Wi-Fi (Share screen open), no SSH needed
tools/grimoire-pull                         # over SSH: rsync + render here
tools/grimoire-pull --png --open            # also PNG pages; open the folder when done
```

Output: `~/Grimoire/<folder>/<notebook>.pdf` (and `<notebook>/page-NNN.png`); `$GRIMOIRE_OUT`
changes it.

SSH mode rsyncs `/mnt/us/grimoire/notebooks/` from `$GRIMOIRE_HOST` (default `kindle`) into
`~/Library/Caches/grimoire`, then runs `build/host/grimoire-export` (`make tools`). It is
incremental: only notebooks changed since their last export are rendered again.

`grimoire-export [--png] [--zip] [--force] <grimoire-root> <out-dir>` also works on any copy of the
`grimoire` folder, e.g. one copied off over USB. Build it from the same release as the Grimoire on
your Kindle (`make tools` again after an update).

## KOReader

KOReader opens the PDFs in `grimoire/PDFs` from its file browser, or with **Tools > Grimoire >
Notebook PDFs** when the plugin is installed. Grimoire writes a notebook's PDF again whenever the
notebook closes after a change, and renames, moves or deletes it with the notebook, so highlights
or notes KOReader writes into that file are lost: read these, and annotate copies.
