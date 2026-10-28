#!/usr/bin/env python3
"""kindle/app/update.sh on the host, with KPM faked (a script keeping its feeds in a file and logging
every call): check puts the chosen channel's feed in (once), takes the preview feed away for
releases only, refreshes and prints the search; its exit codes for no KPM, a feed that will not add,
feeds that will not list, a preview feed that will not go (releases only) and no network, and that
none of those goes on to update, search or install; install waits for the launcher to be gone, installs, starts Grimoire again (not
inside KOReader), and leaves grimoire/.update-failed when KPM fails."""
import os
import subprocess
import tempfile
import time
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
SCRIPT = os.path.join(HERE, '..', 'kindle', 'app', 'update.sh')
PREVIEW = 'https://raw.githubusercontent.com/ganten1998/grimoire-scribe/kpm/preview/manifest.json'
STABLE = 'https://raw.githubusercontent.com/ganten1998/grimoire-scribe/kpm/stable/manifest.json'

FAKE_KPM = r'''#!/bin/sh
# A stand-in for KPM 0.2's CLI, as update.sh uses it.
D=$(dirname "$0")
echo "$*" >> "$D/calls"
[ "$1" = --fbink ] && shift
[ "$1" = -y ] && shift
case "$1" in
list-repo)
    [ -e "$D/fail-list" ] && { echo "Could not read the repositories"; exit 1; }
    echo "Repositories:"
    while read -r id url; do echo "  - $id - Grimoire $id ($url)"; done < "$D/repos"
    ;;
add-repo)
    [ -e "$D/fail-add" ] && exit 1
    case "$2" in *preview*) id=grimoirepreview ;; *) id=grimoirestable ;; esac
    echo "$id $2" >> "$D/repos"
    ;;
remove-repo) [ -e "$D/fail-remove" ] && { echo "Could not remove $2"; exit 1; }; grep -v "^$2 " "$D/repos" > "$D/repos.new"; mv "$D/repos.new" "$D/repos" ;;
update) [ -e "$D/offline" ] && exit 1; echo "Updated indexed packages." ;;
search)
    printf '\033[32mFound 1 package(s) for grimoire:\033[0m\n'
    echo "  - grimoire (Grimoire): Handwriting notebooks for the jailbroken Kindle Scribe. $(cat "$D/offer")"
    ;;
install) [ -e "$D/fail-install" ] && { echo "Failed to install all packages"; exit 1; }; echo installed > "$D/installed" ;;
launch) echo launched > "$D/launched" ;;
esac
'''


def read(path):
    with open(path) as f:
        return f.read()


