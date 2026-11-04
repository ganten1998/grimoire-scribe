#!/usr/bin/env python3
"""The Terminess diet: makes assets/TerminessNerdFontMono-Regular.ttf and -Bold.ttf from Nerd
Fonts' release, keeping only the code points assets/terminess-keep.txt lists.

For the maintainer, through `make fonts-subset` (assets/README.md). It needs fontTools 4.68.0
(python3 -m pip install fonttools==4.68.0, best in a venv outside the repo: the committed fonts are
what that version writes, and another may write other bytes); the gate never runs it.

  1. Downloads, once, into the cache folder: Nerd Fonts 3.4.0's Terminus.zip (the source of the
     shipped fonts) and Terminus TTF 4.49.3's zip (the upstream Terminus it patched), and checks
     both, and the four fonts in them, against their SHA-256.
  2. Checks the keep-list's text against upstream Terminus: every text code point must hold
     Terminus's own outline in Nerd Fonts' files, in both weights, and every code point outside
     the private use areas that does must be listed. Nerd Fonts' own glyphs at ordinary code
     points (its box drawing, the Octicons heart, the power symbols) fail this and stay out.
  3. Checks that every icon is a Material Design icon of the release (U+F0001-U+F1AF0, the range
     Nerd Fonts' font-patcher fills from Material Design Icons) and is in both weights.
  4. Subsets both weights to exactly the keep-list with fontTools: the glyph outlines, metrics,
     hinting (the maxp limits, cvt and gasp, the .notdef program) and embedded bitmaps of the kept
     glyphs are kept as they are, and the name table whole, so FreeType (KOReader) and
     stb_truetype (the app) draw them exactly as before. Dropped: the glyph names (post) and the
     old Mac Roman cmap, which no renderer here reads (they all read the Unicode cmaps).
  5. Checks the result (its cmap is exactly the keep-list, its name table is the source's) and
     writes the two files into the output folder.

With --verify it writes nothing: it makes both fonts in memory and fails unless they match the
files already in the output folder byte for byte (the same fontTools version makes the same bytes).
"""
import argparse
import hashlib
import io
import os
import shutil
import subprocess
import sys
import tempfile
import urllib.request
import zipfile

NERD_FONTS = {
    "url": "https://github.com/ryanoasis/nerd-fonts/releases/download/v3.4.0/Terminus.zip",
    "file": "nerd-fonts-3.4.0-Terminus.zip",
    "sha256": "b52a95517b920038396a64ea5ae9e4006541a63831c4a7397d5ce648a8d56d8e",
    "fonts": {
        "TerminessNerdFontMono-Regular.ttf": "fe0885bbed456ff953b87ad414b0eefd77a6d5e6774528b8440880a64a127d2c",
        "TerminessNerdFontMono-Bold.ttf": "cd6d9ae8d055e7edaa29abc6fa46a23f3a03b73184d48ae2c36acd7408e3a5ce",
    },
}
TERMINUS = {
    "url": "https://files.ax86.net/terminus-ttf/files/4.49.3/terminus-ttf-4.49.3.zip",
    "file": "terminus-ttf-4.49.3.zip",
    "sha256": "0ead921d98d99a4590ffe6cd66dc037fc0a2ceea1c735d866ba73fe058257577",
    "fonts": {
        "terminus-ttf-4.49.3/TerminusTTF-4.49.3.ttf": None,
        "terminus-ttf-4.49.3/TerminusTTF-Bold-4.49.3.ttf": None,
    },
}
# Each weight: the shipped file's name, the Nerd Fonts file it is cut from, the upstream Terminus.
WEIGHTS = [
    ("TerminessNerdFontMono-Regular.ttf", "TerminessNerdFontMono-Regular.ttf",
     "terminus-ttf-4.49.3/TerminusTTF-4.49.3.ttf"),
    ("TerminessNerdFontMono-Bold.ttf", "TerminessNerdFontMono-Bold.ttf",
     "terminus-ttf-4.49.3/TerminusTTF-Bold-4.49.3.ttf"),
]
MDI_FIRST, MDI_LAST = 0xF0001, 0xF1AF0
# The fontTools that made the committed fonts: --verify matches them byte for byte with it.
FONTTOOLS = "4.68.0"


def fail(msg):
    sys.exit("fonts-subset: " + msg)


def is_pua(cp):
    return 0xE000 <= cp <= 0xF8FF or cp >= 0xF0000


def sha256(data):
    return hashlib.sha256(data).hexdigest()


