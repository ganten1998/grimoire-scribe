#!/usr/bin/env python3
# Plants what tests/scenarios/files_9_copy_refused.qs asks a copy of, in the storage root a "# storage:" scenario
# runs on: documents/Big/huge.png, a file one byte over what the template job takes of a template or a photo (16 MB;
# a sparse file: it uses no disk, and no copy of it is ever made). Usage: files_copy_refused.py STORAGE_ROOT
import os
import sys

root = sys.argv[1]
T = 1790000000  # 2026-09-21


def sparse(path, size):
    p = os.path.join(root, path)
    os.makedirs(os.path.dirname(p), exist_ok=True)
    with open(p, 'wb') as f:
        f.truncate(size)
    os.utime(p, (T, T))


sparse('documents/Big/huge.png', (16 << 20) + 1)
