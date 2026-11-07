#!/usr/bin/env python3
"""The scripted scenarios' runner: make scenarios, make scenarios-late, make l10n-check and the CI
shards call it (python3 -I tests/scenario_runner.py ...; the Makefile passes every setting).

  run     the scenarios named (tests/scenarios/*.qs), then the runner's self-check
  l10n    make l10n-check: the .mo files against the .po files, then the language runs
  shard   print which scenarios one CI shard runs (the same split `run --shard` makes)
  times   write tests/scenario-times.txt from the last full run's times

Each run is the headless binary against its own data root (build/scen/NAME) and shots folder
(build/shots/NAME), as the Makefile's loop ran them one after another; here up to --jobs of them run
at once. A scenario with a "# root: NAME" line starts only once NAME has finished (its data is
copied first), whatever NAME's result was, as before: a dependant of a failed root runs from what
the root left, and usually fails with it. The order runs start in is longest first, by the timing
table (tests/scenario-times.txt: each scenario's seconds, and a chain counts its dependants'), so
the long ones never start last. Every run of a batch gets a port of its own for the share server
(GRIMOIRE_TEST_PORT, else 8765, plus the job's slot). Each run's output goes to
build/scen-logs/NAME.log; one line per finished run is printed as it ends, and the failed runs' logs
are printed whole at the end, in the order the scenarios were named. A scenario in --serial runs
alone, after the parallel batch (a scenario that depends on how idle the machine is; the Makefile's
SCEN_SERIAL says which, and why).
"""
import argparse
import heapq
import os
import shutil
import subprocess
import sys
import threading
import time

DEFAULT_PORT = 8765
DEFAULT_WEIGHT = 5.0  # seconds, for a scenario the timing table does not know yet
HERE = os.path.dirname(os.path.abspath(__file__))
TIMES_FILE = os.path.join(HERE, 'scenario-times.txt')


class Task:
    """One run of the headless binary, with what is made before it and checked after it."""

    def __init__(self, name, script, root, shots, env, weight=DEFAULT_WEIGHT):
        self.name = name            # how the summary names it (b_library, selftest/fail, es-classic)
        self.script = script        # the .qs file
        self.root = root            # GRIMOIRE_ROOT, made fresh (removed first)
        self.shots = shots          # GRIMOIRE_SHOTS, removed first
        self.env = env              # [(KEY, VALUE)], on top of the caller's environment
        self.weight = weight
        self.from_root = None       # a data root copied in first (build/scen/NAME)
        self.dep = None             # the Task that makes from_root, when it is in this run
        self.ini = []               # settings.ini's first lines, for a fresh root (l10n)
        self.port = False           # gets http_port=<its slot's port> in settings.ini
        self.settings = []          # "# settings:" lines, appended to settings.ini
        self.copies = []            # "# copy: DIR DEST" lines
        self.generate = None        # "# generate:" line
        self.pdfs = None            # "# pdfs: N"
        self.scenario_checks = False  # the *.tmp and # pdfs: checks after the run
        self.stderr_file = None     # stderr to this file (the fail selftests), else to the log
        self.verify = None          # fn(rc) -> [failure messages]; default: rc must be 0
        self.serial = False
        self.mkdir = False          # make the data root even with nothing to put in it (l10n)
        self.message = ''           # the failure, for the progress line
        self.ok = None
        self.seconds = 0.0
        self.log = None
        self.note = ''           # " (the C build's variant)"
        self.key = name


def log_name(name):
    return name.replace('/', '-')


def read_times(path):
    times = {}
    try:
        with open(path, encoding='utf-8') as f:
            for line in f:
                w = line.split()
                if len(w) >= 2 and not line.startswith('#'):
                    try:
                        times[w[0]] = float(w[1])
                    except ValueError:
                        pass
    except OSError:
        pass
    return times


