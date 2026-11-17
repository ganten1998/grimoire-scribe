#!/usr/bin/env python3
# Plants the tree the home scenarios (tests/scenarios/files_*_home_*.qs, issue #160) work on, in the storage root a
# "# storage:" scenario runs on: the tree of tests/files_place_tree.py (Alpha and Beta at the Library's top, Gamma
# in School; documents/Sorted with Sub/Deep in it; Grimoire's places), and with it a Library folder inside a
# folder (School/2026, holding Delta), a folder of one picture inside Grimoire's photos folder (Scans, which Files
# can rename and delete; not in import, where a folder of pictures becomes a notebook as Grimoire starts), and a
# link where a folder would be (documents/Linked, which leads to Sorted and is never followed).
# Usage: files_home_tree.py STORAGE_ROOT
import os
import runpy
import sys

root = sys.argv[1]
place = runpy.run_path(os.path.join(os.path.dirname(os.path.abspath(__file__)), 'files_place_tree.py'))
place['notebook'](4, 'Delta', 'School/2026')
place['put']('grimoire/photos/Scans/p1.png', 1024, 250)
os.symlink('Sorted', os.path.join(root, 'documents', 'Linked'))
