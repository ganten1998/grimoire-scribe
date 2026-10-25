import hashlib
import importlib.util
import json
import os
from pathlib import Path
import shutil
import subprocess
import tarfile
import tempfile
import unittest
import zipfile

REPO = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('kpm_package', REPO / 'tools/kpm-package.py')
kpm = importlib.util.module_from_spec(spec)
spec.loader.exec_module(kpm)


class PackageTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.zip = self.root / 'installer.zip'
        self.out = self.root / 'out'
        with zipfile.ZipFile(self.zip, 'w') as z:
            z.writestr('documents/Grimoire.sh', '#!/bin/sh\n# Name: Grimoire\n# Author: Ganten\n# Icon: data:image/png;base64,test\n# DontUseFBInk\n# Capabilities: guard unpack-only hosted\n'
                       'printf "%s\\n" "$@" >"$GRIMOIRE_US/args"\nexit 0\n__GRIMOIRE_APP_ARCHIVE__\narchive\n')
            z.writestr('documents/Grimoire - reset pen calibration.sh', '#!/bin/sh\n')
            z.writestr('LICENSE.txt', 'GPL-3.0-or-later\n')
            z.writestr('README.txt', 'Grimoire 1.1.0-rc.1\n')
        self.name = kpm.build(self.zip, '1.1.0-rc.1', self.out, 'https://example.org/v1.1.0-rc.1', REPO)
        self.pkg = self.root / 'pkg'
        self.pkg.mkdir()
        with tarfile.open(self.out / self.name) as tar:
            tar.extractall(self.pkg, filter='data')
        self.us = self.root / 'us'
        self.us.mkdir()
        self.tmp = self.root / 'tmp'
        self.tmp.mkdir()
        self.env = dict(os.environ, GRIMOIRE_US=str(self.us), GRIMOIRE_TMP=str(self.tmp), KPM_PLATFORM='kindlehf')

    def hook(self, name, *args, ok=True):
        result = subprocess.run(['sh', str(self.pkg / name), *args], env=self.env, capture_output=True, text=True)
        if ok:
            self.assertEqual(result.returncode, 0, result.stderr)
        else:
            self.assertNotEqual(result.returncode, 0)
        return result

    def test_versions_upgrade_in_order(self):
        ordered = ['1.1.0-beta.1', '1.1.0-beta.2', '1.1.0-rc.1', '1.1.0-rc.2', '1.1.0', '1.1.1-beta.1', '1.1.1', '1.2.0-beta.1', '2.0.0-beta.1']
        nums = list(map(kpm.package_version, ordered))
        self.assertEqual(nums, sorted(nums))
        self.assertEqual(kpm.package_version('1.1.0-rc.1'), [1, 1, 5001])
        for bad in ['1.1', '1.1.0-rc.0', '1.1.0-rc.1000', '1.1.0-dev.1', '../evil']:
            with self.assertRaises(ValueError):
                kpm.package_version(bad)

    def test_reproducible_and_feed_matches_artifact(self):
        first = (self.out / self.name).read_bytes()
        kpm.build(self.zip, '1.1.0-rc.1', self.out, 'https://example.org/v1.1.0-rc.1', REPO)
        self.assertEqual(first, (self.out / self.name).read_bytes())
        feed = json.loads((self.out / 'manifest.json').read_text())
        self.assertEqual(feed['manifest_version'], 2)
        artifact = feed['packages']['grimoire']['artifacts'][0]
        self.assertEqual(artifact['sha256'], hashlib.sha256(first).hexdigest())
        self.assertEqual(artifact['supported_platforms'], ['kindlehf'])
        self.assertEqual(artifact['version'], json.loads((self.pkg / 'manifest.json').read_text())['version'])
        self.assertEqual((self.out / 'manifest.json').read_bytes(), (self.out / 'manifest.v2.json').read_bytes())
        self.assertFalse((self.pkg / 'rootfs').exists())

    def test_install_upgrade_remove_keep_data_and_koreader_untouched(self):
        notebook = self.us / 'grimoire/notebooks/private.qpg'
        notebook.parent.mkdir(parents=True)
        notebook.write_bytes(b'private notebook')
        ko = self.us / 'koreader/patches/user.lua'
        ko.parent.mkdir(parents=True)
        ko.write_bytes(b'user patch')
        self.hook('install.sh')
        launcher = self.us / 'documents/Grimoire.sh'
        self.assertIn('kpm', launcher.read_text())
        self.assertFalse((self.us / 'koreader/plugins/grimoire.koplugin').exists())
        self.hook('uninstall.sh', 'upgrade')
        self.assertTrue(launcher.exists())
        self.hook('install.sh', 'upgrade')
        self.hook('uninstall.sh')
        self.assertFalse(launcher.exists())
        self.assertEqual(notebook.read_bytes(), b'private notebook')
        self.assertEqual(ko.read_bytes(), b'user patch')
        self.assertFalse((self.us / 'grimoire/.kpm-install').exists())
        self.assertFalse((self.tmp / 'grimoire.kpm-install').exists())

    def test_install_leaves_legacy_migration_for_first_launch(self):
        legacy = self.us / 'quire/notebooks/old.qpg'
        legacy.parent.mkdir(parents=True)
        legacy.write_bytes(b'legacy notebook')
        self.hook('install.sh')
        # Grimoire.sh migrates quire/ only when grimoire/ does not exist yet. Hook bookkeeping
        # must not create that directory and silently make legacy notebooks disappear from view.
        self.assertFalse((self.us / 'grimoire').exists())
        self.assertEqual(legacy.read_bytes(), b'legacy notebook')
        self.assertFalse((self.tmp / 'grimoire.kpm-install').exists())

    def test_kindles_documents_rename_boundary_and_rollback(self):
        # fsp permits same-directory rename/replace in actual documents/, but rejects moving
        # a staged launcher from grimoire/.kpm-install into documents/. Reproduce that policy.
        shims = self.root / 'shims'
        shims.mkdir()
        mv = shims / 'mv'
        mv.write_text("""#!/usr/bin/env python3
import os, pathlib, sys
args = [x for x in sys.argv[1:] if x != '-f']
src, dst = map(pathlib.Path, args)
if src.parent != dst.parent:
    print('fsp: cross-directory move denied', file=sys.stderr)
    sys.exit(1)
if os.environ.get('FAIL_RESET_MOVE') and dst.name.startswith('Grimoire - reset'):
    sys.exit(1)
os.replace(src, dst)
""")
        mv.chmod(0o755)
        self.env['PATH'] = str(shims) + os.pathsep + self.env['PATH']
        docs = self.us / 'documents'
        docs.mkdir()
        original = '#!/bin/sh\n# Name: Grimoire\n# Author: Ganten\n# old manual launcher\n'
        (docs / 'Grimoire.sh').write_text(original)
        self.env['FAIL_RESET_MOVE'] = '1'
        self.hook('install.sh', ok=False)
        self.assertEqual((docs / 'Grimoire.sh').read_text(), original)
        self.assertFalse((docs / 'Grimoire - reset pen calibration.sh').exists())
        self.assertFalse(list(docs.glob('.grimoire-kpm-*')))
        del self.env['FAIL_RESET_MOVE']
        self.hook('install.sh')
        self.assertIn('kpm', (docs / 'Grimoire.sh').read_text())
        self.assertFalse(list(docs.glob('.grimoire-kpm-*')))
        self.hook('uninstall.sh')

    def test_modified_and_unrelated_launchers_are_preserved(self):
        self.hook('install.sh')
        launcher = self.us / 'documents/Grimoire.sh'
        launcher.write_text(launcher.read_text() + '# user edit\n')
        self.hook('uninstall.sh')
        self.assertTrue(launcher.exists())
        launcher.write_text('unrelated document\n')
        self.hook('install.sh', ok=False)
        self.assertEqual(launcher.read_text(), 'unrelated document\n')

    def test_install_refuses_busy_app_and_wrong_platform(self):
        (self.tmp / 'grimoire.launcher.pid').write_text(str(os.getpid()))
        self.hook('install.sh', ok=False)
        self.assertFalse((self.us / 'documents/Grimoire.sh').exists())
        (self.tmp / 'grimoire.launcher.pid').unlink()
        self.env['KPM_PLATFORM'] = 'kindle5'
        self.hook('install.sh', ok=False)

    def test_launch_preserves_arguments(self):
        self.hook('launch.sh', '--hosted', 'koreader', '--annotate')
        self.assertEqual((self.us / 'args').read_text(), '--hosted\nkoreader\n--annotate\n')

    def test_failed_staging_does_not_replace_a_manual_install(self):
        docs = self.us / 'documents'
        docs.mkdir()
        manual = docs / 'Grimoire.sh'
        original = '#!/bin/sh\n# Name: Grimoire\n# Author: Ganten\n# manual install\n'
        manual.write_text(original)
        (self.pkg / 'scriptlets/Grimoire - reset pen calibration.sh').unlink()
        self.hook('install.sh', ok=False)
        self.assertEqual(manual.read_text(), original)
        self.assertFalse((self.us / 'grimoire/.kpm-install').exists())