def header(path):
    """The lines of a .qs file the runner reads, as the Makefile's sed and grep read them."""
    h = {'rust_only': False, 'root': None, 'settings': [], 'copies': [], 'generate': None, 'env': [],
         'pdfs': None}
    with open(path, encoding='utf-8', errors='replace') as f:
        for line in f:
            line = line.rstrip('\n')
            if line.startswith('# rust-only'):
                h['rust_only'] = True
            elif line.startswith('# root:'):
                if h['root'] is None:
                    h['root'] = line[len('# root:'):].strip()
            elif line.startswith('# settings: '):
                h['settings'].append(line[len('# settings:'):].lstrip(' '))
            elif line.startswith('# copy: '):
                w = line[len('# copy:'):].strip().split(None, 1)
                if w:
                    h['copies'].append((w[0], w[1].strip() if len(w) > 1 else ''))
            elif line.startswith('# generate:'):
                if h['generate'] is None:
                    h['generate'] = line[len('# generate:'):].strip()
            elif line.startswith('# env:'):
                h['env'] += line[len('# env:'):].split()
            elif line.startswith('# pdfs:'):
                if h['pdfs'] is None:
                    h['pdfs'] = line[len('# pdfs:'):].strip()
    return h


def env_pairs(words):
    out = []
    for w in words:
        k, eq, v = w.partition('=')
        if not eq or not k:
            raise ValueError(f'not KEY=VALUE: {w}')
        out.append((k, v))
    return out


# ---- the scenarios ----

def scenario_tasks(args, files, times):
    """The scenarios' Tasks, in the order named, and the names skipped (SCEN_SKIP_RUST_ONLY)."""
    b = args.build
    tasks, skipped, by_name = [], [], {}
    scen_env = env_pairs(args.env.split())
    # Each file's header (a rust-only one's C variant in its place) or None: rust-only, no variant.
    heads = []
    for s in files:
        n = os.path.splitext(os.path.basename(s))[0]
        note, h = '', header(s)
        if args.skip_rust_only and h['rust_only']:
            variant = os.path.join('tests', 'c-scenarios', n + '.qs')
            if os.path.isfile(variant):
                s, note = variant, " (the C build's variant)"
                h = header(s)
            else:
                h = None
        heads.append((n, s, note, h))
    # A scenario whose root is skipped is skipped too (it would copy a data root never made), in
    # whatever order the files sort, and its own dependants with it.
    gone = {n for n, _, _, h in heads if h is None}
    more = True
    while more:
        more = False
        for n, _, _, h in heads:
            if h is not None and n not in gone and h['root'] in gone:
                gone.add(n)
                more = True
    for n, s, note, h in heads:
        if h is None:
            print(f'== {n} (rust-only: skipped)')
            skipped.append(n)
            continue
        if n in gone:
            print(f"== {n} (its root {h['root']} is rust-only: skipped)")
            skipped.append(n)
            continue
        try:
            envs = env_pairs(h['env'])
        except ValueError as e:
            envs = None
            bad = str(e)
        t = Task(n, s, f'{b}/scen/{n}', f'{b}/shots/{n}',
                 [('GRIMOIRE_LANG', 'en')] + scen_env + (envs or []), times.get(n, DEFAULT_WEIGHT))
        t.note = note
        if envs is None:
            t.message = f'a # env: word is {bad}'
        t.from_root = h['root']
        t.port = True
        t.settings, t.copies, t.generate, t.pdfs = h['settings'], h['copies'], h['generate'], h['pdfs']
        t.scenario_checks = True
        t.serial = n in args.serial
        by_name[n] = t
        tasks.append(t)
    for t in tasks:
        if t.from_root and t.from_root in by_name and by_name[t.from_root] is not t:
            t.dep = by_name[t.from_root]
    return tasks, skipped