def read_keep(path):
    """The keep-list: a set of code points (one per line, or a range U+XXXX-U+YYYY; # comments)."""
    keep = set()
    with open(path, encoding="utf-8") as f:
        for n, line in enumerate(f, 1):
            line = line.split("#", 1)[0].strip()
            if not line:
                continue
            try:
                ends = [int(p.strip()[2:], 16) for p in line.split("-") if p.strip().upper().startswith("U+")]
            except ValueError:
                ends = []
            if len(ends) not in (1, 2) or len(ends) != len(line.split("-")) or ends[-1] < ends[0]:
                fail("%s:%d: not a code point or a range: %r" % (path, n, line))
            for cp in range(ends[0], ends[-1] + 1):
                if cp in keep:
                    fail("%s:%d: U+%04X is listed twice" % (path, n, cp))
                keep.add(cp)
    return keep


def fetch(cache, src):
    """The source zip's bytes, downloaded into the cache once and checked by SHA-256."""
    os.makedirs(cache, exist_ok=True)
    path = os.path.join(cache, src["file"])
    if not os.path.exists(path):
        part = path + ".part"
        print("fonts-subset: downloading %s" % src["url"])
        if shutil.which("curl"):
            subprocess.run(["curl", "-fsSL", "--retry", "3", "-o", part, src["url"]], check=True)
        else:
            with urllib.request.urlopen(src["url"]) as r, open(part, "wb") as out:
                shutil.copyfileobj(r, out)
        os.replace(part, path)
    with open(path, "rb") as f:
        data = f.read()
    if sha256(data) != src["sha256"]:
        fail("%s is not the expected download (SHA-256 %s, expected %s); delete it to fetch it again"
             % (path, sha256(data), src["sha256"]))
    files = {}
    with zipfile.ZipFile(io.BytesIO(data)) as z:
        for name, want in src["fonts"].items():
            files[name] = z.read(name)
            if want and sha256(files[name]) != want:
                fail("%s in %s has SHA-256 %s, expected %s" % (name, path, sha256(files[name]), want))
    return files


def outline(font, gname):
    glyf = font["glyf"]
    g = glyf[gname]
    if g.numberOfContours == 0 and not g.isComposite():
        return None
    coords, ends, flags = g.getCoordinates(glyf)
    return (tuple(coords), tuple(ends), tuple(f & 1 for f in flags))


def terminus_text(nerd, terminus):
    """The code points outside the private use areas where Nerd Fonts' file holds Terminus's own
    outline (and the ones where it does not)."""
    nc, tc = nerd.getBestCmap(), terminus.getBestCmap()
    same, other = set(), set()
    for cp, gname in nc.items():
        if is_pua(cp):
            continue
        if cp in tc and outline(terminus, tc[cp]) == outline(nerd, gname):
            same.add(cp)
        else:
            other.add(cp)
    return same, other


def names(font):
    return sorted((r.platformID, r.platEncID, r.langID, r.nameID, r.string) for r in font["name"].names)


def make(nerd_bytes, keep):
    """The subset font's bytes."""
    from fontTools import subset
    from fontTools.ttLib import TTFont
    opts = subset.Options()
    opts.layout_features = ["*"]
    opts.name_IDs = ["*"]
    opts.name_legacy = True
    opts.name_languages = ["*"]
    opts.notdef_outline = True
    opts.passthrough_tables = True  # FontForge's BDF strike properties and its font log
    opts.drop_tables = [t for t in opts.drop_tables if t not in ("EBDT", "EBLC", "EBSC")]
    opts.hinting = True
    opts.recalc_bounds = False
    opts.recalc_timestamp = False
    font = TTFont(io.BytesIO(nerd_bytes), recalcBBoxes=False, recalcTimestamp=False, lazy=True)
    sub = subset.Subsetter(options=opts)
    sub.populate(unicodes=sorted(keep))
    sub.subset(font)
    out = io.BytesIO()
    subset.save_font(font, out, opts)
    return out.getvalue()


