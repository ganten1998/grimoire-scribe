#!/usr/bin/env python3
"""Two notebooks for the lasso's cut, paste and read-only scenario (z_lasso_4_cut_paste). Never touches a
user's data root.

  lasso-big (1986 x 2648): p1 a ballpoint word (two strokes), p2 blank, p3 read-only (a critical chunk
  this version does not know after its strokes) holding a line nearly as wide as the page.
  lasso-small (1860 x 2480): p1 blank, where the wide line must be scaled to fit.
"""
import pathlib, struct, sys

root = pathlib.Path(sys.argv[1])


def book(ident, name, w, h, pages, modified):
    b = root / 'notebooks' / ident
    b.mkdir(parents=True, exist_ok=True)
    ids = ','.join(str(i) for i in range(1, len(pages) + 1))
    (b / 'notebook.ini').write_text(
        f'# Grimoire notebook\nname={name}\nfolder=\npaper=0\ncreated=1\nmodified={modified}\nlast_page=0\n'
        f'next_page_id={len(pages) + 1}\npage_w={w}\npage_h={h}\npages={ids}\n')
    for i, (strokes, tail) in enumerate(pages, 1):
        out = bytearray(struct.pack('<4sHHIII', b'QPG1', 1, 0, w, h, len(strokes)))
        for seed, pts in strokes:
            # a ballpoint (tool 2), 0.4 mm
            params = [0.4, 0.4, 0.8, 1.0, 0.0, 1.0, 1.0, 0.0, 70.0, 0.0, 0.0, 0.5, 0.5]
            out += struct.pack('<B3sI13fI', 2, b'\0\0\0', seed, *params, len(pts))
            for k, (x, y) in enumerate(pts):
                out += struct.pack('<ffHhhI', x, y, 40000, 0, 0, k * 8)
        out += tail
        (b / f'p{i}.qpg').write_bytes(bytes(out))


def line(x0, y0, x1, y1, n=40):
    return [(x0 + (x1 - x0) * k / (n - 1), y0 + (y1 - y0) * k / (n - 1)) for k in range(n)]


word = [(0x101, line(500, 700, 800, 760)), (0x102, line(520, 820, 820, 800))]
wide = [(0x201, line(40, 1500, 1940, 1560, 120))]
unknown_critical = b'ZZZZ' + struct.pack('<I', 0)
book('lasso-big', 'Big', 1986, 2648, [(word, b''), ([], b''), (wide, unknown_critical)], 2)
book('lasso-small', 'Small', 1860, 2480, [([], b'')], 1)
with (root / 'settings.ini').open('a') as f:
    f.write('\nmirror_pdf=0\nstatus_mode=1\n')