def selftest_tasks(args):
    """make scenarios' check of the runner itself (the Makefile's comment above the rule)."""
    b, d = args.build, f'{args.build}/selftest'
    shutil.rmtree(d, ignore_errors=True)
    os.makedirs(d, exist_ok=True)
    lang = [('GRIMOIRE_LANG', 'en')]
    out = []

    def pass_task(name, script, need_pdf):
        t = Task(f'selftest/{name}', script, f'{d}/{name}', f'{d}/shots/{name}', lang, 3.0)

        def verify(rc):
            msgs = [] if rc == 0 else [f'exit {rc}']
            if need_pdf and not any(f.endswith('.pdf') for f in listdir(f'{d}/{name}/PDFs')):
                msgs.append(f'selftest {name}: the run ended before its PDF export')
            return msgs
        t.verify = verify
        return t

    def fail_task(name, script):
        t = Task(f'selftest/{name}', script, f'{d}/{name}', f'{d}/shots/{name}', lang, 3.0)
        t.stderr_file = f'{d}/{name}.log'
        # Every line but the comments, waits and taps must be reported, by its number, and the
        # run must exit with status 3.
        want = []
        with open(script, encoding='utf-8') as f:
            for i, line in enumerate(f, 1):
                w = line.split()
                if w and not w[0].startswith('#') and w[0] not in ('wait', 'tap'):
                    want.append(i)

        def verify(rc):
            got = []
            with open(t.stderr_file, encoding='utf-8', errors='replace') as f:
                text = f.read()
            for line in text.splitlines():
                if line.startswith('expect failed (line '):
                    num = line[len('expect failed (line '):].split(')')[0]
                    if num.isdigit():
                        got.append(int(num))
            if rc != 3 or got != want:
                return [text.rstrip('\n'),
                        f'selftest {name}: exit {rc} with lines{fmt_lines(got)} failed, '
                        f'wanted exit 3 with lines{fmt_lines(want)}']
            return []
        t.verify = verify
        return t

    out.append(pass_task('pass', 'tests/selftest/pass-c.qs' if args.skip_rust_only else 'tests/selftest/pass.qs', True))
    out.append(fail_task('fail', 'tests/selftest/fail.qs'))
    if not args.skip_rust_only:
        out.append(pass_task('pass-rust', 'tests/selftest/pass-rust.qs', False))
        out.append(fail_task('fail-rust', 'tests/selftest/fail-rust.qs'))
    return out


def fmt_lines(nums):
    return ''.join(f' {n}' for n in nums)


def listdir(path):
    try:
        return os.listdir(path)
    except OSError:
        return []


# ---- one run ----

def prepare(t, port, log):
    """Makes the run's data root as the Makefile's loop did; False (the reason logged) on failure."""
    shutil.rmtree(t.root, ignore_errors=True)
    shutil.rmtree(t.shots, ignore_errors=True)
    if t.from_root:
        src = os.path.join(os.path.dirname(t.root), t.from_root)
        if subprocess.call(['cp', '-R', src, t.root], stdout=log, stderr=subprocess.STDOUT) != 0:
            t.message = f'could not copy its root {t.from_root}'
            return False
    if t.mkdir or t.ini or t.port or t.settings or t.copies:
        os.makedirs(t.root, exist_ok=True)
    ini = os.path.join(t.root, 'settings.ini')
    if t.ini:
        with open(ini, 'w', encoding='utf-8') as f:
            f.write(''.join(line + '\n' for line in t.ini))
    if t.port and port is not None:
        try:
            with open(ini, encoding='utf-8', errors='surrogateescape') as f:
                lines = [ln for ln in f.read().splitlines() if not ln.startswith('http_port=')]
        except OSError:
            lines = []
        with open(ini + '.port', 'w', encoding='utf-8', errors='surrogateescape') as f:
            f.write(''.join(ln + '\n' for ln in lines) + f'http_port={port}\n')
        os.replace(ini + '.port', ini)
    if t.settings:
        with open(ini, 'a', encoding='utf-8') as f:
            f.write(''.join(line + '\n' for line in t.settings))
    for src, dst in t.copies:
        dest = os.path.join(t.root, dst) if dst else t.root
        os.makedirs(dest, exist_ok=True)
        if subprocess.call(['cp', '-R', src + '/.', dest + '/'], stdout=log, stderr=subprocess.STDOUT) != 0:
            t.message = 'a # copy: line failed'
            return False
    if t.generate:
        if subprocess.call(['python3'] + t.generate.split() + [t.root], stdout=log, stderr=subprocess.STDOUT) != 0:
            t.message = f'its generator failed ({t.generate})'
            return False
    return True