def check_result(data, src, keep, label):
    from fontTools.ttLib import TTFont
    got = TTFont(io.BytesIO(data))
    cmap = set(got.getBestCmap())
    if cmap != keep:
        fail("%s: the subset's cmap is not the keep-list (missing %s, extra %s)"
             % (label, ["U+%04X" % c for c in sorted(keep - cmap)][:20], ["U+%04X" % c for c in sorted(cmap - keep)][:20]))
    if names(got) != names(src):
        fail("%s: the subset's name table differs from the source's" % label)
    for tag in ("cvt ", "gasp", "EBDT", "EBLC", "BDF "):
        if (tag in src) != (tag in got):
            fail("%s: the subset lost the %r table" % (label, tag))
    for field in ("maxSizeOfInstructions", "maxFunctionDefs", "maxStorage", "maxZones", "maxStackElements"):
        if getattr(got["maxp"], field) != getattr(src["maxp"], field):
            fail("%s: maxp.%s changed (it decides how FreeType hints)" % (label, field))
    if len(got["EBLC"].strikes) != len(src["EBLC"].strikes):
        fail("%s: the subset lost embedded bitmap strikes" % label)
    for table, fields in (("hhea", ("ascent", "descent", "lineGap", "advanceWidthMax")),
                          ("head", ("unitsPerEm", "xMin", "yMin", "xMax", "yMax", "flags")),
                          ("OS/2", ("usWinAscent", "usWinDescent", "sTypoAscender", "sTypoDescender", "xAvgCharWidth"))):
        for field in fields:
            if getattr(got[table], field) != getattr(src[table], field):
                fail("%s: %s.%s changed" % (label, table, field))


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--cache", required=True, help="download folder, outside the repo")
    ap.add_argument("--keep", required=True, help="assets/terminess-keep.txt")
    ap.add_argument("--out", required=True, help="where the two fonts go (assets)")
    ap.add_argument("--verify", action="store_true", help="write nothing; compare with the fonts in --out")
    args = ap.parse_args()
    try:
        import fontTools
        from fontTools.ttLib import TTFont
    except ImportError:
        fail("needs fontTools: python3 -m pip install fonttools==%s (in a venv), then "
             "make fonts-subset FONTS_PYTHON=<that venv>/bin/python" % FONTTOOLS)
    print("fonts-subset: fontTools %s" % fontTools.version)
    if fontTools.version != FONTTOOLS:
        print("fonts-subset: warning: the committed fonts were made with fontTools %s; this one may "
              "write other bytes (python3 -m pip install fonttools==%s)" % (FONTTOOLS, FONTTOOLS))
    keep = read_keep(args.keep)
    text = {cp for cp in keep if not is_pua(cp)}
    icons = keep - text
    nerd = fetch(args.cache, NERD_FONTS)
    upstream = fetch(args.cache, TERMINUS)
    made = {}
    for out_name, nerd_name, terminus_name in WEIGHTS:
        src = TTFont(io.BytesIO(nerd[nerd_name]))
        same, other = terminus_text(src, TTFont(io.BytesIO(upstream[terminus_name])))
        if text - same:
            fail("%s: listed as text but not Terminus's own glyph there: %s"
                 % (nerd_name, ["U+%04X" % c for c in sorted(text - same)]))
        if same - text:
            fail("%s: Terminus's own glyphs missing from the keep-list: %s"
                 % (nerd_name, ["U+%04X" % c for c in sorted(same - text)]))
        cmap = src.getBestCmap()
        bad = sorted(cp for cp in icons if not (MDI_FIRST <= cp <= MDI_LAST) or cp not in cmap)
        if bad:
            fail("%s: not a Material Design icon of the release: %s" % (nerd_name, ["U+%05X" % c for c in bad]))
        print("fonts-subset: %s: %d text code points (Terminus's own; %d Nerd Fonts glyphs at ordinary "
              "code points left out), %d icons" % (nerd_name, len(text), len(other), len(icons)))
        data = make(nerd[nerd_name], keep)
        check_result(data, src, keep, out_name)
        made[out_name] = data
    if args.verify:
        for name, data in made.items():
            path = os.path.join(args.out, name)
            with open(path, "rb") as f:
                if f.read() != data:
                    fail("%s is not what the recipe makes from the keep-list" % path)
            print("fonts-subset: %s matches (%d bytes, SHA-256 %s)" % (path, len(data), sha256(data)))
        return
    for name, data in made.items():
        path = os.path.join(args.out, name)
        fd, tmp = tempfile.mkstemp(dir=args.out, prefix="." + name, suffix=".tmp")
        with os.fdopen(fd, "wb") as f:
            f.write(data)
        os.chmod(tmp, 0o644)
        os.replace(tmp, path)
        print("fonts-subset: wrote %s (%d bytes, SHA-256 %s)" % (path, len(data), sha256(data)))


if __name__ == "__main__":
    main()
