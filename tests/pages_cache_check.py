#!/usr/bin/env python3
"""Verify persistent thumbnail reuse across real app processes, then update only an edited page."""
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import time

binary = Path(sys.argv[1]).resolve()
out = Path(sys.argv[2]).resolve()
root = out / 'root'
shutil.rmtree(out, ignore_errors=True)
out.mkdir(parents=True)
subprocess.run([sys.executable, 'tests/page_management_fixture.py', str(root)], check=True)
with (root / 'settings.ini').open('a') as stream:
    stream.write('\nmirror_pdf=0\nhttp_port=8892\n')
env = {k: v for k, v in os.environ.items() if not k.startswith('GRIMOIRE_')}
env.update(GRIMOIRE_ROOT=str(root), GRIMOIRE_LANG='en', GRIMOIRE_CHECK_FIT='1')


def run(name, script):
    path = out / (name + '.qs')
    path.write_text('wait 300\ntap library folder.Void\ntap library notebook.A\n' + script + '\nquit\n')
    env.update(GRIMOIRE_SCRIPT=str(path), GRIMOIRE_SHOTS=str(out / 'shots' / name))
    started = time.monotonic()
    with (out / (name + '.log')).open('w') as log:
        subprocess.run([str(binary)], env=env, stdout=log, stderr=log, check=True, timeout=45)
    return round(time.monotonic() - started, 3)


def stamps():
    return {p.name: (p.stat().st_ino, p.stat().st_mtime_ns, hashlib.sha256(p.read_bytes()).hexdigest())
            for p in (root / 'notebooks/A/.page-previews').iterdir() if p.name.endswith(('.pgm', '.stamp'))}


warm = run('warm', 'pentap panel pages\nwaitbg 30000\nexpect editor.pages.cached is 9')
before = stamps()
assert len(before) == 18, 'Expected nine persistent image/stamp pairs'
cold = run('cold', 'pentap panel pages\nexpect editor.pages.cached is 9\nexpect bg.busy is 0')
assert stamps() == before, 'A new process rebuilt valid thumbnails'
# A page turn saves the edited page and writes no preview: the background job makes it later.
saved = run('saved', 'pen line 600 900 1200 920 25 0.6 0.6\npen leave\npentap panel next\nexpect editor.page is 2\n'
            'pentap panel previous\nexpect editor.page is 1')
assert stamps() == before, 'Saving a page wrote its preview in the UI process'
# Pages opened on the page just edited shows its tile at once, from the page in memory.
seeded = run('seeded', 'pen line 600 900 1200 980 25 0.6 0.6\npen leave\npentap panel pages\nexpect editor.pages.cached is 9')
edited = run('edited', 'pen line 600 900 1200 920 25 0.6 0.6\npen leave\npentap panel next\n'
             'pentap panel pages\nwaitbg 30000\nwait 600\nexpect editor.pages.cached is 9')
after = stamps()
assert after['p1.pgm'][2] != before['p1.pgm'][2], 'Edited-page preview did not change'
assert all(after[name] == value for name, value in before.items() if not name.startswith('p1.')), 'Unedited previews were rewritten'
result = {'warm_process_seconds': warm, 'cold_process_seconds': cold, 'edit_process_seconds': edited,
          'save_process_seconds': saved, 'seeded_process_seconds': seeded, 'save_writes_no_preview': True,
          'cached_pages': 9, 'cold_open_without_render_job': True, 'only_edited_page_updated': True}
(out / 'result.json').write_text(json.dumps(result, indent=2) + '\n')
print('PAGES CACHE CHECK PASSED', json.dumps(result), flush=True)