def execute(t, args, port, hooks):
    os.makedirs(os.path.dirname(t.log), exist_ok=True)
    started = time.monotonic()
    msgs = []
    with open(t.log, 'w', encoding='utf-8', errors='replace') as log:
        log.write(f'== {t.name}{t.note} ({t.script}' + (f', port {port}' if t.port and port is not None else '') + ')\n')
        log.flush()
        if t.message:  # (a bad header: nothing runs, and nothing of an earlier run is left)
            shutil.rmtree(t.root, ignore_errors=True)
            shutil.rmtree(t.shots, ignore_errors=True)
            msgs.append(t.message)
        elif not prepare(t, port, log):
            msgs.append(t.message)
        else:
            env = {k: v for k, v in os.environ.items() if k not in hooks}
            for k, v in t.env:
                env[k] = v
            env.update(GRIMOIRE_ROOT=t.root, GRIMOIRE_SCRIPT=t.script, GRIMOIRE_SHOTS=t.shots)
            err = open(t.stderr_file, 'w') if t.stderr_file else subprocess.STDOUT
            try:
                rc = subprocess.call([args.binary], env=env, stdin=subprocess.DEVNULL, stdout=log, stderr=err)
            finally:
                if t.stderr_file:
                    err.close()
            if t.verify:
                msgs += t.verify(rc)
            elif rc != 0:
                msgs.append(f'exit {rc}')
            if t.scenario_checks:
                n = t.key
                for dirpath, dirnames, filenames in os.walk(t.root):
                    if any(x.endswith('.tmp') for x in dirnames + filenames):
                        msgs.append(f'{n} left a file half-written')
                        break
                if t.pdfs is not None:
                    got = 0
                    for dirpath, dirnames, filenames in os.walk(os.path.join(t.root, 'PDFs')):
                        got += sum(1 for x in dirnames + filenames if x.endswith('.pdf'))
                    if str(got) != t.pdfs:
                        msgs.append(f'{n} left {got} PDFs in PDFs/, wanted {t.pdfs}')
        for m in msgs:
            log.write(m + '\n')
    t.seconds = time.monotonic() - started
    t.ok = not msgs
    t.message = '; '.join(m.splitlines()[-1] for m in msgs if m) if msgs else ''


# ---- the schedule ----

def run_all(tasks, args, hooks):
    """Runs every task, at most args.jobs at once, roots before their dependants, the serial ones
    alone after the rest; returns once all have finished."""
    jobs = max(1, args.jobs)
    for t in tasks:
        t.log = os.path.join(args.build, 'scen-logs', args.log_prefix + log_name(t.name) + '.log')
    deps = {id(t): [] for t in tasks}
    for t in tasks:
        if t.dep is not None:
            deps[id(t.dep)].append(t)
    # A run's priority: its own time and the longest chain of dependants after it.
    prio = {}

    def chain(t, seen=()):
        if id(t) in prio:
            return prio[id(t)]
        if id(t) in seen:
            return t.weight
        prio[id(t)] = t.weight + max((chain(d, seen + (id(t),)) for d in deps[id(t)]), default=0.0)
        return prio[id(t)]
    for t in tasks:
        chain(t)
    order = {id(t): i for i, t in enumerate(tasks)}
    ready, serial_ready = [], []

    def push(t):
        heapq.heappush(serial_ready if t.serial else ready, (-prio[id(t)], order[id(t)], t))
    for t in tasks:
        if t.dep is None:
            push(t)
    cond = threading.Condition()
    free = list(range(jobs))
    finished = []
    running = 0
    total = len(tasks)
    done = 0
    port_base = args.port_base

    def worker(t, slot):
        port = port_base + slot if port_base is not None else None
        try:
            execute(t, args, port, hooks)
        except Exception as e:  # (the runner's own error: report it as the run's failure)
            t.ok, t.message = False, f'runner error: {e!r}'
        with cond:
            finished.append((t, slot))
            cond.notify()

    serial_running = False
    with cond:
        while done < total:
            # Start what can start: the parallel batch first; a serial run only once nothing else
            # runs or is ready, and nothing starts beside it.
            while free and ready and not serial_running:
                _, _, t = heapq.heappop(ready)
                running += 1
                threading.Thread(target=worker, args=(t, free.pop(0)), daemon=True).start()
            if running == 0 and not ready and serial_ready:
                _, _, t = heapq.heappop(serial_ready)
                running += 1
                serial_running = True
                free.sort()
                threading.Thread(target=worker, args=(t, free.pop(0)), daemon=True).start()
            if running == 0:
                # Nothing runs and nothing can start: the rest wait for a root that never runs.
                for t in tasks:
                    if t.ok is None:
                        t.ok, t.message = False, f'its root {t.from_root} never ran (a cycle of # root: lines)'
                        done += 1
                        os.makedirs(os.path.dirname(t.log), exist_ok=True)
                        with open(t.log, 'w', encoding='utf-8') as f:
                            f.write(f'== {t.name}{t.note} ({t.script})\n{t.message}\n')
                        print(f'== {args.log_prefix}{t.name}{t.note}: FAILED: {t.message}', flush=True)
                break
            while not finished:
                cond.wait()
            while finished:
                t, slot = finished.pop()
                running -= 1
                done += 1
                free.append(slot)
                if t.serial:
                    serial_running = False
                status = f'ok ({t.seconds:.1f} s)' if t.ok else f'FAILED ({t.seconds:.1f} s): {t.message}'
                print(f'== {args.log_prefix}{t.name}{t.note}: {status}', flush=True)
                for d in deps[id(t)]:
                    push(d)


