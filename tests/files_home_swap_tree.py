#!/usr/bin/env python3
# Plants the home tree (tests/files_home_tree.py) and with it a link out of the storage beside documents/Sorted
# (documents/Root, which leads to /), for files_34_home_swap: the home folder swapped for that link while Grimoire
# runs. The link is never followed. Usage: files_home_swap_tree.py STORAGE_ROOT
import os
import runpy
import sys

root = sys.argv[1]
runpy.run_path(os.path.join(os.path.dirname(os.path.abspath(__file__)), 'files_home_tree.py'), run_name='__main__')
os.symlink('/', os.path.join(root, 'documents', 'Root'))
