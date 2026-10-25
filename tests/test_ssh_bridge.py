import json
import os
from pathlib import Path
import signal
import stat
import subprocess
import tempfile
import unittest

SCRIPT = Path(__file__).resolve().parents[1] / 'kindle/lab/ssh-bridge.sh'


class BridgeTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.state = self.root / 'state'
        self.proc = self.root / 'proc'
        self.proc.mkdir()
        self.ko = self.root / 'koreader'
        auth = self.ko / 'settings/SSH'
        auth.mkdir(parents=True)
        (self.ko / 'reader.lua').write_text('-- fixture')
        (auth / 'authorized_keys').write_text('public-test-key\n')
        (auth / 'dropbear_ed25519_host_key').write_bytes(b'private-test-host-identity')
        donor = self.ko / 'dropbear'
        donor.write_text('''#!/bin/sh
if [ "$1" = -h ]; then echo '-D directory -s disable passwords'; exit 0; fi
printf '%s\\n' "$@" > "$KINDLELAB_SSH_STATE/args"
(trap 'exit 0' TERM; while :; do sleep 0.1; done) </dev/null >/dev/null 2>&1 &
p=$!
mkdir -p "$KINDLELAB_PROC/$p"
ln -s "$KINDLELAB_SSH_STATE/dropbear-lab" "$KINDLELAB_PROC/$p/exe"
echo "$p" > "$KINDLELAB_SSH_STATE/listener.pid"
''')
        donor.chmod(0o700)
        firewall = self.root / 'iptables'
        firewall.write_text('''#!/usr/bin/env python3
import json, os, pathlib, sys
p=pathlib.Path(os.environ['RULES'])
rules=json.loads(p.read_text()) if p.exists() else []
op, chain, *args = sys.argv[1:]
if op == '-I': args=args[1:]
rule=[chain,*args]
if op == '-C': sys.exit(0 if rule in rules else 1)
if op == '-I':
    if os.environ.get('FAIL_FIREWALL') and chain == 'OUTPUT': sys.exit(1)
    rules.append(rule)
elif op == '-D': rules.remove(rule)
else: sys.exit(2)
p.write_text(json.dumps(rules))
''')
        firewall.chmod(0o700)
        self.env = dict(os.environ, KINDLELAB_SSH_STATE=str(self.state), KINDLELAB_PROC=str(self.proc),
                        KINDLELAB_IPTABLES=str(firewall), RULES=str(self.root / 'rules.json'))
        self.addCleanup(self.stop_children)

    def stop_children(self):
        for path in self.proc.iterdir():
            try:
                os.kill(int(path.name), signal.SIGTERM)
            except ProcessLookupError:
                pass

    def run_bridge(self, *args, ok=True):
        run = subprocess.run(['sh', str(SCRIPT), *map(str, args)], env=self.env,
                             capture_output=True, text=True, timeout=15)
        if ok:
            self.assertEqual(run.returncode, 0, run.stderr)
        else:
            self.assertNotEqual(run.returncode, 0)
        return run

    def rules(self):
        p = self.root / 'rules.json'
        return json.loads(p.read_text()) if p.exists() else []

    def test_key_only_independent_identity_and_lifecycle(self):
        self.run_bridge('install', self.ko)
        self.assertEqual(stat.S_IMODE(self.state.stat().st_mode), 0o700)
        self.assertEqual(stat.S_IMODE((self.state / 'hostkey').stat().st_mode), 0o600)
        self.assertFalse((self.state / 'listener.pid').exists())
        self.run_bridge('start')
        args = (self.state / 'args').read_text().splitlines()
        self.assertIn('-s', args)
        self.assertNotIn('-n', args)
        self.assertNotIn('-B', args)
        self.assertEqual(args[args.index('-D') + 1], str(self.state))
        self.assertEqual(args[args.index('-p') + 1], '0.0.0.0:2223')
        self.assertEqual(len(self.rules()), 2)
        self.run_bridge('start')
        self.assertEqual(len(self.rules()), 2)
        self.run_bridge('install', self.ko, ok=False)
        self.run_bridge('status')
        self.run_bridge('stop')
        self.assertEqual(self.rules(), [])
        self.run_bridge('status', ok=False)
        self.run_bridge('stop')

    def test_missing_keys_never_start_an_open_server(self):
        (self.ko / 'settings/SSH/authorized_keys').unlink()
        self.run_bridge('install', self.ko, ok=False)
        self.run_bridge('start', ok=False)
        self.assertEqual(self.rules(), [])

    def test_unrelated_pid_and_directory_are_untouched(self):
        self.state.mkdir()
        sentinel = self.state / 'owner'
        sentinel.write_text('someone else')
        self.run_bridge('install', self.ko, ok=False)
        self.assertEqual(sentinel.read_text(), 'someone else')
        sentinel.unlink()
        self.state.rmdir()
        self.run_bridge('install', self.ko)
        (self.state / 'listener.pid').write_text(str(os.getpid()))
        self.run_bridge('start', ok=False)
        self.run_bridge('stop', ok=False)
        self.assertEqual((self.state / 'listener.pid').read_text(), str(os.getpid()))
        os.kill(os.getpid(), 0)

    def test_firewall_failure_rolls_back_only_own_rules(self):
        foreign = ['INPUT', '-p', 'tcp', '--dport', '2222', '-j', 'ACCEPT']
        (self.root / 'rules.json').write_text(json.dumps([foreign]))
        self.run_bridge('install', self.ko)
        self.env['FAIL_FIREWALL'] = '1'
        self.run_bridge('start', ok=False)
        self.assertFalse((self.state / 'listener.pid').exists())
        self.assertEqual(self.rules(), [foreign])


if __name__ == '__main__':
    unittest.main()