def report(tasks, label, before=()):
    failed = [t for t in tasks if not t.ok]
    for t in failed:
        print(f'\n==== {t.name}: the whole log ({t.log}) ====')
        try:
            with open(t.log, encoding='utf-8', errors='replace') as f:
                sys.stdout.write(f.read())
        except OSError:
            print(t.message or '(no log)')
    sys.stdout.flush()
    if failed or before:
        print(f'{label}:' + ''.join(' ' + n for n in list(before) + [t.name for t in failed]))
        return 1
    return 0


# ---- shards (CI) ----

def shard_select(files, shards, index, times):
    """The scenarios of shard `index` (1-based) of `shards`: whole dependency trees, balanced by
    the timing table; a tree too big for one shard is split below its root, and a part still too
    big below its own top, and so on, the roots above a part going with it, so a dependant always
    finds its root's data in its own shard. Scenarios in a cycle of # root: lines go to shard 1,
    whose runner reports them."""
    names = [os.path.splitext(os.path.basename(f))[0] for f in files]
    path = dict(zip(names, files))
    root = {n: header(path[n])['root'] for n in names}
    root = {n: r if r in path else None for n, r in root.items()}
    kids = {n: [] for n in names}
    for n in names:
        if root[n]:
            kids[root[n]].append(n)
    w = {n: times.get(n, DEFAULT_WEIGHT) for n in names}

    def subtree(n):
        out = [n]
        for k in kids[n]:
            out += subtree(k)
        return out

    target = sum(w.values()) / shards
    # Units: (lead, top), running lead (top's own roots, topmost first) and top's whole tree. A unit
    # over the target whose top has dependants is split into one unit per dependant, top joining
    # the lead; each split goes one level deeper, so the loop ends (at the leaves at worst).
    def runs(u):
        return u[0] + subtree(u[1])

    def weight(u):
        return sum(w[x] for x in runs(u))
    units = [([], n) for n in names if not root[n]]
    changed = True
    while changed:
        changed = False
        for u in sorted(units, key=lambda u: -weight(u)):
            lead, top = u
            if weight(u) > target and kids[top]:
                units.remove(u)
                units += [(lead + [top], k) for k in kids[top]]
                changed = True
                break
    units = [runs(u) for u in units]
    bins = [set() for _ in range(shards)]
    load = [0.0] * shards
    for u in sorted(units, key=lambda u: (-sum(w[x] for x in u), u[-1])):
        best = min(range(shards), key=lambda i: (load[i] + sum(w[x] for x in u if x not in bins[i]), i))
        load[best] += sum(w[x] for x in u if x not in bins[best])
        bins[best].update(u)
    placed = set().union(*bins)
    bins[0].update(n for n in names if n not in placed)
    return [path[n] for n in names if n in bins[index - 1]], load


# ---- the languages (make l10n-check) ----

