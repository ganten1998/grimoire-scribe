#!/usr/bin/env python3
"""The keyboard scheme's scenarios: a data root with BOOKS blank
notebooks k000, k001, ... ("Book 0", ...) in the folder Shelf, and no shortcuts.ini (so the starter map is
read). Never touches a user's data root.

tests/keys_fixture.py [--books N] ROOT"""
import pathlib, struct, sys

blank = struct.pack('<4sHHIII', b'QPG1', 1, 4, 1860, 2480, 0)
args = sys.argv[1:]
books = 13
if args and args[0] == '--books':
    books = int(args[1])
    args = args[2:]
root = pathlib.Path(args[-1])
for k in range(books):
    book = root / 'notebooks' / f'k{k:03d}'
    book.mkdir(parents=True, exist_ok=True)
    (book / 'notebook.ini').write_text('# Grimoire notebook\nname=Book %d\nfolder=Shelf\npaper=4\ncreated=1\n'
                                       'modified=%d\nlast_page=0\nnext_page_id=2\npage_w=1860\npage_h=2480\npages=1\n'
                                       % (k, 1000 + k))
    (book / 'p1.qpg').write_bytes(blank)
(root / 'folders.ini').write_text('Shelf\n')
