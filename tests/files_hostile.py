#!/usr/bin/env python3
# Plants what a fixture tree cannot hold (git keeps no FIFO, and a link into /etc is not a fixture) in the storage
# root a "# storage:" scenario runs on: links to /etc, to "..", to a PDF and to Grimoire's notebooks, a FIFO, a name
# with ':' and a name that is not UTF-8. The file picker must show the plain files and folders and none of these
# as a link, and never enter or open a link (tests/scenarios/z_files_2_hostile.qs, rust/tests/test_browser.rs).
# Usage: files_hostile.py STORAGE_ROOT
import os
import shutil
import sys

root = sys.argv[1]
doc = os.path.join(root, 'documents')
os.makedirs(os.path.join(root, 'grimoire', 'notebooks'), exist_ok=True)
os.symlink('/etc', os.path.join(doc, 'etc'))
os.symlink('..', os.path.join(doc, 'up'))
os.symlink('Planner.pdf', os.path.join(doc, 'Alias.pdf'))
os.symlink(os.path.join(root, 'grimoire', 'notebooks'), os.path.join(root, 'Downloads', 'nb'))
os.symlink('/etc/hosts', os.path.join(doc, 'Hosts.pdf'))
os.mkfifo(os.path.join(doc, 'pipe.pdf'))
shutil.copy(os.path.join(doc, 'Planner.pdf'), os.path.join(doc, 'time 10:30.pdf'))
try:
    shutil.copy(os.path.join(doc, 'Planner.pdf'), os.path.join(os.fsencode(doc), b'caf\xe9.pdf'))
except OSError:
    pass  # (macOS's file system takes UTF-8 names only: the name is planted where the file system lets it)
# a folder that is only a link to a folder of pictures
os.symlink(os.path.join(doc, 'Scans'), os.path.join(doc, 'LinkedScans'))
# a link inside the folder of pictures
os.symlink('/etc/hosts', os.path.join(doc, 'Scans', 'page-3.png'))