# The Classic fonts besides Terminess (rust/src/font.rs's FONT_FAMILIES, by settings.ini id), and the
# languages every screen is checked in each of.
CLASSIC_FONTS = ('atkinson', 'noto-sans', 'literata')
FONT_LANGS = ('en', 'es')

def l10n(args, hooks):
    failed = []
    if shutil.which('msgfmt'):
        os.makedirs(f'{args.build}/l10n', exist_ok=True)
        for p in sorted(x for x in listdir('po') if x.endswith('.po')):
            p = f'po/{p}'
            m = p[:-3] + '.mo'
            t = f'{args.build}/l10n/{os.path.basename(m)}'
            if subprocess.call(['msgfmt'] + args.msgfmt_flags.split() + ['-o', t, p]) != 0:
                print(f'l10n: {p} does not compile')
                failed.append(p)
            elif not same_file(t, m):
                print(f'l10n: {m} is not what msgfmt makes of {p} now (run make l10n)')
                failed.append(m)
            else:
                print(f'l10n: {m} is what msgfmt makes of {p}')
    else:
        print('l10n: skipped the check that po/*.mo match po/*.po (msgfmt is not installed)')
    b = args.build
    langs = args.langs.split()
    # The styles, and the settings.ini lines each runs with: Gothic and Classic (in Terminess, its default
    # font), then Classic in each other font of Settings > Font (2.1, the Rust build's), in English and
    # Spanish (Japanese takes its letters from DotGothic16 in every font).
    inis = {'gothic': ['theme=gothic'], 'classic': ['theme=classic']}
    for font in CLASSIC_FONTS:
        inis[f'classic-{font}'] = ['theme=classic', f'font={font}']

    def styles_for(lang):
        return [st for st in inis
                if not st.startswith('classic-') or (lang in FONT_LANGS and not args.skip_rust_only)]
    tasks = []

    def task(n, script, root, env, ini, weight):
        t = Task(n, script, root, f'{b}/shots/l10n/{n}', env, weight)
        t.ini, t.port, t.mkdir = ini, True, True
        tasks.append(t)
        return t
    times = read_times(args.times)
    w_all = times.get('y_all_screens', DEFAULT_WEIGHT)
    if not args.skip_rust_only:
        for lang in langs:
            for style in styles_for(lang):
                n = f'{lang}-{style}'
                if n in args.skip:
                    print(f'== l10n {n}: skipped (make scenarios ran {args.scenario} in it)')
                    continue
                task(n, args.scenario, f'{b}/scen/l10n/{n}',
                     [('GRIMOIRE_LANG', lang), ('GRIMOIRE_CHECK_FIT', '1'), ('GRIMOIRE_SIM_ADDRS', 'wlan0=192.168.0.20')],
                     inis[style], w_all)
        for lang in langs:
            for style in styles_for(lang):
                n = f'shapes-{lang}-{style}'
                task(n, 'tests/l10n/shapes.qs', f'{b}/scen/l10n/{n}',
                     [('GRIMOIRE_LANG', lang), ('GRIMOIRE_CHECK_FIT', '1')], inis[style], 10.0)
    else:
        print(f'l10n: skipped {args.scenario} (rust-only: it visits the Rust build\'s screens)')
    shutil.rmtree(f'{b}/scen/l10n/long-root', ignore_errors=True)
    for lang in langs:
        n = f'long-root-{lang}'
        task(n, 'tests/l10n/long_root.qs', f'{b}/scen/l10n/long-root/' + 'a-long-data-root-' * 8 + lang,
             [('GRIMOIRE_LANG', lang), ('GRIMOIRE_CHECK_FIT', '1')], [], 5.0)
    for lang in langs:
        for style in styles_for(lang):
            n = f'hosted-{lang}-{style}'
            task(n, 'tests/l10n/hosted.qs', f'{b}/scen/l10n/{n}',
                 [('GRIMOIRE_LANG', lang), ('GRIMOIRE_CHECK_FIT', '1'), ('GRIMOIRE_HOST', 'koreader')],
                 inis[style], 5.0)
    copies = header(args.templates)['copies']
    for lang in langs:
        for style in styles_for(lang):
            n = f'templates-{lang}-{style}'
            t = task(n, args.templates, f'{b}/scen/l10n/{n}',
                     [('GRIMOIRE_LANG', lang), ('GRIMOIRE_CHECK_FIT', '1')], inis[style], 8.0)
            t.copies = copies
    if not args.skip_rust_only:
        # Settings > Font (2.1) in every language and in each font, Terminess too: its chip, and the sheet
        # with each family highlighted.
        for lang in langs:
            for style in ['classic'] + [f'classic-{f}' for f in CLASSIC_FONTS]:
                n = f'fonts-{lang}-{style}'
                task(n, 'tests/l10n/fonts.qs', f'{b}/scen/l10n/{n}',
                     [('GRIMOIRE_LANG', lang), ('GRIMOIRE_CHECK_FIT', '1')], inis[style], 4.0)
        # The unified Index and the cross-notebook selector (tests/index_l10n_check.py was these),
        # each from its fixture, then the Wi-Fi controls with and without an address
        # (tests/wifi_l10n_check.py was these), in every language and style.
        fixtures = {'z_page_management': 'page_management_fixture.py',
                    'z_links_1_backlinks': 'notebook_backlinks_fixture.py'}
        for lang in langs:
            for style in styles_for(lang):
                for case in ('z_index_unified', 'z_notebook_links', 'z_page_management', 'z_links_1_backlinks'):
                    n = f'index-{case}-{lang}-{style}'
                    t = task(n, f'tests/scenarios/{case}.qs', f'{b}/scen/l10n/{n}',
                             [('GRIMOIRE_LANG', lang), ('GRIMOIRE_CHECK_FIT', '1')], inis[style], 4.0)
                    t.generate = 'tests/' + fixtures.get(case, 'notebook_links_fixture.py')
        for lang in langs:
            for style in styles_for(lang):
                for online in (0, 1):
                    n = f'wifi-{lang}-{style}-{online}'
                    task(n, 'tests/scenarios/z_wifi.qs', f'{b}/scen/l10n/{n}',
                         [('GRIMOIRE_LANG', lang), ('GRIMOIRE_CHECK_FIT', '1'),
                          ('GRIMOIRE_WIFI_SCRIPT', os.path.abspath('tests/fixtures/wifi/helper.sh')),
                          ('GRIMOIRE_SIM_ADDRS', 'wlan0=192.168.0.20' if online else '')],
                         inis[style], 4.0)
    for t in tasks:
        t.serial = t.name in args.serial
    run_all(tasks, args, hooks)
    # (the msgfmt failures first, as the Makefile's loop named them)
    if report(tasks, 'L10N CHECK FAILED', failed):
        return 1
    print(f'l10n-check: every language fits ({" ".join(langs)})')
    return 0


