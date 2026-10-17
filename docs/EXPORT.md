# Getting notebooks onto a computer

Three roads. All of them use the same renderer (`src/export.c`), so a page looks identical
on the Kindle, in a PNG, and in a PDF.

## 1. Wi-Fi: nothing to install

On the Kindle: turn Wi-Fi on, then **Library > Share**. It shows an address like
`http://192.168.1.23:8765`. Open it in a browser on any computer or phone on the same network:

- every folder and notebook with a thumbnail
- per notebook: **PDF**, **ZIP** (PDF + one PNG per page + the raw files), and a page view
  where each page is a full-resolution PNG
- per folder: a zip; at the top: **Download everything**

Keep the Share screen open while downloading; leaving it stops the server.

Security: the server has no password. It only listens while the Share screen is open, the
firewall hole is only on the Wi-Fi interface, it is read-only (GET), and it refuses requests
whose `Host` is not an IP address (so a web page cannot reach it through a rebound hostname).
Do not open Share on a network you do not trust. PDF, zip and PNG downloads are rendered by a
separate process, so the Kindle stays responsive during a big download (two at a time).

Scriptable endpoints:

| path | returns |
|---|---|
| `/api/list.json` | listing (below) |
| `/nb/<id>.pdf` | notebook PDF |
| `/nb/<id>.zip` | notebook zip |
| `/nb/<id>/<n>.png` | page n (1-based), 1860x2480 grayscale PNG |
| `/nb/<id>/thumb.png` | thumbnail |
| `/folder/<name>.zip` | every notebook in a folder (PDF + raw) |
| `/all.zip` | everything, grouped by folder (PDF + raw) |

```json
{"device": "Grimoire", "version": "0.1.0",
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

## 3. USB cable

With **Settings > Mirror PDFs to grimoire/PDFs** on (the default), closing a notebook writes
`grimoire/PDFs/<folder>/<name>.pdf` in the background. Renaming, moving or deleting a notebook moves
or removes its PDF too. Plug the Kindle in and copy them. (Not in documents/: the Kindle indexes every PDF there, and each one made its home screen redraw on top of Grimoire.)
The raw notebooks are in the `grimoire` folder if you want everything.
