"""Exercise actual Library folder actions, then inspect their filesystem effects."""
import os
from pathlib import Path
import subprocess
import tempfile
import unittest


class MirrorCleanup(unittest.TestCase):
    def exercise(self, *, rename=False, full=False, content=False, link=False):
        with tempfile.TemporaryDirectory(prefix="grimoire-mirror-cleanup-") as directory:
            root = Path(directory)
            (root / "folders.ini").write_text("# Grimoire folders\nOld\n")
            mirror = root / "PDFs"
            mirror.mkdir()
            old = mirror / "Old"
            outside = root / "outside"
            outside.mkdir()
            if link:
                old.symlink_to(outside, target_is_directory=True)
            else:
                old.mkdir()
            if content:
                (old / "notes.txt").write_bytes(b"keep this file")
            # (the folder menu's rows by name: rc.12 added Move folder between Rename and Remove)
            actions = "tap library fmore.rename\ntap 569 2205\ntap 1643 2365\n" if rename else "tap library fmore.remove\n"
            script = root / "test.qs"
            script.write_text("wait 100\nexpect library.items is 1\ntap 547 1052\n"
                              "expect library.sheet is folder-more\n" +
                              ("storage full\n" if full else "") + actions +
                              "expect library.sheet is none\nexpect library.items is " +
                              ("1" if rename or full else "0") + "\nquit\n")
            env = {k: v for k, v in os.environ.items() if not k.startswith("GRIMOIRE_")}
            env.update(GRIMOIRE_ROOT=str(root), GRIMOIRE_SCRIPT=str(script), GRIMOIRE_LANG="en")
            binary = Path(os.environ.get("BUILD", "build")) / "host/grimoire-headless"
            result = subprocess.run([str(binary.resolve())], env=env, capture_output=True, text=True, timeout=30)
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            self.assertEqual(old.exists(), full or content or link)
            self.assertTrue(outside.is_dir())
            if content:
                self.assertEqual((old / "notes.txt").read_bytes(), b"keep this file")
            if link:
                self.assertTrue(old.is_symlink())
            if rename and not full:
                self.assertIn("Oldx\n", (root / "folders.ini").read_text())

    def test_remove_empty_folder(self):
        self.exercise()

    def test_rename_empty_folder(self):
        self.exercise(rename=True)

    def test_failed_save_keeps_directory(self):
        self.exercise(full=True)

    def test_nonempty_directory_is_kept(self):
        self.exercise(content=True)

    def test_symlink_is_kept(self):
        self.exercise(link=True)

    def test_failed_rename_keeps_directory(self):
        self.exercise(rename=True, full=True)
