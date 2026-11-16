#!/usr/bin/env python3
# Plants the tree the Files scenarios of rename, delete and restore (tests/scenarios/files_8_rename.qs to
# files_14_es_fit_classic.qs, issue #144) work on, in the storage root a "# storage:" scenario runs on: three
# notebooks (Alpha and Beta at the Library's top, Gamma in School); two templates and a photo of known sizes; two
# folders of a picture each in photos (the folder card a writer swaps); and a trash holding a notebook named Beta
# (store_delete's name, `<id>-<time>`: its name is taken at the top by the live Beta) and a page of Alpha
# (`<id>-p<page>-<time>`). Every size is a whole number of KB, so the rows say them exactly: the trashed notebook
# 5 KB, the page 2 KB (the trash 7 KB), the photo 3 KB. Usage: files_trash_tree.py STORAGE_ROOT
import os
import sys

root = sys.argv[1]
g = os.path.join(root, 'grimoire')
T = 1790000000  # 2026-09-21: every time below is before it, but the trashed Beta's own, which is the newest


def ini(name, folder, n):
    return (f'name={name}\nfolder={folder}\npaper=0\ncreated={T}\nmodified={T + n}\nlast_page=0\n'
            f'next_page_id=2\npages=1\npage_w=1860\npage_h=2480\n')


def notebook(n, name, folder, d=None, total=None):
    d = d or os.path.join(g, 'notebooks', f'20261101-000000-000{n}')
    os.makedirs(d, exist_ok=True)
    text = ini(name, folder, n)
    with open(os.path.join(d, 'notebook.ini'), 'w') as f:
        f.write(text)
    if total:
        # (its one page file pads the folder to exactly `total` bytes)
        with open(os.path.join(d, 'p1.qpg'), 'wb') as f:
            f.write(b'q' * (total - len(text.encode())))


def put(path, size, age):
    p = os.path.join(root, path)
    os.makedirs(os.path.dirname(p), exist_ok=True)
    with open(p, 'wb') as f:
        f.write(b'x' * size)
    os.utime(p, (T - age, T - age))


notebook(1, 'Alpha', '')
notebook(2, 'Beta', '')
notebook(3, 'Gamma', 'School')
for d in ['import', 'templates', 'photos', 'trash']:
    os.makedirs(os.path.join(g, d), exist_ok=True)
put('grimoire/templates/Grid.png', 1024, 300)
put('grimoire/templates/Lines.png', 2048, 200)
put('grimoire/photos/beach.jpg', 3072, 100)
put('grimoire/photos/Trip/p1.png', 100, 400)
put('grimoire/photos/Other/q1.png', 100, 500)
notebook(4, 'Beta', '', os.path.join(g, 'trash', f'20261101-000000-0004-{T - 4000}'), 5120)
put(f'grimoire/trash/20261101-000000-0001-p3-{T - 5000}', 2048, 5000)
