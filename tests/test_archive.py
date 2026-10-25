import os
from pathlib import Path
import subprocess
import sys
import tarfile
import tempfile
import time
import unittest
import zipfile

REPO = Path(__file__).resolve().parents[1]
TOOL = REPO / 'tools/archive.py'
EPOCH = 1761350400  # 2025-10-25 00:00:00 UTC


class ArchiveTests(unittest.TestCase):
    def setUp(self):
        temp = tempfile.TemporaryDirectory()
        self.addCleanup(temp.cleanup)
        self.root = Path(temp.name)

    def tree(self, name, order, file_mode, dir_mode, mtime):
        """The same files under ROOT/NAME, made in ORDER, with the modes and times a builder's
        umask and clock would give them."""
        base = self.root / name
        files = {'app/grimoire': b'\x7fELF binary', 'app/grimoire.sh': b'#!/bin/sh\n',
                 'app/licenses/b.txt': b'b\n', 'app/licenses/a.txt': b'a\n', 'app/CAPABILITIES': b'guard\n',
                 'README.txt': b'Grimoire\n'}
        exe = 0o100 if file_mode == 0o600 else 0o111  # chmod +x under umask 077, and under 022
        for rel in (sorted(files) if order else sorted(files, reverse=True)):
            path = base / rel
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(files[rel])
            path.chmod(file_mode | exe if rel.endswith(('grimoire', '.sh')) else file_mode)
            os.utime(path, (mtime, mtime))
        for path in [base, *base.rglob('*')]:
            if path.is_dir():
                path.chmod(dir_mode)
                os.utime(path, (mtime, mtime))
        return base

    def archive(self, fmt, base, *names, epoch=EPOCH, tz='UTC'):
        out = self.root / f'{base.name}-{epoch}-{tz.replace("/", "_")}.{fmt}'
        env = dict(os.environ, SOURCE_DATE_EPOCH=str(epoch), TZ=tz)
        result = subprocess.run([sys.executable, str(TOOL), fmt, str(out), '-C', str(base), *names],
                                env=env, capture_output=True, text=True)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertFalse(out.with_name(out.name + '.tmp').exists())
        return out

    def test_same_files_make_the_same_bytes(self):
        one = self.tree('one', True, 0o644, 0o755, 1000000000)
        two = self.tree('two', False, 0o600, 0o700, 1700000000)
        for fmt in ('tar', 'zip'):
            a = self.archive(fmt, one, 'app', 'README.txt')
            b = self.archive(fmt, two, 'app', 'README.txt', tz='Asia/Kathmandu')
            self.assertEqual(a.read_bytes(), b.read_bytes(), fmt)
            self.assertNotEqual(a.read_bytes(), self.archive(fmt, one, 'app', 'README.txt', epoch=EPOCH + 3600).read_bytes())

    def test_tar_metadata_and_order(self):
        base = self.tree('t', False, 0o600, 0o700, 1700000000)
        with tarfile.open(self.archive('tar', base, 'app', 'README.txt')) as tar:
            members = tar.getmembers()
        self.assertEqual([m.name for m in members], ['app', 'app/CAPABILITIES', 'app/grimoire', 'app/grimoire.sh',
                                                     'app/licenses', 'app/licenses/a.txt', 'app/licenses/b.txt', 'README.txt'])
        modes = {m.name: m.mode for m in members}
        self.assertEqual(modes['app'], 0o755)
        self.assertEqual(modes['app/licenses'], 0o755)
        self.assertEqual(modes['app/grimoire'], 0o755)
        self.assertEqual(modes['app/grimoire.sh'], 0o755)
        self.assertEqual(modes['app/CAPABILITIES'], 0o644)
        for m in members:
            self.assertEqual((m.mtime, m.uid, m.gid, m.uname, m.gname, m.pax_headers), (EPOCH, 0, 0, 'root', 'root', {}), m.name)

    def test_zip_metadata_and_order(self):
        base = self.tree('z', True, 0o600, 0o700, 1700000000)
        with zipfile.ZipFile(self.archive('zip', base, 'README.txt', 'app', tz='America/St_Johns')) as archive:
            infos = archive.infolist()
            self.assertEqual(archive.read('app/grimoire.sh'), b'#!/bin/sh\n')
        self.assertEqual([i.filename for i in infos], ['README.txt', 'app/', 'app/CAPABILITIES', 'app/grimoire', 'app/grimoire.sh',
                                                       'app/licenses/', 'app/licenses/a.txt', 'app/licenses/b.txt'])
        modes = {i.filename: i.external_attr >> 16 for i in infos}
        self.assertEqual(modes['app/'], 0o40755)
        self.assertEqual(modes['app/grimoire'], 0o100755)
        self.assertEqual(modes['README.txt'], 0o100644)
        for i in infos:
            self.assertEqual(i.date_time, time.gmtime(EPOCH)[:6], i.filename)
            self.assertEqual((i.create_system, i.extra), (3, b''), i.filename)
            self.assertEqual(i.compress_type, zipfile.ZIP_STORED if i.is_dir() else zipfile.ZIP_DEFLATED, i.filename)

    def test_time_comes_from_the_commit_without_source_date_epoch(self):
        base = self.tree('c', True, 0o644, 0o755, 1700000000)
        env = {k: v for k, v in os.environ.items() if k != 'SOURCE_DATE_EPOCH'}
        out = self.root / 'commit.tar'
        subprocess.run([sys.executable, str(TOOL), 'tar', str(out), '-C', str(base), 'README.txt'], env=env, check=True)
        top = subprocess.run(['git', '-C', str(REPO), 'rev-parse', '--show-toplevel'], capture_output=True, text=True)
        if top.returncode == 0 and Path(top.stdout.strip()).resolve() == REPO:
            want = int(subprocess.run(['git', '-C', str(REPO), 'log', '-1', '--format=%ct'], capture_output=True, text=True).stdout)
        else:
            want = 315532800
        with tarfile.open(out) as tar:
            self.assertEqual(tar.getmember('README.txt').mtime, want)

    def test_refuses_links_and_paths_outside(self):
        base = self.tree('r', True, 0o644, 0o755, 1700000000)
        (base / 'link').symlink_to('README.txt')
        for names in (['link'], ['../r/README.txt'], ['missing']):
            out = self.root / 'refused.zip'
            result = subprocess.run([sys.executable, str(TOOL), 'zip', str(out), '-C', str(base), *names], capture_output=True)
            self.assertNotEqual(result.returncode, 0, names)
            self.assertFalse(out.exists())
            self.assertFalse(out.with_name('refused.zip.tmp').exists())


if __name__ == '__main__':
    unittest.main()
