#!/usr/bin/env python3
"""kindle/app/bluetooth.sh on the host, with the Kindle's commands faked: the stock Bluetooth jobs
(status/start/initctl), the helper (a script that answers /health while it runs) and curl. Checks
that it starts only a helper of its own, never one someone else runs, records and restores the stock
jobs, stops what it started and whatever runs from Grimoire's own folder (recorded or not) but never a
helper someone else runs elsewhere, and keeps the media remote off in Grimoire's own folder. The fake
helper runs as the real one does: its program execs a loader in dist/, so its command line names its
folder and never kindle-hid-passthrough."""
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
        # pgrep, blind to every process whose command line does not name this test's folder: another
        # run's helpers (a gate in another worktree runs these tests too) are never seen.
        write(os.path.join(self.bin, 'pgrep'),
              '#!/bin/sh\nfound=1\nfor p in $(/usr/bin/pgrep "$@"); do\n'
              '  if ps -o command= -p "$p" 2>/dev/null | grep -qF "%s"; then echo "$p"; found=0; fi\n'
              'done\nexit $found\n' % self.tmp)
        self.env = dict(os.environ, PATH=self.bin + ':/usr/bin:/bin', GRIMOIRE_ROOT=self.root,
                        BT_STATE=os.path.join(self.tmp, 'state'), BT_NO_PROC='1')
        self.launched = []

    def tearDown(self):
        pid_file = os.path.join(self.tmp, 'state', 'pid')
        if os.path.exists(pid_file):
            try:
                os.kill(int(read(pid_file)), signal.SIGKILL)
            except (OSError, ValueError):
                pass
        for pid in self.launched:
            try:
                os.kill(pid, signal.SIGKILL)
            except OSError:
                pass
        cursor = os.path.join(self.tmp, 'cursor.pid')
        if os.path.exists(cursor):
            try:
                os.kill(int(read(cursor)), signal.SIGKILL)
            except (OSError, ValueError):
                pass
        subprocess.run(['rm', '-rf', self.tmp])

    def program(self, d, body):
        # As the helper's release ships it: kindle-hid-passthrough is a wrapper that execs the loader
        # in dist/ with dist/main.bin (here the loader is the helper's body), so the helper's command
        # line is "<d>/dist/ld-linux-armhf.so.3 --library-path <d>/dist <d>/dist/main.bin --daemon".
        write(os.path.join(d, 'kindle-hid-passthrough'),
              '#!/bin/sh\nd=$(cd "$(dirname "$0")" && pwd)\n'
              'exec "$d/dist/ld-linux-armhf.so.3" --library-path "$d/dist" "$d/dist/main.bin" "$@"\n')
        write(os.path.join(d, 'dist', 'ld-linux-armhf.so.3'), '#!/bin/sh\n' + body)
        write(os.path.join(d, 'dist', 'main.bin'), '', 0o644)

    def serving(self, up):
        # --daemon makes the file "up" and runs until TERM (which takes it away).
        return 'touch "%s"\ntrap \'rm -f "%s"; exit 0\' TERM\nwhile :; do sleep 0.1; done\n' % (up, up)

    def helper(self, behaviour='serve'):
        # The helper: --daemon makes "up" and runs until TERM (which takes "up" away), or dies at once.
        d = os.path.join(self.root, 'bluetooth')
        body = {'serve': self.serving(os.path.join(self.tmp, 'up')), 'die': 'exit 3\n'}[behaviour]
        self.program(d, body)
        # As the helper's release ships it (3.18), shortened.
        write(os.path.join(d, 'config.ini'),
              '# BLE/Classic HID Host Configuration\n\n[paths]\ncache_dir = /mnt/us/kindle_hid_passthrough/cache\n'
              'devices_config = /mnt/us/kindle_hid_passthrough/devices.conf\n\n[connection]\nreconnect_delay = 5\n\n'
              '[media_remote]\n# Phone volume keys turn pages\nenabled = true\n\n[logging]\n'
              'log_file = /var/log/hid_passthrough.log\n', 0o644)
        return d

    def stubborn_helper(self, cursor=False):
        # A helper that ignores TERM (stuck detaching a device): stop-owned waits its 12 s, then KILLs.
        # With cursor, it first starts its mouse cursor overlay (scripts/mousecursor), which it would
        # take down on a clean exit and cannot once killed.
        d = os.path.join(self.root, 'bluetooth')
        start = ''
        if cursor:
            write(os.path.join(d, 'scripts', 'mousecursor'),
                  '#!/bin/sh\necho $$ > "%s/cursor.pid"\nwhile :; do /bin/sleep 0.1; done\n' % self.tmp)
            start = '"%s/scripts/mousecursor" &\n' % d
        self.program(d, 'trap "" TERM\n%stouch "%s/up"\nwhile :; do /bin/sleep 0.1; done\n' % (start, self.tmp))
        return d

    def busybox_env(self, with_usleep):
        # The Kindle's busybox: sleep takes whole seconds only, usleep takes microseconds. Both are
        # recorded and return at once, so the test counts the waits instead of sitting through them.
        bb = os.path.join(self.tmp, 'busybox')
        log = os.path.join(self.tmp, 'naps')
        write(os.path.join(bb, 'sleep'),
              '#!/bin/sh\ncase "$1" in *[!0-9]*) echo "sleep: invalid number \'$1\'" >&2; exit 1 ;; esac\n'
              'echo "sleep $1" >> "%s"\n' % log)
        if with_usleep:
            write(os.path.join(bb, 'usleep'), '#!/bin/sh\necho "usleep $1" >> "%s"\n' % log)
        return dict(self.env, PATH=bb + ':' + self.env['PATH']), log

    def run_script(self, *args):
        p = subprocess.run(['sh', SCRIPT, *args], env=self.env, capture_output=True, text=True, timeout=30)
        return p.returncode, p.stdout.strip(), p.stderr

    def launch(self, d):
        # The helper in d started by someone, not by the script (as the helper's own scripts or a
        # restart would), from a shell that leaves at once, as the helper's own start script does: its
        # pid. (Not a child of this test, which would keep it a zombie, alive to kill -0, once stopped.)
        p = subprocess.run(['sh', '-c', '"$1" --daemon </dev/null >/dev/null 2>&1 & echo $!', 'sh',
                            os.path.join(d, 'kindle-hid-passthrough')], capture_output=True, text=True, timeout=10)
        pid = int(p.stdout)
        self.launched.append(pid)
        return pid

    def wait_for(self, what, timeout=10.0):
        end = time.time() + timeout
        while time.time() < end:
            if what():
                return True
            time.sleep(0.05)
        return what()

    @staticmethod
    def alive(pid):
        try:
            os.kill(pid, 0)
        except OSError:
            return False
        # (a zombie of ours, not yet reaped, is gone too)
        out = subprocess.run(['ps', '-o', 'stat=', '-p', str(pid)], capture_output=True, text=True).stdout
        return out.strip() != '' and not out.strip().startswith('Z')

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
            # (The helper itself runs from Grimoire's folder: Grimoire's, recorded or not, so it is
            # stopped all the same.)
            self.assertFalse(self.alive(int(real)))
        finally:
            other.kill()
            other.wait()
            try:
                os.kill(int(real), signal.SIGKILL)
            except OSError:
                pass


    def stop_with_busybox(self, with_usleep):
        self.stubborn_helper()
        self.assertEqual(self.run_script('start')[1], 'owned')
        env, log = self.busybox_env(with_usleep)
        p = subprocess.run(['sh', SCRIPT, 'stop-owned'], env=env, capture_output=True, text=True, timeout=30)
        self.assertEqual(p.stdout.strip(), 'stopped')
        self.assertNotIn('invalid number', p.stderr)
        self.assertIn('the helper did not stop: killing it', p.stderr)
        return read(log).split('\n')[:-1]

    def test_the_stop_grace_waits_with_busybox_usleep(self):
        # 12 s of grace before the KILL: sixty fifths of a second.
        self.assertEqual(self.stop_with_busybox(True), ['usleep 200000'] * 60)

    def test_the_stop_grace_waits_whole_seconds_without_usleep(self):
        # No usleep and no fractions: twelve whole seconds, the same 12 s.
        self.assertEqual(self.stop_with_busybox(False), ['sleep 1'] * 12)

    def test_a_helper_whose_record_was_lost_is_still_stopped(self):
        # The record goes (a pid file removed, /tmp/grimoire-bt cleaned) while the helper runs: the
        # stop used to know the helper by that record alone and left it running, with the Kindle's own
        # jobs started again beside it. It runs from Grimoire's folder, so it is stopped.
        self.helper()
        open(os.path.join(self.jobs, 'btmanagerd'), 'w').close()
        self.assertEqual(self.run_script('start')[1], 'owned')
        pid = int(read(os.path.join(self.tmp, 'state', 'pid')))
        os.remove(os.path.join(self.jobs, 'btmanagerd'))  # the helper stopped it
        os.remove(os.path.join(self.tmp, 'state', 'pid'))
        os.remove(os.path.join(self.tmp, 'state', 'dir'))
        rc, out, err = self.run_script('stop-owned')
        self.assertEqual((rc, out), (0, 'stopped'), err)
        self.assertTrue(self.wait_for(lambda: not self.alive(pid)))
        self.assertFalse(os.path.exists(os.path.join(self.tmp, 'up')))
        self.assertEqual(read(os.path.join(self.tmp, 'started')).split(), ['btmanagerd'])

    def test_a_helper_restarted_outside_the_script_is_stopped(self):
        # The recorded helper ended and the same program was started again by someone else (the
        # helper's own scripts, say): the record names a pid that is gone.
        d = self.helper()
        self.assertEqual(self.run_script('start')[1], 'owned')
        first = int(read(os.path.join(self.tmp, 'state', 'pid')))
        os.kill(first, signal.SIGTERM)
        self.assertTrue(self.wait_for(lambda: not os.path.exists(os.path.join(self.tmp, 'up'))))
        again = self.launch(d)
        self.assertTrue(self.wait_for(lambda: os.path.exists(os.path.join(self.tmp, 'up'))))
        rc, out, err = self.run_script('stop-owned')
        self.assertEqual((rc, out), (0, 'stopped'), err)
        self.assertNotIn('killing it', err)  # TERM, and it left as it does
        self.assertFalse(self.alive(again))
        self.assertFalse(os.path.exists(os.path.join(self.tmp, 'up')))

    def test_start_takes_back_a_helper_of_its_own_folder(self):
        # One runs from Grimoire's folder with no record: start takes it back as its own instead of
        # calling it someone else's (whose helper Grimoire never stops) or starting a second one.
        d = self.helper()
        running = self.launch(d)
        self.assertTrue(self.wait_for(lambda: os.path.exists(os.path.join(self.tmp, 'up'))))
        rc, out, err = self.run_script('start')
        self.assertEqual((rc, out), (0, 'owned'), err)
        self.assertIn('taking it back', err)
        self.assertEqual(int(read(os.path.join(self.tmp, 'state', 'pid'))), running)
        self.assertFalse(os.path.exists(os.path.join(self.tmp, 'state', 'helper.log')))  # none started
        self.assertEqual(self.run_script('status')[1], 'owned')
        rc, out, err = self.run_script('stop-owned')
        self.assertEqual((rc, out), (0, 'stopped'), err)
        self.assertNotIn('killing it', err)
        self.assertFalse(self.alive(running))

    def test_someone_elses_running_helper_is_never_stopped(self):
        # A helper unpacked elsewhere and started by someone else, before its API answers: its command
        # line names neither kindle-hid-passthrough nor Grimoire's folder, only its loader and program.
        self.helper()
        other = os.path.join(self.tmp, 'elsewhere', 'khp')
        self.program(other, self.serving(os.path.join(self.tmp, 'other-up')))
        p = self.launch(other)
        self.assertTrue(self.wait_for(lambda: os.path.exists(os.path.join(self.tmp, 'other-up'))))
        self.assertEqual(self.run_script('start')[1], 'external')  # not a second helper
        self.assertFalse(os.path.exists(os.path.join(self.tmp, 'state', 'pid')))
        self.assertEqual(self.run_script('stop-owned')[1], 'stopped')
        self.assertTrue(self.alive(p))  # still running
        self.assertTrue(os.path.exists(os.path.join(self.tmp, 'other-up')))

    def test_a_killed_helper_leaves_no_cursor_overlay(self):
        # The helper takes its mouse cursor overlay down as it leaves; one that has to be killed
        # cannot, and the overlay stayed on the screen. It runs from the helper's folder: it goes too.
        self.stubborn_helper(cursor=True)
        self.assertEqual(self.run_script('start')[1], 'owned')
        cursor = os.path.join(self.tmp, 'cursor.pid')
        self.assertTrue(self.wait_for(lambda: os.path.exists(cursor) and read(cursor).strip() != ''))
        overlay = int(read(cursor))
        env, _ = self.busybox_env(True)
        p = subprocess.run(['sh', SCRIPT, 'stop-owned'], env=env, capture_output=True, text=True, timeout=30)
        self.assertEqual(p.stdout.strip(), 'stopped')
        self.assertIn('the helper did not stop: killing it', p.stderr)
        self.assertTrue(self.wait_for(lambda: not self.alive(overlay)))

    def test_a_stale_lock_is_taken_after_its_wait(self):
        env, log = self.busybox_env(True)
        os.makedirs(os.path.join(self.tmp, 'state.lock'))
        self.helper()
        p = subprocess.run(['sh', SCRIPT, 'start'], env=env, capture_output=True, text=True, timeout=30)
        self.assertIn('a stale lock: taking it', p.stderr)
        self.assertNotIn('invalid number', p.stderr)
        # 20 s of waiting (a hundred fifths) came before the lock was taken.
        self.assertGreaterEqual(read(log).count('usleep 200000'), 100)


if __name__ == '__main__':
    unittest.main()
