#!/usr/bin/env python3
"""The Classic fonts: makes assets/AtkinsonHyperlegibleNext-Regular.ttf and -Bold.ttf, NotoSans-*
and Literata-*, the three families the Font sheet offers beside Terminess, from Google Fonts'
releases, with each family's OFL text beside them.

For the maintainer, through `make fonts-subset` (assets/README.md), which runs it after the
Terminess diet. It needs fontTools 4.68.0 (python3 -m pip install fonttools==4.68.0, best in a venv
outside the repo: the committed fonts are what that version writes, and another may write other
bytes); the gate never runs it.

  1. Downloads, once, into the cache folder: each family's variable font and OFL.txt from Google
     Fonts' repository (github.com/google/fonts) at the commit that added the version shipped, and
     checks each file against its SHA-256.
  2. Makes each weight a static font: fontTools' instancer pins the weight axis at 400 (Regular) or
     700 (Bold) and every other axis at its default (Literata's optical size at 12, its default,
     the sturdy text size; Noto Sans's width at 100). stb_truetype reads no variations, so each
     weight is a font of its own.
  3. Subsets it to the text Grimoire's screens draw, as much of it as the source has: Basic Latin,
     Latin-1, Latin Extended-A, General Punctuation, and the few symbols the keyboard and the screens
     use (the euro, the minus sign, the arrows, the replacement character). Not every family has all
     of them (Noto Sans has no arrows; Literata no long s and no replacement character; Atkinson
     Hyperlegible Next neither of those nor the arrows, the micro sign, the soft hyphen and some
     Latin Extended-A letters): step 4 names what is missing outside General Punctuation, and
     Terminess draws it. Kerning stays (the kern feature: stb reads its pairs), hinting goes (stb
     draws unhinted, at 300 dpi), and so do the glyph names, the substitutions, the axes' names and
     the vertical metrics, which nothing here reads. The names kept are the copyright, the family,
     the style, the version and the license (IDs 0 to 6, 13, 14). Greek, Cyrillic and Japanese are
     left out: Terminess draws Greek and Cyrillic, DotGothic16 Japanese, as for every theme
     (rust/src/font.rs's fallback).
  4. Checks the result (every character every family must have is there, REQUIRED below, and the
     others of the subset the source lacks are named; the weight is the one asked, the family name
     is the source's) and writes the font and the family's OFL text into the output folder.

With --verify it writes nothing: it makes every font in memory and fails unless each matches the
file already in the output folder byte for byte (the same fontTools version makes the same bytes).
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

GF = "https://raw.githubusercontent.com/google/fonts"
# Each family: the files' stem here, Google Fonts' folder and the commit that added the version
# shipped (each at least two weeks old when it came in: the project's rule), the variable font and
# the OFL text there with their SHA-256, the version the font names, and the axes besides weight.
FAMILIES = [
    {
        "stem": "AtkinsonHyperlegibleNext",
        "family": "Atkinson Hyperlegible Next",
        "dir": "ofl/atkinsonhyperlegiblenext",
        "commit": "66eef70728ca29f28c31cdc5e7947f2c2de73e8d",  # 2025-01-07, version 2.001
        "font": "AtkinsonHyperlegibleNext[wght].ttf",
        "font_sha256": "5a455d1cfa099b601ab70751bb9673e8fe1854dc4500c80e1a220d0d75e31745",
        "ofl_sha256": "aca6a428580965d2297d1b718042dd427c2a9443ece3b0d02d758e161e0c4030",
        "version": "Version 2.001",
        "axes": {},
    },
    {
        "stem": "NotoSans",
        "family": "Noto Sans",
        "dir": "ofl/notosans",
        "commit": "2984c575fdce412ee02b2baaba67672b9a9434d8",  # 2024-11-20, version 2.015
        "font": "NotoSans[wdth,wght].ttf",
        "font_sha256": "bfb7bb691513f12e734dc346c03a03f784912432d7e3fa8e56efcf906fe86b3d",
        "ofl_sha256": "cee9892f9f0cc8fe882c9e9537ee6a89621d86ee7ceaf70b02e2b2b1c25c061a",
        "version": "Version 2.015",
        "axes": {"wdth": 100},
    },
    {
        "stem": "Literata",
        "family": "Literata",
        "dir": "ofl/literata",
        "commit": "c8018e986708f2aed63c928b1e9026826da1553d",  # 2023-05-19, version 3.103
        "font": "Literata[opsz,wght].ttf",
        "font_sha256": "b41138c9373112f32abb589cc22e8674b06ed4048b0c513be922bdd26f274440",
        "ofl_sha256": "8742963604cd89dc81437811a850018fc03b2bfad686d7422c8235967c87614e",
        "version": "Version 3.103",
        "axes": {"opsz": 12},
    },
]
WEIGHTS = [("Regular", 400), ("Bold", 700)]
# Basic Latin, Latin-1, Latin Extended-A, General Punctuation; the euro, the arrows, the minus sign
# (the count pad's key), the replacement character (what broken UTF-8 decodes to).
UNICODES = (list(range(0x20, 0x7F)) + list(range(0xA0, 0x180)) + list(range(0x2000, 0x2070))
            + [0x20AC, 0x2190, 0x2191, 0x2192, 0x2193, 0x2212, 0xFFFD])
# What every family must have: printable ASCII and every other character the translations' Latin
# text uses (rust/tests/test_i18n.rs checks the catalogs against the fonts themselves), and the
# keyboard's accent and symbol pages (rust/src/keyboard.rs's K_ROWS).
REQUIRED = (list(range(0x20, 0x7F)) + [ord(c) for c in
            "\u00a1\u00a3\u00ab\u00b0\u00b7\u00bb\u00bf\u00c1\u00c9\u00cd\u00d1\u00d3\u00d7\u00da\u00dc"
            "\u00df\u00e0\u00e1\u00e2\u00e3\u00e4\u00e5\u00e6\u00e7\u00e8\u00e9\u00ea\u00eb\u00ec\u00ed"
            "\u00ee\u00ef\u00f1\u00f2\u00f3\u00f4\u00f5\u00f6\u00f8\u00f9\u00fa\u00fb\u00fc\u0153"
            "\u2019\u2026\u2039\u203a\u20ac\u2212"])
NAME_IDS = [0, 1, 2, 3, 4, 5, 6, 13, 14]
# The fontTools that made the committed fonts: --verify matches them byte for byte with it.
FONTTOOLS = "4.68.0"


def fail(msg):
    sys.exit("fonts-classic: " + msg)


def sha256(data):
    return hashlib.sha256(data).hexdigest()


def fetch(cache, fam, name, want):
    """A file of the family's folder at its commit: downloaded into the cache once, checked by
    SHA-256."""
    os.makedirs(cache, exist_ok=True)
    path = os.path.join(cache, "%s-%s-%s" % (fam["stem"], fam["commit"][:12], name.replace("[", "_").replace("]", "_")))
    url = "%s/%s/%s/%s" % (GF, fam["commit"], fam["dir"], name.replace("[", "%5B").replace("]", "%5D"))
    if not os.path.exists(path):
        part = path + ".part"
        print("fonts-classic: downloading %s" % url)
        if shutil.which("curl"):
            subprocess.run(["curl", "-fsSL", "--retry", "3", "-o", part, url], check=True)
        else:
            with urllib.request.urlopen(url) as r, open(part, "wb") as out:
                shutil.copyfileobj(r, out)
        os.replace(part, path)
    with open(path, "rb") as f:
        data = f.read()
    if sha256(data) != want:
        fail("%s is not the expected download (SHA-256 %s, expected %s); delete it to fetch it again"
             % (path, sha256(data), want))
    return data


def set_names(font, fam, style, weight):
    """The static font's names and style bits, as a family of two weights has them: the family's
    name (Literata's own name table would call an instance at 12 pt "Literata 12pt"), Regular or
    Bold, and the bits that pair the two."""
    ps = "%s-%s" % (fam["family"].replace(" ", ""), style)
    name = font["name"]
    for nid in (1, 2, 3, 4, 6, 16, 17, 21, 22, 25):
        name.removeNames(nameID=nid)
    for nid, text in ((1, fam["family"]), (2, style), (3, "%s;%s" % (fam["version"][len("Version "):], ps)),
                      (4, "%s %s" % (fam["family"], style)), (6, ps)):
        name.setName(text, nid, 3, 1, 0x409)
    os2 = font["OS/2"]
    os2.usWeightClass = weight
    bold = weight >= 700
    os2.fsSelection = (os2.fsSelection & ~0b1100001) | (1 << 5 if bold else 1 << 6)
    font["head"].macStyle = (font["head"].macStyle & ~0b11) | (1 if bold else 0)


def make(src_bytes, fam, style, weight):
    """One weight of the family, static and subset: the font's bytes."""
    from fontTools import subset
    from fontTools.ttLib import TTFont
    from fontTools.varLib import instancer
    font = TTFont(io.BytesIO(src_bytes), recalcBBoxes=False, recalcTimestamp=False)
    loc = {a.axisTag: a.defaultValue for a in font["fvar"].axes}
    loc.update(fam["axes"])
    loc["wght"] = weight
    font = instancer.instantiateVariableFont(font, loc)
    set_names(font, fam, style, weight)
    opts = subset.Options()
    opts.layout_features = ["kern"]
    opts.name_IDs = NAME_IDS
    opts.notdef_outline = True
    opts.hinting = False
    opts.glyph_names = False
    opts.recalc_timestamp = False
    # (nothing here reads the axes' names, substitutions or vertical metrics)
    opts.drop_tables += ["STAT", "GSUB", "vhea", "vmtx"]
    sub = subset.Subsetter(options=opts)
    sub.populate(unicodes=UNICODES)
    sub.subset(font)
    out = io.BytesIO()
    font.save(out)
    return out.getvalue()


def check_result(data, fam, style, weight, label):
    from fontTools.ttLib import TTFont
    got = TTFont(io.BytesIO(data))
    if "fvar" in got or "gvar" in got:
        fail("%s: still variable" % label)
    cmap = got.getBestCmap()
    need = [cp for cp in REQUIRED if cp not in cmap]
    if need:
        fail("%s: lacks %s" % (label, ["U+%04X" % c for c in need]))
    missing = [cp for cp in UNICODES if cp not in cmap and not (0x2000 <= cp < 0x2070 and cp not in REQUIRED)]
    if missing:
        print("fonts-classic: %s: the source has no %s (Terminess draws them)"
              % (label, " ".join("U+%04X" % c for c in missing)))
    if got["OS/2"].usWeightClass != weight:
        fail("%s: weight class %d, not %d" % (label, got["OS/2"].usWeightClass, weight))
    if got["name"].getDebugName(1) != fam["family"] or got["name"].getDebugName(2) != style:
        fail("%s: family name %r, not %r" % (label, got["name"].getDebugName(1), fam["family"]))
    if not (got["name"].getDebugName(5) or "").startswith(fam["version"]):
        fail("%s: version %r, not %r" % (label, got["name"].getDebugName(5), fam["version"]))


def write(out_dir, name, data):
    path = os.path.join(out_dir, name)
    fd, tmp = tempfile.mkstemp(dir=out_dir, prefix="." + name, suffix=".tmp")
    with os.fdopen(fd, "wb") as f:
        f.write(data)
    os.chmod(tmp, 0o644)
    os.replace(tmp, path)
    print("fonts-classic: wrote %s (%d bytes, SHA-256 %s)" % (path, len(data), sha256(data)))


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--cache", required=True, help="download folder, outside the repo")
    ap.add_argument("--out", required=True, help="where the fonts and their OFL texts go (assets)")
    ap.add_argument("--verify", action="store_true", help="write nothing; compare with the files in --out")
    args = ap.parse_args()
    try:
        import fontTools
    except ImportError:
        fail("needs fontTools: python3 -m pip install fonttools==%s (in a venv), then "
             "make fonts-subset FONTS_PYTHON=<that venv>/bin/python" % FONTTOOLS)
    print("fonts-classic: fontTools %s" % fontTools.version)
    if fontTools.version != FONTTOOLS:
        print("fonts-classic: warning: the committed fonts were made with fontTools %s; this one may "
              "write other bytes (python3 -m pip install fonttools==%s)" % (FONTTOOLS, FONTTOOLS))
    made = {}
    total = 0
    for fam in FAMILIES:
        src = fetch(args.cache, fam, fam["font"], fam["font_sha256"])
        ofl = fetch(args.cache, fam, "OFL.txt", fam["ofl_sha256"])
        # (a Reserved Font Name would follow the copyright line, before the license's text: the
        # subsets keep the family's name, which only a font without one may)
        head = ofl.split(b"This Font Software is licensed", 1)[0]
        if b"SIL Open Font License, Version 1.1" not in ofl or b"Reserved Font Name" in head:
            fail("%s's OFL.txt is not the OFL 1.1 without a Reserved Font Name" % fam["family"])
        made["%s-OFL-1.1.txt" % fam["stem"]] = ofl
        for style, weight in WEIGHTS:
            name = "%s-%s.ttf" % (fam["stem"], style)
            data = make(src, fam, style, weight)
            check_result(data, fam, style, weight, name)
            made[name] = data
            total += len(data)
    print("fonts-classic: %d fonts, %d bytes in all" % (len(FAMILIES) * len(WEIGHTS), total))
    if args.verify:
        for name, data in made.items():
            path = os.path.join(args.out, name)
            with open(path, "rb") as f:
                if f.read() != data:
                    fail("%s is not what the recipe makes" % path)
            print("fonts-classic: %s matches (%d bytes, SHA-256 %s)" % (path, len(data), sha256(data)))
        return
    for name, data in made.items():
        write(args.out, name, data)


if __name__ == "__main__":
    main()