class UpdateScript(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp(prefix='upd-')
        self.kpm = os.path.join(self.tmp, 'kpm')
        with open(self.kpm, 'w') as f:
            f.write(FAKE_KPM)
        os.chmod(self.kpm, 0o755)
        open(os.path.join(self.tmp, 'repos'), 'w').close()
        with open(os.path.join(self.tmp, 'offer'), 'w') as f:
            f.write('2.0.0-beta.6')
        self.root = os.path.join(self.tmp, 'grimoire')
        os.makedirs(self.root)
        self.env = dict(os.environ, GRIMOIRE_KPM=self.kpm, GRIMOIRE_ROOT=self.root)

    def tearDown(self):
        subprocess.run(['rm', '-rf', self.tmp])

    def run_script(self, *args, env=None):
        return subprocess.run(['sh', SCRIPT, *args], env=env or self.env, capture_output=True, text=True, timeout=60)

    def calls(self):
        p = os.path.join(self.tmp, 'calls')
        return read(p).split('\n')[:-1] if os.path.exists(p) else []

    def test_check_preview_adds_its_feed_once(self):
        p = self.run_script('check', 'preview')
        self.assertEqual(p.returncode, 0, p.stderr)
        self.assertIn('- grimoire (Grimoire): Handwriting notebooks for the jailbroken Kindle Scribe. 2.0.0-beta.6', p.stdout)
        self.assertEqual(self.calls(), ['list-repo', 'add-repo ' + PREVIEW, 'update', 'search grimoire'])
        self.run_script('check', 'preview')
        self.assertEqual(self.calls().count('add-repo ' + PREVIEW), 1)  # there already: not again

    def test_releases_only_takes_the_preview_feed_away(self):
        self.run_script('check', 'preview')
        p = self.run_script('check', 'stable')
        self.assertEqual(p.returncode, 0, p.stderr)
        repos = read(os.path.join(self.tmp, 'repos'))
        self.assertIn('grimoirestable ' + STABLE, repos)
        self.assertNotIn('grimoirepreview', repos)
        self.assertIn('remove-repo grimoirepreview', self.calls())
        # Betas again: the stable feed may stay (the preview feed carries the releases too).
        self.run_script('check', 'preview')
        self.assertIn('grimoirestable', read(os.path.join(self.tmp, 'repos')))

    def test_failures_say_why(self):
        env = dict(self.env, GRIMOIRE_KPM=os.path.join(self.tmp, 'nothing'))
        self.assertEqual(self.run_script('check', 'preview', env=env).returncode, 3)
        open(os.path.join(self.tmp, 'fail-add'), 'w').close()
        self.assertEqual(self.run_script('check', 'preview').returncode, 4)
        os.remove(os.path.join(self.tmp, 'fail-add'))
        open(os.path.join(self.tmp, 'offline'), 'w').close()
        self.assertEqual(self.run_script('check', 'preview').returncode, 5)
        self.assertEqual(self.run_script('check', 'beta').returncode, 2)

    def test_a_feed_list_that_fails_stops_the_check_for_either_channel(self):
        open(os.path.join(self.tmp, 'fail-list'), 'w').close()
        for channel in ('preview', 'stable'):
            p = self.run_script('check', channel)
            self.assertEqual(p.returncode, 4, p.stderr)
            self.assertEqual(p.stdout, '')  # no search result to offer
            self.assertIn('Could not read the repositories', p.stderr)
        # Both checks ran `list-repo` and nothing else: no add, remove, update, search or install.
        self.assertEqual(self.calls(), ['list-repo', 'list-repo'])
        self.assertFalse(os.path.exists(os.path.join(self.tmp, 'installed')))

    def test_releases_only_stops_when_the_preview_feed_will_not_go(self):
        self.run_script('check', 'preview')
        before = len(self.calls())
        open(os.path.join(self.tmp, 'fail-remove'), 'w').close()
        p = self.run_script('check', 'stable')
        self.assertEqual(p.returncode, 4, p.stderr)
        self.assertEqual(p.stdout, '')  # the beta the preview feed lists is not offered
        self.assertIn('Could not remove grimoirepreview', p.stderr)
        # The channel change was tried and refused; update, search and install never followed it.
        self.assertEqual(self.calls()[before:], ['list-repo', 'add-repo ' + STABLE, 'remove-repo grimoirepreview'])
        self.assertIn('grimoirepreview', read(os.path.join(self.tmp, 'repos')))
        self.assertFalse(os.path.exists(os.path.join(self.tmp, 'installed')))
        # Betas again is a different request: the stable feed may stay, and nothing needs removing.
        p = self.run_script('check', 'preview')
        self.assertEqual(p.returncode, 0, p.stderr)

    def test_install_waits_then_installs_and_starts_grimoire_again(self):
        # The launcher as on the Kindle: a process nobody here waits for (an orphan, reaped by init).
        pid = int(subprocess.run(['sh', '-c', 'sleep 1.5 >/dev/null 2>&1 & echo $!'], capture_output=True,
                                 text=True).stdout)
        t0 = time.time()
        p = self.run_script('install', str(pid), '')
        self.assertGreaterEqual(time.time() - t0, 1.2)  # not before the launcher was gone
        self.assertEqual(p.returncode, 0, p.stderr)
        self.assertEqual(self.calls(), ['--fbink -y install grimoire', 'launch grimoire'])
        self.assertTrue(os.path.exists(os.path.join(self.tmp, 'installed')))
        self.assertFalse(os.path.exists(os.path.join(self.root, '.update-failed')))

    def test_inside_koreader_nothing_starts_and_a_failure_is_left_to_tell(self):
        open(os.path.join(self.tmp, 'fail-install'), 'w').close()
        p = self.run_script('install', '999999', 'koreader')
        self.assertEqual(p.returncode, 0, p.stderr)
        self.assertEqual(self.calls(), ['--fbink -y install grimoire'])  # KOReader is back: no launch
        self.assertIn('Failed to install', read(os.path.join(self.root, '.update-failed')))


if __name__ == '__main__':
    unittest.main()
