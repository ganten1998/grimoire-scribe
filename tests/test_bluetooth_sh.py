#!/usr/bin/env python3
"""kindle/app/bluetooth.sh on the host, with the Kindle's commands faked: the stock Bluetooth jobs
(status/start/initctl), the helper (a script that answers /health while it runs) and curl. Checks
that it starts only a helper of its own, never one someone else runs, records and restores the stock
jobs, stops only what it started, and keeps the media remote off in Grimoire's own folder."""
import os
import signal
import stat
import subprocess
import tempfile
import time
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
SCRIPT = os.path.join(HERE, '..', 'kindle', 'app', 'bluetooth.sh')


def read(path):
    with open(path) as f:
        return f.read()


def write(path, text, mode=0o755):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, 'w') as f:
        f.write(text)
    os.chmod(path, mode)


class BluetoothScript(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp(prefix='bt-')
        self.bin = os.path.join(self.tmp, 'bin')
        self.root = os.path.join(self.tmp, 'grimoire')
        self.jobs = os.path.join(self.tmp, 'jobs')  # a file per running stock job
        os.makedirs(self.jobs)
        # Upstart, as far as the script uses it.
        write(os.path.join(self.bin, 'status'),
              '#!/bin/sh\nif [ -e "%s/$1" ]; then echo "$1 start/running, process 1"; '
              'else echo "$1 stop/waiting"; fi\n' % self.jobs)
        write(os.path.join(self.bin, 'start'), '#!/bin/sh\ntouch "%s/$1"\necho "$1" >> "%s/started"\n'
              % (self.jobs, self.tmp))
        write(os.path.join(self.bin, 'initctl'), '#!/bin/sh\nexit 1\n')
        # curl: /health answers while the fake helper's "up" file is there.
        write(os.path.join(self.bin, 'curl'), '#!/bin/sh\nif [ -e "%s/up" ]; then echo \'{"ok": true}\'; '
              'else exit 7; fi\n' % self.tmp)
        write(os.path.join(self.bin, 'pgrep'), '#!/bin/sh\nexit 1\n')
        self.env = dict(os.environ, PATH=self.bin + ':/usr/bin:/bin', GRIMOIRE_ROOT=self.root,
                        BT_STATE=os.path.join(self.tmp, 'state'), BT_NO_PROC='1')

    def tearDown(self):
        pid_file = os.path.join(self.tmp, 'state', 'pid')
        if os.path.exists(pid_file):
            try:
                os.kill(int(read(pid_file)), signal.SIGKILL)
            except (OSError, ValueError):
                pass
        subprocess.run(['rm', '-rf', self.tmp])

    def helper(self, behaviour='serve'):
        # The helper: --daemon makes "up" and runs until TERM (which takes "up" away), or dies at once.
        d = os.path.join(self.root, 'bluetooth')
        body = {'serve': 'touch "%s/up"\ntrap \'rm -f "%s/up"; exit 0\' TERM\nwhile :; do sleep 0.1; done\n'
                         % (self.tmp, self.tmp),
                'die': 'exit 3\n'}[behaviour]
        write(os.path.join(d, 'kindle-hid-passthrough'), '#!/bin/sh\n' + body)
        # As the helper's release ships it (3.18), shortened.
        write(os.path.join(d, 'config.ini'),
              '# BLE/Classic HID Host Configuration\n\n[paths]\ncache_dir = /mnt/us/kindle_hid_passthrough/cache\n'
              'devices_config = /mnt/us/kindle_hid_passthrough/devices.conf\n\n[connection]\nreconnect_delay = 5\n\n'
              '[media_remote]\n# Phone volume keys turn pages\nenabled = true\n\n[logging]\n'
              'log_file = /var/log/hid_passthrough.log\n', 0o644)
        return d

    def run_script(self, *args):
        p = subprocess.run(['sh', SCRIPT, *args], env=self.env, capture_output=True, text=True, timeout=30)
        return p.returncode, p.stdout.strip(), p.stderr

    def test_missing_helper(self):
        self.assertEqual(self.run_script('status')[1], 'missing')
        rc, out, _ = self.run_script('start')
        self.assertEqual((rc, out), (1, 'missing'))

    def test_start_stop_restores_the_stock_jobs(self):
        d = self.helper()
        open(os.path.join(self.jobs, 'btmanagerd'), 'w').close()  # running before; acsbtfd not
        self.assertEqual(self.run_script('status')[1], 'stopped')
        rc, out, err = self.run_script('start')
        self.assertEqual((rc, out), (0, 'owned'), err)
        self.assertTrue(os.path.exists(os.path.join(self.tmp, 'up')))
        cfg = read(os.path.join(d, 'config.ini'))
        self.assertIn('[media_remote]\n# Phone volume keys turn pages\nenabled = false\n', cfg)
        self.assertIn('cache_dir = %s/cache\ndevices_config = %s/devices.conf\n' % (d, d), cfg)
        self.assertIn('[connection]\nreconnect_delay = 5\n', cfg)  # the rest as it was
        self.assertNotIn('enabled = true', cfg)
        self.assertEqual(self.run_script('start')[1], 'owned')  # once is enough
        self.assertEqual(read(os.path.join(d, 'config.ini')), cfg)  # (and never twice)
        self.assertEqual(self.run_script('status')[1], 'owned')
        os.remove(os.path.join(self.jobs, 'btmanagerd'))  # the helper stopped it
        rc, out, _ = self.run_script('stop-owned')
        self.assertEqual((rc, out), (0, 'stopped'))
        self.assertFalse(os.path.exists(os.path.join(self.tmp, 'up')))
        self.assertEqual(read(os.path.join(self.tmp, 'started')).split(), ['btmanagerd'])
        self.assertEqual(self.run_script('stop-owned')[1], 'stopped')  # idempotent, starts nothing again
        self.assertEqual(read(os.path.join(self.tmp, 'started')).split(), ['btmanagerd'])

    def test_someone_elses_helper_is_left_alone(self):
        self.helper()
        open(os.path.join(self.tmp, 'up'), 'w').close()  # a helper this script did not start answers
        self.assertEqual(self.run_script('start')[1], 'external')
        self.assertEqual(self.run_script('stop-owned')[1], 'stopped')
        self.assertTrue(os.path.exists(os.path.join(self.tmp, 'up')))
        self.assertFalse(os.path.exists(os.path.join(self.tmp, 'started')))

    def test_a_helper_that_dies_gives_the_jobs_back(self):
        self.helper('die')
        open(os.path.join(self.jobs, 'acsbtfd'), 'w').close()
        rc, out, _ = self.run_script('start')
        self.assertEqual((rc, out), (1, 'failed'))
        self.assertEqual(self.run_script('status')[1], 'stopped')

    def test_only_the_recorded_pid_is_stopped(self):
        self.helper()
        self.assertEqual(self.run_script('start')[1], 'owned')
        state = os.path.join(self.tmp, 'state')
        real = read(os.path.join(state, 'pid'))
        # A pid file that names some other process (a reboot reused the number): not ours to stop.
        other = subprocess.Popen(['sleep', '30'])
        try:
            with open(os.path.join(state, 'pid'), 'w') as f:
                f.write(str(other.pid))
            env = dict(self.env)
            env.pop('BT_NO_PROC')
            p = subprocess.run(['sh', SCRIPT, 'stop-owned'], env=env, capture_output=True, text=True, timeout=30)
            self.assertEqual(p.stdout.strip(), 'stopped')
            self.assertIsNone(other.poll())  # still running
        finally:
            other.kill()
            os.kill(int(real), signal.SIGKILL)


if __name__ == '__main__':
    unittest.main()
