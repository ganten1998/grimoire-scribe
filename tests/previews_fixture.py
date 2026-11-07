#!/usr/bin/env python3
"""Notebooks for the page previews scenarios (z_previews_*): "previews" has twelve pages, the first a dense
one (1,000 strokes, one in ten an erase stroke: over the 512 KB that counts as dense, store_page_dense) and
the rest light (a few strokes each); it opens on page 2. "other" has ten light pages and a dense ninth, for
the destination picker (Copy to notebook..., Link to...'s other notebooks). "dense" opens on its dense first
page. With "slow" before the root (# generate: tests/previews_fixture.py slow), "slow" too: three pages, the
first a slow one under 512 KB (300 strokes: over the 256 KB that says "Loading page...", store_page_slow, and
no dense page) and two light ones; it opens on page 2. With "linked", "dense"'s first page has a link to its second,
with an area (z_keep_ghost_5_redo). With "ghosts", what 2.0.0 and 2.0.1 left of a cleared page (z_keep_ghost_7 to
_10): an old picture as their drawing kept of the page (their tag, with no "|k2"), as its Pages preview and, for page 1,
as the Library card, in four notebooks of two pages: "ghost" (page 1 cleared, its drawing current; opens on page 2),
"stale" (page 1 written on since, a light page: the drawing of the file as it was, the preview and the card of the
old ink and the new, the preview current; opens on page 1), "stale2" (the same with page 1 dense and its preview not
current, as the previews job finds it; opens on page 2) and "barghost" (page 1 links to page 2, which is cleared, its
drawing current; opens on page 1). Deterministic; never touches a user's data root."""
import math
import pathlib
import struct
import sys

root = pathlib.Path(sys.argv[-1])
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


def dense_page(strokes=1000):
    out = bytearray(struct.pack('<4sHHIII', b'QPG1', 1, 0, W, H, strokes))
    for _ in range(strokes):
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


def notebook(ident, name, count, dense, last, strokes=1000):
    book = root / 'notebooks' / ident
    book.mkdir(parents=True, exist_ok=True)
    (book / 'notebook.ini').write_text(
        f'# Grimoire notebook\nname={name}\nfolder=\npaper=0\ncreated=1\nmodified=1\nlast_page={last}\n'
        f'next_page_id={count + 1}\npage_w={W}\npage_h={H}\npages=' + ','.join(str(i) for i in range(1, count + 1)) + '\n')
    for i in range(1, count + 1):
        (book / f'p{i}.qpg').write_bytes(dense_page(strokes) if i == dense else light_page(i))


notebook('previews', 'Previews', 12, 1, 1)
notebook('other', 'Other', 10, 9, 0)
notebook('dense', 'Dense', 2, 1, 0)
if 'slow' in sys.argv[1:-1]:
    notebook('slow', 'Slow', 3, 1, 1, 300)
if 'linked' in sys.argv[1:-1]:
    (root / 'notebooks' / 'dense' / 'links.ini').write_text('# Grimoire links\nlink.1=00000001,p2\narea.1=00000001,300,500,400,200\n')


def pgm(path, w, h, ink):
    """A P5 picture of w x h: the old ink, one pixel in seven dark, and with `ink` a band more (the new)."""
    px = bytearray(40 if i % 7 == 0 else 255 for i in range(w * h))
    if ink:
        px[h // 2 * w + w // 4:h // 2 * w + w // 2] = b'\0' * (w // 4)
    path.write_bytes(b'P5\n%d %d\n255\n' % (w, h) + bytes(px))


def stamp_of(source):
    """The page file's identity as store.rs preview_stamp writes it (no previews generation file)."""
    st = source.stat()
    return f'{st.st_ino}:{st.st_size}:{st.st_mtime_ns // 10**9}:{st.st_mtime_ns % 10**9}:{st.st_ctime_ns % 10**9}:'


def ghost(ident, name, last, pid, page, current, preview_current, ink):
    """Notebook ident (two pages) whose page pid holds `page` (bytes, or None for a cleared page), with 2.0.1's
    leftovers: its kept drawing current for the page file or not, its preview (current or not) and, for page 1, the
    Library card, all of the old picture (and with `ink` of the new ink over it)."""
    notebook(ident, name, 2, 0, last)
    book = root / 'notebooks' / ident
    ini = book / 'notebook.ini'
    ini.write_text(ini.read_text().replace('modified=1', 'modified=2'))  # (first in the Library: one screenful)
    source = book / f'p{pid}.qpg'
    source.write_bytes(page if page is not None else struct.pack('<4sHHIII', b'QPG1', 1, 0, W, H, 0))
    now = stamp_of(source)
    before = '1:20:1:0:0:'  # (a file of the page as it was)
    cache = book / '.page-previews'
    cache.mkdir()
    (cache / f'p{pid}.full').write_bytes(b'\x7f\xff')  # (never read)
    (cache / f'p{pid}.full.stamp').write_text(f'{now if current else before}|{W}x{H}|9d3ca8ad67886c79')
    pgm(cache / f'p{pid}.pgm', 300, 400, ink)
    (cache / f'p{pid}.stamp').write_text(now if preview_current else before)
    if pid == 1:
        pgm(book / 'thumb.pgm', W // 3, H // 3, ink)


if 'ghosts' in sys.argv[1:-1]:
    ghost('ghost', 'Ghost', 1, 1, None, True, True, False)
    ghost('stale', 'Stale', 0, 1, light_page(1), False, True, True)
    ghost('stale2', 'Stale 2', 1, 1, dense_page(), False, False, True)
    ghost('barghost', 'Bar ghost', 0, 2, None, True, True, False)
    (root / 'notebooks' / 'barghost' / 'links.ini').write_text('# Grimoire links\nlink.1=00000001,p2\narea.1=00000001,300,500,400,200\n')
with (root / 'settings.ini').open('a') as f:
    f.write('\nmirror_pdf=0\nstatus_mode=1\n')
