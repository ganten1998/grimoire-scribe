#!/usr/bin/env python3
"""The PDF import's ownership rules across real app processes (DESIGN.md 3.9, lane L5): one owner per import.

usage: tests/import_check.py <headless binary> <work dir>

1. Grimoire quits in the middle of an import: a detached child finishes it, holding the import lock.
2. A second Grimoire started meanwhile finds the lock held: it queues nothing, a PDF picked by hand is recorded
   as waiting (import/.marks), and nothing is imported twice.
3. When the detached child is done: one notebook, no work folder, the PDF in import/done/, no mark left for it.
4. A later start takes the lock afresh: the pick that waited is imported (read in place, never moved), and
   another PDF dropped meanwhile as usual (a continuation a full queue drops is queued again: rust/tests/
   test_import_queue.rs).
"""
import os
import shutil
import subprocess
import sys
import time
from pathlib import Path

binary = Path(sys.argv[1]).resolve()
out = Path(sys.argv[2]).resolve()
shutil.rmtree(out, ignore_errors=True)
out.mkdir(parents=True)
fix = Path('tests/fixtures/pdf').resolve()
root = out / 'data'
(root / 'import').mkdir(parents=True)
(root / 'Docs').mkdir()
shutil.copy(fix / 'planner' / '2027 Planner.pdf', root / 'import' / 'Planner.pdf')
shutil.copy(fix / 'pick' / 'Planner.pdf', root / 'Docs' / 'Other.pdf')
env = {k: v for k, v in os.environ.items() if not k.startswith('GRIMOIRE_')}
env.update(GRIMOIRE_ROOT=str(root), GRIMOIRE_LANG='en', GRIMOIRE_IMPORT_STABLE='fast',
           GRIMOIRE_PDF_SLICE_PAGES='1', GRIMOIRE_BG_DELAY_MS='500')


def run(name, script, timeout=60):
    path = out / (name + '.qs')
    path.write_text(script + '\nquit\n')
    e = dict(env, GRIMOIRE_SCRIPT=str(path), GRIMOIRE_SHOTS=str(out / 'shots' / name))
    log = out / (name + '.log')
    with log.open('w') as f:
        subprocess.run([str(binary)], env=e, stdout=f, stderr=f, check=True, timeout=timeout)
    return log.read_text()


def notebooks():
    d = root / 'notebooks'
    return sorted(p.name for p in d.iterdir() if not p.name.startswith('.') and (p / 'notebook.ini').exists()) if d.exists() else []


def work_folders():
    d = root / 'notebooks'
    return sorted(p.name for p in d.iterdir() if p.name.startswith('.import-')) if d.exists() else []


# 1. quit in the middle of the import (the headless runner waits for the detached child when it exits, so the
# first run is left going while the second starts)
path1 = out / 'first.qs'
path1.write_text('waitfor notice has Importing: page\nquit\n')
e1 = dict(env, GRIMOIRE_SCRIPT=str(path1), GRIMOIRE_SHOTS=str(out / 'shots' / 'first'))
log1f = (out / 'first.log').open('w')
first = subprocess.Popen([str(binary)], env=e1, stdout=log1f, stderr=log1f)
for _ in range(200):
    if 'left to a detached child' in (out / 'first.log').read_text():
        break
    time.sleep(0.05)
else:
    raise AssertionError('Grimoire never handed the import to a detached child')
assert notebooks() == [] and work_folders(), 'the import was not in the middle when Grimoire quit'
# 2. a second Grimoire while the detached child imports (the data root's parent is the storage root)
log2 = run('second', 'pdfimport %s/Docs/Other.pdf\nwait 200' % root.name)
assert 'an import from an earlier run is still going' in log2, log2
assert 'Waiting' in log2, log2
marks = list((root / 'import' / '.marks').glob('*'))
assert any('state=waiting' in m.read_text() for m in marks), 'the pick was not recorded as waiting'
assert 'bg: PDF import of' not in log2, 'the second run started an import beside the detached one: ' + log2
first.wait(timeout=60)
# 3. the detached child finishes: one notebook
for _ in range(120):
    if len(notebooks()) == 1 and not work_folders():
        break
    time.sleep(0.5)
assert len(notebooks()) == 1, notebooks()
assert not work_folders(), work_folders()
done = list((root / 'import' / 'done').glob('Planner-*.pdf'))
assert len(done) == 1 and not (root / 'import' / 'Planner.pdf').exists(), 'the PDF did not move to done/'
# 4. the lock is free again: the next start imports the pick that waited, and a file dropped meanwhile
env['GRIMOIRE_BG_DELAY_MS'] = '0'
shutil.copy(fix / 'twice' / 'First.pdf', root / 'import' / 'Cut.pdf')
log3 = run('cut', 'waitbg\nwait 200')
assert 'pick(s) that waited for the lock queued' in log3, log3
assert len(notebooks()) == 3, notebooks()
assert (root / 'Docs' / 'Other.pdf').exists(), 'the picked file moved'
marks = list((root / 'import' / '.marks').glob('*')) if (root / 'import' / '.marks').exists() else []
assert not any('state=waiting' in m.read_text() for m in marks), 'a waiting mark is left'
print('IMPORT CHECK PASSED')
