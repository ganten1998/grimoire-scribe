"""tests/scenario_runner.py's own checks (make runner-check): the CI shards' split, the order the
skipped scenarios are found in, and what a failed run reports, with a stand-in for the headless
binary. python3 -m unittest discover -s tests -p 'test_scenario_runner.py' -v"""
import argparse
import contextlib
import glob
import io
import os
import signal
import sys
import tempfile
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import scenario_runner as sr  # noqa: E402


@contextlib.contextmanager
def deadline(seconds):
    """Fails the test, instead of hanging it, if the block takes longer."""
    def expired(signum, frame):
        raise AssertionError(f'still running after {seconds} s')
    old = signal.signal(signal.SIGALRM, expired)
    signal.alarm(seconds)
    try:
        yield
    finally:
        signal.alarm(0)
        signal.signal(signal.SIGALRM, old)


class Scratch(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.dir = self.tmp.name

    def tearDown(self):
        self.tmp.cleanup()

    def qs(self, name, *lines):
        path = os.path.join(self.dir, 'scen', name + '.qs')
        os.makedirs(os.path.dirname(path), exist_ok=True)
        with open(path, 'w', encoding='utf-8') as f:
            f.write(''.join(line + '\n' for line in lines) + 'wait 1\n')
        return path


class Shards(Scratch):
    def check_split(self, files, shards, times):
        """Every shard's list: each scenario in at least one, each with its roots beside it."""
        names = {os.path.splitext(os.path.basename(f))[0]: f for f in files}
        root = {n: sr.header(f)['root'] for n, f in names.items()}
        seen = set()
        with deadline(20):
            for i in range(1, shards + 1):
                got, _ = sr.shard_select(files, shards, i, times)
                mine = {os.path.splitext(os.path.basename(f))[0] for f in got}
                for n in mine:
                    if root[n] in names:
                        self.assertIn(root[n], mine, f'{n} in shard {i}/{shards} without its root')
                seen |= mine
        self.assertEqual(seen, set(names))

    def test_a_single_child_chain_over_the_target_ends(self):
        files = [self.qs('a'), self.qs('b', '# root: a'), self.qs('c', '# root: b'), self.qs('x')]
        self.check_split(sorted(files), 2, {'a': 10, 'b': 10, 'c': 10, 'x': 1})

    def test_a_child_with_its_own_dependants_is_split_below_it(self):
        # r -> k (-> g1, g2, g3, each heavy) and r -> j: k's part alone is over the target.
        files = [self.qs('r'), self.qs('k', '# root: r'), self.qs('j', '# root: r'),
                 self.qs('g1', '# root: k'), self.qs('g2', '# root: k'), self.qs('g3', '# root: k')]
        times = {'r': 1, 'k': 1, 'j': 1, 'g1': 100, 'g2': 100, 'g3': 100}
        self.check_split(sorted(files), 3, times)
        with deadline(20):
            parts = [sr.shard_select(sorted(files), 3, i, times)[0] for i in (1, 2, 3)]
        with_g = [sum(1 for f in p if os.path.basename(f).startswith('g')) for p in parts]
        self.assertEqual(with_g, [1, 1, 1], 'the three heavy grandchildren, one per shard')

    def test_a_cycle_goes_to_shard_one(self):
        files = sorted([self.qs('e', '# root: f'), self.qs('f', '# root: e'), self.qs('x')])
        with deadline(20):
            got, _ = sr.shard_select(files, 2, 1, {})
        self.assertEqual({os.path.basename(f) for f in got} & {'e.qs', 'f.qs'}, {'e.qs', 'f.qs'})

    def test_the_real_scenarios_at_every_shard_count(self):
        files = sorted(glob.glob(os.path.join(HERE, 'scenarios', '*.qs')))
        times = sr.read_times(sr.TIMES_FILE)
        for shards in (1, 2, 6, 15, 55, 60, 100):
            self.check_split(files, shards, times)

    def test_the_real_scenarios_with_a_heavy_child_tree(self):
        files = sorted(glob.glob(os.path.join(HERE, 'scenarios', '*.qs')))
        times = dict(sr.read_times(sr.TIMES_FILE))
        names = [os.path.splitext(os.path.basename(f))[0] for f in files]
        if 'z_bar_10_card' in names:
            times['z_bar_10_card'] = 350.0
        self.check_split(files, 6, times)


def args_for(build, binary, **kw):
    a = argparse.Namespace(build=build, binary=binary, jobs=2, log_prefix='', port_base=18861,
                           serial=set(), env='', skip_rust_only=False)
    for k, v in kw.items():
        setattr(a, k, v)
    return a


class Runs(Scratch):
    def setUp(self):
        super().setUp()
        self.build = os.path.join(self.dir, 'build')
        # The stand-in for the headless binary: leaves a mark in its data root and passes.
        self.binary = os.path.join(self.dir, 'fake-headless')
        with open(self.binary, 'w', encoding='utf-8') as f:
            f.write('#!/bin/sh\necho "$RUN" > "$GRIMOIRE_ROOT/mark"\n')
        os.chmod(self.binary, 0o755)

    def run_batch(self, files, run, **kw):
        args = args_for(self.build, self.binary, **kw)
        out = io.StringIO()
        os.environ['RUN'] = run
        try:
            with contextlib.redirect_stdout(out):
                tasks, skipped = sr.scenario_tasks(args, files, {})
                sr.run_all(tasks, args, set())
                rc = sr.report(tasks, 'SCENARIOS FAILED')
        finally:
            del os.environ['RUN']
        return rc, out.getvalue(), {t.name: t for t in tasks}, skipped

    def test_a_cycle_says_why(self):
        files = sorted([self.qs('e', '# root: f'), self.qs('f', '# root: e')])
        rc, out, tasks, _ = self.run_batch(files, '1')
        self.assertEqual(rc, 1)
        self.assertIn('== e: FAILED: its root f never ran (a cycle of # root: lines)', out)
        self.assertNotIn('(no log)', out)
        self.assertEqual(out.count('never ran (a cycle'), 4)  # a progress line and a log, each

    def test_a_bad_header_leaves_no_old_data_root(self):
        a = self.qs('a')
        b = self.qs('b', '# root: a')
        rc, out, tasks, _ = self.run_batch([a, b], '1')
        self.assertEqual(rc, 0, out)
        self.assertTrue(os.path.isfile(os.path.join(self.build, 'scen', 'b', 'mark')))
        self.qs('a', '# env: BAD')
        rc, out, tasks, _ = self.run_batch([a, b], '2')
        self.assertEqual(rc, 1)
        self.assertFalse(tasks['a'].ok)
        self.assertFalse(os.path.exists(os.path.join(self.build, 'scen', 'a')))
        self.assertFalse(tasks['b'].ok, 'b must not pass on the first run\'s copy of a')
        self.assertIn('could not copy its root a', tasks['b'].message)

    def test_a_rust_only_root_skips_its_dependants_in_any_order(self):
        files = sorted([self.qs('aa_x', '# root: zz_rust'), self.qs('ab_y', '# root: aa_x'),
                        self.qs('zz_rust', '# rust-only'), self.qs('mm_plain')])
        rc, out, tasks, skipped = self.run_batch(files, '1', skip_rust_only=True)
        self.assertEqual(sorted(skipped), ['aa_x', 'ab_y', 'zz_rust'])
        self.assertEqual(sorted(tasks), ['mm_plain'])
        self.assertEqual(rc, 0, out)


if __name__ == '__main__':
    unittest.main()
