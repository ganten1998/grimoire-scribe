#!/usr/bin/env python3
"""Notebooks for the page previews scenarios (z_previews_*): "previews" has twelve pages, the first a dense
one (1,000 strokes, one in ten an erase stroke: over the 512 KB that counts as dense, store_page_dense) and
the rest light (a few strokes each); it opens on page 2. "other" has ten light pages and a dense ninth, for
the destination picker (Copy to notebook..., Link to...'s other notebooks). "dense" opens on its dense first
page. Deterministic; never touches a user's data root."""
import math
import pathlib
import struct
import sys

root = pathlib.Path(sys.argv[1])
W, H = 1860, 2480
seed = 0xD0D0


def rnd():
    global seed
    seed = (seed * 1664525 + 1013904223) & 0xFFFFFFFF
    return seed >> 8


def frand(lo, hi):
    return lo + (hi - lo) * (rnd() % 10001) / 10000.0


def stroke(tool, size, x, y, n, step):
    params = [size[0], size[1], 0.8, 1.0, 0.0, 1.0, 1.0, 0.0, 70.0, 0.0, 0.0, 0.5, 0.5]
    s = rnd() & 0xFFFFFF
    ang = frand(0, 6.28)
    pts = bytearray()
    for i in range(n):
        pts += struct.pack('<ffHhhI', x, y, int(frand(0.3, 1.0) * 65535), 0, 0, i * 6)
        ang += frand(-0.4, 0.4)
        x += math.cos(ang) * step
        y += math.sin(ang) * step
        x = min(max(x, -100), W + 100)
        y = min(max(y, -100), H + 100)
    return struct.pack('<B3sI13fI', tool, b'\0\0\0', s, *params, n) + pts


def dense_page():
    out = bytearray(struct.pack('<4sHHIII', b'QPG1', 1, 0, W, H, 1000))
    for _ in range(1000):
        erase = rnd() % 10 == 0
        tool = 3 if erase else [2, 0, 6, 7][rnd() % 4]
        out += stroke(tool, (2.0, 2.0) if erase else (0.3, 0.6), frand(0, W), frand(0, H), 10 + rnd() % 120, 4)
    return bytes(out)


def light_page(k):
    n = 2 + k % 3
    out = bytearray(struct.pack('<4sHHIII', b'QPG1', 1, 0, W, H, n))
    for j in range(n):
        out += stroke(2, (0.3, 0.6), 200 + 300 * j, 300 + 150 * k % 1800, 60, 6)
    return bytes(out)


def notebook(ident, name, count, dense, last):
    book = root / 'notebooks' / ident
    book.mkdir(parents=True, exist_ok=True)
    (book / 'notebook.ini').write_text(
        f'# Grimoire notebook\nname={name}\nfolder=\npaper=0\ncreated=1\nmodified=1\nlast_page={last}\n'
        f'next_page_id={count + 1}\npage_w={W}\npage_h={H}\npages=' + ','.join(str(i) for i in range(1, count + 1)) + '\n')
    for i in range(1, count + 1):
        (book / f'p{i}.qpg').write_bytes(dense_page() if i == dense else light_page(i))


notebook('previews', 'Previews', 12, 1, 1)
notebook('other', 'Other', 10, 9, 0)
notebook('dense', 'Dense', 2, 1, 0)
with (root / 'settings.ini').open('a') as f:
    f.write('\nmirror_pdf=0\nstatus_mode=1\n')
