#!/usr/bin/env python3
"""Wi-Fi controls, with/without an IP, English/Spanish, both styles and orientations."""
import os
from pathlib import Path
import shutil
import subprocess
repo=Path(__file__).resolve().parents[1]
build=Path(os.environ.get('BUILD','build')).resolve()
for lang in ('en','es'):
    for style in ('gothic','classic'):
        for online in (False,True):
            name=f'wifi-{lang}-{style}-{int(online)}';root=build/'scen/l10n'/name;shots=build/'shots/l10n'/name
            shutil.rmtree(root,ignore_errors=True);shutil.rmtree(shots,ignore_errors=True);root.mkdir(parents=True)
            (root/'settings.ini').write_text(f'theme={style}\nhttp_port={os.environ.get("GRIMOIRE_TEST_PORT","8895")}\n')
            env={k:v for k,v in os.environ.items() if not k.startswith('GRIMOIRE_')}
            env.update(GRIMOIRE_ROOT=str(root),GRIMOIRE_SCRIPT=str(repo/'tests/scenarios/z_wifi.qs'),GRIMOIRE_SHOTS=str(shots),
                GRIMOIRE_LANG=lang,GRIMOIRE_CHECK_FIT='1',GRIMOIRE_WIFI_SCRIPT=str(repo/'tests/fixtures/wifi/helper.sh'),
                GRIMOIRE_SIM_ADDRS='wlan0=192.168.0.20' if online else '')
            print('==',name,flush=True)
            subprocess.run([str(build/'host/grimoire-headless')],env=env,cwd=repo,check=True)
print('WIFI LANGUAGE CHECK PASSED',flush=True)