class FeedTests(unittest.TestCase):
    def setUp(self):
        spec = importlib.util.spec_from_file_location('kpm_feed', REPO / 'tools/kpm-feed.py')
        self.feed = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(self.feed)
        temp = tempfile.TemporaryDirectory()
        self.addCleanup(temp.cleanup)
        self.root = Path(temp.name)
        self.source = self.root / 'source.json'
        # The feed folder starts empty, as the kpm branch does before the first publication.
        self.dest = self.root / 'feed'

    def publish(self, version, digest='test'):
        self.source.write_text(json.dumps({'manifest_version': 2, 'id': 'grimoire' + ('preview' if '-' in version else 'stable'),
            'packages': {'grimoire': {'artifacts': [{'grimoire_version': version,
                'version': kpm.package_version(version), 'sha256': digest}]}}}))
        return self.feed.update(self.source, self.dest)

    def channel(self, name):
        data = (self.dest / name / 'manifest.json').read_text()
        self.assertEqual(data, (self.dest / name / 'manifest.v2.json').read_text())
        return json.loads(data)

    def offered(self, name):
        return self.channel(name)['packages']['grimoire']['artifacts'][0]['grimoire_version']

    def snapshot(self):
        return {p.relative_to(self.dest): p.read_bytes() for p in sorted(self.dest.rglob('*')) if p.is_file()}

    def test_channels_never_downgrade(self):
        self.assertEqual(self.publish('1.1.0-rc.2'), {'preview': 'updated'})
        self.assertFalse((self.dest / 'stable').exists())
        self.assertEqual(self.publish('1.1.0-rc.1'), {'preview': 'kept newer'})
        self.assertEqual(self.offered('preview'), '1.1.0-rc.2')
        self.assertEqual(self.publish('1.1.0'), {'stable': 'updated', 'preview': 'updated'})
        self.assertEqual(self.publish('1.2.0-beta.1'), {'preview': 'updated'})
        self.assertEqual(self.offered('stable'), '1.1.0')
        self.assertEqual(self.offered('preview'), '1.2.0-beta.1')
        with self.assertRaises(ValueError):
            self.publish('1.2.0-beta.1', 'different bytes')

    def test_stable_release_is_mirrored_into_preview(self):
        self.assertEqual(self.publish('2.0.0-beta.1'), {'preview': 'updated'})
        # The first publication of a stable version also creates the stable folder.
        self.assertEqual(self.publish('2.0.0'), {'stable': 'updated', 'preview': 'updated'})
        stable, preview = self.channel('stable'), self.channel('preview')
        self.assertEqual(stable['id'], 'grimoirestable')
        self.assertEqual(preview['id'], 'grimoirepreview')
        self.assertEqual(preview['packages'], stable['packages'])
        self.assertEqual(self.offered('preview'), '2.0.0')
        # A prerelease still goes to preview only.
        self.assertEqual(self.publish('2.1.0-beta.1'), {'preview': 'updated'})
        self.assertEqual(self.offered('stable'), '2.0.0')
        self.assertEqual(self.offered('preview'), '2.1.0-beta.1')

    def test_older_stable_never_replaces_newer_preview(self):
        self.publish('2.1.0-beta.1')
        self.assertEqual(self.publish('2.0.1'), {'stable': 'updated', 'preview': 'kept newer'})
        self.assertEqual(self.offered('stable'), '2.0.1')
        self.assertEqual(self.offered('preview'), '2.1.0-beta.1')
        self.assertEqual(self.channel('preview')['id'], 'grimoirepreview')

    def test_rerun_is_idempotent_and_refusal_changes_nothing(self):
        self.publish('2.0.0-rc.1')
        self.assertEqual(self.publish('2.0.0'), {'stable': 'updated', 'preview': 'updated'})
        before = self.snapshot()
        self.assertEqual(self.publish('2.0.0'), {'stable': 'unchanged', 'preview': 'unchanged'})
        self.assertEqual(self.snapshot(), before)
        with self.assertRaises(ValueError):
            self.publish('2.0.0', 'different bytes')
        self.assertEqual(self.snapshot(), before)
        # Preview already offering this stable version with other bytes refuses stable too.
        self.dest.joinpath('stable/manifest.json').unlink()
        with self.assertRaises(ValueError):
            self.publish('2.0.0', 'different bytes')
        self.assertFalse((self.dest / 'stable/manifest.json').exists())

    def test_mirrored_preview_matches_a_preview_build(self):
        # The channel fields kpm-feed.py gives the mirror are the ones kpm-package.py writes.
        built = {}
        for version in ('2.0.0', '2.0.0-rc.1'):
            installer = self.root / (version + '.zip')
            with zipfile.ZipFile(installer, 'w') as z:
                z.writestr('documents/Grimoire.sh', '#!/bin/sh\n# Name: Grimoire\n# Icon: data:image/png;base64,test\n'
                           'exit 0\n__GRIMOIRE_APP_ARCHIVE__\narchive\n')
                z.writestr('documents/Grimoire - reset pen calibration.sh', '#!/bin/sh\n')
                z.writestr('LICENSE.txt', 'GPL-3.0-or-later\n')
                z.writestr('README.txt', 'Grimoire ' + version + '\n')
            out = self.root / ('out-' + version)
            kpm.build(installer, version, out, 'https://example.org/v' + version, REPO)
            built[version] = json.loads((out / 'manifest.json').read_text())
        self.assertEqual(self.feed.update(self.root / 'out-2.0.0/manifest.json', self.dest),
                         {'stable': 'updated', 'preview': 'updated'})
        self.assertEqual(self.channel('stable'), built['2.0.0'])
        preview = self.channel('preview')
        for key in ('manifest_version', 'id', 'name', 'description'):
            self.assertEqual(preview[key], built['2.0.0-rc.1'][key], key)
        self.assertEqual(preview['packages'], built['2.0.0']['packages'])


if __name__ == '__main__':
    unittest.main()
