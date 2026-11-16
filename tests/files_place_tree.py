#!/usr/bin/env python3
# Plants the tree the Files scenarios (tests/scenarios/files_*.qs) browse, in the storage root a "# storage:" scenario
# runs on: three notebooks (two at the Library's top, one in a folder), Grimoire's places (import, templates, photos,
# fonts, PDF copies, a trashed notebook), the Kindle's fonts folder, Downloads, a top folder that is no place (Notes),
# and documents/Sorted, whose files have known sizes and times for Sort, one file Grimoire cannot use and a link for
# Show all, and a folder two levels deep for the path bar. Usage: files_place_tree.py STORAGE_ROOT
import os
import sys

root = sys.argv[1]
g = os.path.join(root, 'grimoire')
T = 1790000000  # 2026-09-21: every time below is before it


def notebook(n, name, folder, d=None):
    d = d or os.path.join(g, 'notebooks', f'20261101-000000-000{n}')
    os.makedirs(d, exist_ok=True)
    with open(os.path.join(d, 'notebook.ini'), 'w') as f:
        f.write(f'name={name}\nfolder={folder}\npaper=0\ncreated={T}\nmodified={T + n}\nlast_page=0\n'
                f'next_page_id=2\npages=1\npage_w=1860\npage_h=2480\n')


def put(path, size, age):
    p = os.path.join(root, path)
    os.makedirs(os.path.dirname(p), exist_ok=True)
    with open(p, 'wb') as f:
        f.write(b'%PDF-1.4\n' if path.endswith('.pdf') else b'')
        f.write(b'x' * (size - f.tell()))
    os.utime(p, (T - age, T - age))


notebook(1, 'Alpha', '')
notebook(2, 'Beta', '')
notebook(3, 'Gamma', 'School')
for d in ['import', 'templates', 'photos', 'fonts', 'PDFs/School', 'trash']:
    os.makedirs(os.path.join(g, d), exist_ok=True)
notebook(9, 'Gone', '', os.path.join(g, 'trash', f'20261101-000000-0009-{T - 5000}'))
put('grimoire/PDFs/School/Gamma.pdf', 2000, 400)
put('fonts/Kindle Serif.ttf', 700, 900)
put('Downloads/Daily.pdf', 1200, 800)
os.makedirs(os.path.join(root, 'Notes'), exist_ok=True)  # a top folder that is no place (Go to lists it)
# Sort: by name Sub, a.pdf, b.pdf, c.png, d.ttf; by newest c.png, b.pdf, d.ttf, a.pdf; by size b.pdf, c.png,
# a.pdf, d.ttf (folders first in each). Show all adds e.txt (newest and largest of all) and the link f.pdf.
put('documents/Sorted/a.pdf', 1000, 300)
put('documents/Sorted/b.pdf', 3000, 100)
put('documents/Sorted/c.png', 2000, 50)
put('documents/Sorted/d.ttf', 500, 200)
put('documents/Sorted/e.txt', 4000, 10)
put('documents/Sorted/Sub/Deep/inside.pdf', 900, 600)
os.utime(os.path.join(root, 'documents/Sorted/Sub'), (T - 5, T - 5))
os.symlink('a.pdf', os.path.join(root, 'documents/Sorted/f.pdf'))
