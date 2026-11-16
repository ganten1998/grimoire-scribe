#!/usr/bin/env python3
# KOReader's version 2 request (2.2, docs/FORMAT.md, Whole books): the plugin names the PDF it has open by its
# absolute path, which a fixture cannot hold, as it depends on where the run's root is. This writes the request into
# the data root of a scenario's run as the plugin does, for the one PDF in a folder the scenario copied in
# (tests/scenarios/y_koreader_pdf_*.qs).
# Usage: book_v2_request.py --dir DIR --book KEY --page N --pages M [--data DATA] [--title TITLE] ROOT
#   DIR: the PDF's folder, from ROOT; DATA: Grimoire's folder, from ROOT ("." by default, "grimoire" for a
#   "# storage:" run); TITLE: the book's title (by default the PDF's name without .pdf)
import argparse
import os
import time

p = argparse.ArgumentParser()
p.add_argument('--dir', required=True)
p.add_argument('--book', required=True)
p.add_argument('--page', type=int, required=True)
p.add_argument('--pages', type=int, required=True)
p.add_argument('--data', default='.')
p.add_argument('--title')
p.add_argument('root')
a = p.parse_args()
folder = os.path.abspath(os.path.join(a.root, a.dir))
pdfs = sorted(n for n in os.listdir(folder) if n.lower().endswith('.pdf'))
if len(pdfs) != 1:
    raise SystemExit(f'book_v2_request.py: {len(pdfs)} PDFs in {folder}, wanted 1')
title = a.title or os.path.splitext(pdfs[0])[0]
book = os.path.join(a.root, a.data, 'import', '.book')
os.makedirs(book, exist_ok=True)
lines = [
    "# Grimoire: a whole book to open, from KOReader's Grimoire plugin (docs/FORMAT.md)",
    'version=2',
    f'book={a.book}',
    f'file={os.path.join(folder, pdfs[0])}',
    f'title={title}',
    f'pages={a.pages}',
    f'page={a.page}',
    'kind=paged',
    f'time={int(time.time())}',
]
part = os.path.join(book, 'request.ini.part')
with open(part, 'w', encoding='utf-8') as f:
    f.write('\n'.join(lines) + '\n')
os.replace(part, os.path.join(book, 'request.ini'))
