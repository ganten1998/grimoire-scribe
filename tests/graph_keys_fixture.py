#!/usr/bin/env python3
"""tests/graph_keys_fixture.py ARGS... ROOT: tests/map_fixture.py's data (the same ARGS), with the Graph's zoom and
fit keys of the 2.0.0-beta.2 keyboard scheme added to its shortcuts.ini: - and =
(code12, code13) zoom out and in, Ctrl+0 fits. Never touches a user's data root."""
import pathlib,subprocess,sys
here=pathlib.Path(__file__).resolve().parent
subprocess.check_call([sys.executable,'-I',str(here/'map_fixture.py')]+sys.argv[1:])
with (pathlib.Path(sys.argv[-1])/'shortcuts.ini').open('a') as f:
    f.write('key.code12=zoom-out\nkey.code13=zoom-in\nkey.Ctrl+0=fit-page\n')