def same_file(a, b):
    try:
        with open(a, 'rb') as fa, open(b, 'rb') as fb:
            return fa.read() == fb.read()
    except OSError:
        return False


# ---- main ----

def common(p):
    p.add_argument('--build', default='build')
    p.add_argument('--binary', help='the headless binary (default BUILD/host/grimoire-headless)')
    p.add_argument('--jobs', default='', help='runs at once (default: the number of CPUs)')
    p.add_argument('--unset', default='', help="test hooks the caller's environment must not pass on")
    p.add_argument('--serial', default='', help='scenarios run alone, after the parallel batch')
    p.add_argument('--times', default=TIMES_FILE, help='the timing table')
    p.add_argument('--skip-rust-only', action='store_true')
    p.add_argument('--log-prefix', default='', help=argparse.SUPPRESS)


def main():
    ap = argparse.ArgumentParser(description=__doc__.split('\n\n')[0])
    sub = ap.add_subparsers(dest='cmd', required=True)
    r = sub.add_parser('run')
    common(r)
    r.add_argument('--env', default='', help='KEY=VALUE words every run gets (SCEN_ENV)')
    r.add_argument('--no-selftest', action='store_true')
    r.add_argument('--shard', default='', help='I/N: run only shard I (from 1) of N')
    r.add_argument('--times-out', default='', help="write this run's times here")
    r.add_argument('files', nargs='*')
    lp = sub.add_parser('l10n')
    common(lp)
    lp.add_argument('--langs', required=True)
    lp.add_argument('--scenario', required=True)
    lp.add_argument('--templates', required=True)
    lp.add_argument('--msgfmt-flags', default='')
    lp.add_argument('--skip', default='', help='language runs of --scenario to leave out (en-gothic)')
    sp = sub.add_parser('shard')
    sp.add_argument('--times', default=TIMES_FILE)
    sp.add_argument('spec', help='I/N')
    sp.add_argument('files', nargs='*')
    tp = sub.add_parser('times')
    tp.add_argument('--build', default='build')
    tp.add_argument('--out', default=TIMES_FILE)
    a = ap.parse_args()

    if a.cmd == 'shard':
        i, n = (int(x) for x in a.spec.split('/'))
        files, load = shard_select(sorted(a.files), n, i, read_times(a.times))
        print(' '.join(files))
        return 0
    if a.cmd == 'times':
        got = read_times(os.path.join(a.build, 'scen-times.txt'))
        if not got:
            print(f'times: {a.build}/scen-times.txt has no times (run make scenarios first)')
            return 1
        old = read_times(a.out)
        old.update(got)
        # (a scenario that is gone leaves the table)
        old = {k: v for k, v in old.items() if os.path.isfile(os.path.join('tests', 'scenarios', k + '.qs'))}
        with open(a.out, 'w', encoding='utf-8') as f:
            f.write('# Seconds each scenario took in a full parallel make scenarios on the maintainer\'s Mac\n'
                    '# (make scenario-times writes it from the last run; the runner starts the longest\n'
                    '# first and the CI shards are balanced by it; a scenario missing here counts '
                    f'{DEFAULT_WEIGHT:g} s).\n')
            for k in sorted(old):
                f.write(f'{k} {old[k]:.1f}\n')
        print(f'times: {len(got)} scenarios written to {a.out}')
        return 0

    a.jobs = int(a.jobs) if str(a.jobs).strip() else (os.cpu_count() or 1)
    a.binary = a.binary or os.path.join(a.build, 'host', 'grimoire-headless')
    a.serial = set(a.serial.split())
    hooks = set(a.unset.split())
    env_port = os.environ.get('GRIMOIRE_TEST_PORT', '')
    # A port per slot: GRIMOIRE_TEST_PORT's (or the app's own default, 8765) and on.
    a.port_base = int(env_port) if env_port else DEFAULT_PORT
    started = time.monotonic()
    if a.cmd == 'l10n':
        a.skip = set(a.skip.split())
        a.log_prefix = 'l10n/'
        rc = l10n(a, hooks)
        print(f'l10n-check: {time.monotonic() - started:.0f} s, {a.jobs} at a time')
        return rc

    times = read_times(a.times)
    files = a.files
    if a.shard:
        i, n = (int(x) for x in a.shard.split('/'))
        files, _ = shard_select(files, n, i, times)
        print(f'scenarios: shard {i} of {n}: {len(files)} scenarios')
    tasks, skipped = scenario_tasks(a, files, times)
    if not a.no_selftest and (not a.shard or a.shard.split('/')[0] == '1'):
        tasks += selftest_tasks(a)
    ports = f', ports {a.port_base} to {a.port_base + a.jobs - 1}' if a.port_base is not None else ''
    print(f'scenarios: {len(tasks)} runs, {a.jobs} at a time{ports}' +
          (f'; alone after them: {" ".join(sorted(a.serial))}' if a.serial else ''), flush=True)
    run_all(tasks, a, hooks)
    if a.times_out:
        with open(a.times_out, 'w', encoding='utf-8') as f:
            for t in sorted(tasks, key=lambda t: t.key):
                if not t.key.startswith('selftest/'):
                    f.write(f'{t.key} {t.seconds:.1f} {"ok" if t.ok else "failed"}\n')
    print(f'scenarios: {time.monotonic() - started:.0f} s for {len(tasks)} runs, {a.jobs} at a time')
    return report(tasks, 'SCENARIOS FAILED')


if __name__ == '__main__':
    sys.exit(main())
