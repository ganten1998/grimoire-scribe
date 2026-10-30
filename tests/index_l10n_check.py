#!/usr/bin/env python3
"""Exercise the unified Index and cross-notebook selector in both languages and styles."""
from pathlib import Path
import argparse
import os
import shutil
import subprocess
import sys

repo=Path(__file__).resolve().parents[1]
parser=argparse.ArgumentParser()
parser.add_argument('--binary',type=Path)
args=parser.parse_args()
build=Path(os.environ.get('BUILD','build')).resolve()
binary=args.binary.resolve() if args.binary else build/'host/grimoire-headless'
for lang in ('en','es'):
    for style in ('gothic','classic'):
        for case in ('z_index_unified','z_notebook_links','z_page_management'):
            name=f'index-{case}-{lang}-{style}'
            root=build/'scen/l10n'/name
            shots=build/'shots/l10n'/name
            for path in (root,shots):
                if path.exists():shutil.rmtree(path)
                path.mkdir(parents=True)
            subprocess.run([sys.executable,str(repo/('tests/page_management_fixture.py' if case=='z_page_management' else 'tests/notebook_links_fixture.py')),str(root)],cwd=repo,check=True)
            with (root/'settings.ini').open('a') as f:
                f.write(f'\ntheme={style}\nhttp_port={os.environ.get("GRIMOIRE_TEST_PORT","8896")}\n')
            env=dict(os.environ)
            for key in tuple(env):
                if key.startswith('GRIMOIRE_SIM_') or key in ('GRIMOIRE_HOST','GRIMOIRE_ANNOTATE','GRIMOIRE_BOOK'):
                    env.pop(key)
            env.update(GRIMOIRE_LANG=lang,GRIMOIRE_CHECK_FIT='1',GRIMOIRE_ROOT=str(root),
                       GRIMOIRE_SCRIPT=str(repo/'tests/scenarios'/f'{case}.qs'),GRIMOIRE_SHOTS=str(shots))
            print('== l10n',name,flush=True)
            subprocess.run([str(binary)],cwd=repo,env=env,check=True)
print('INDEX LANGUAGE CHECK PASSED',flush=True)
