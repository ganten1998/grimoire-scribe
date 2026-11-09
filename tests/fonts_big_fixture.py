#!/usr/bin/env python3
# Plants what a fixture tree cannot hold in a data root's fonts folder: a font file of 30 MB, sparse (no byte of it
# is ever read: Grimoire refuses a font over 24 MB by its size alone, "Too big (over 24 MB)."). For
# tests/l10n/fonts-own.qs and tests/scenarios/y_fonts_own_3_notes.qs.
# Usage: fonts_big_fixture.py DATA_ROOT
import os
import sys

fonts = os.path.join(sys.argv[1], 'fonts')
os.makedirs(fonts, exist_ok=True)
with open(os.path.join(fonts, 'Huge.ttf'), 'wb') as f:
    f.truncate(30 << 20)
