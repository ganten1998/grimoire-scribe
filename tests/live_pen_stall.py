#!/usr/bin/env python3
# Fast ink's samples while the main loop is busy: the headless app reads a stand-in raw digitizer
# (a FIFO of input_event records, GRIMOIRE_SIM_RAWPEN_MAIN) as the Kindle's main loop reads the raw
# node, and is held GRIMOIRE_SIM_STALL_MS after each read that brought frames, as an expensive ink
# frame (a textured brush) holds it on the Kindle. A feeder writes a frame every 4 ms, stamped with
# CLOCK_MONOTONIC, to that FIFO and to a second one (GRIMOIRE_SIM_RAWPEN), which a pen viewer reads
# itself, as each reader of an evdev node has a queue of its own. Each sample on /live/pen is timed
# from its frame (its identity is the frame's microseconds) to its arrival here.
#   before: GRIMOIRE_LIVE_PEN_SOURCE=ring, the samples the main loop's input path stores in the ring,
#           which leave only when it reads the pens: in bursts, up to a stall late.
#   after:  the pen viewer's own reader of the node, whatever the main loop does.
# Prints both, and fails unless the viewer's own reader delivers nearly every frame, its 90th
# percentile is under 25 ms, and it beats the ring's median. A raw run that misses is measured once
# more, and the second run decides: a busy machine (CI's macOS runner, issue #56) can keep the
# feeder, the pen viewer or the reader here off the CPU for longer than the main loop is held, once,
# while samples that wait for the main loop wait in every run. Standard library only.
# usage: live_pen_stall.py <port> <grimoire-headless> <out dir>
import http.client, os, re, shutil, signal, struct, subprocess, sys, threading, time

PORT, HEADLESS, OUT = int(sys.argv[1]), sys.argv[2], sys.argv[3]
STALL_MS, FRAME_MS, FRAMES = 60, 4, 500
HERE = os.path.dirname(os.path.abspath(__file__))
fail = 0


def check(name, ok):
    global fail
    print(('ok   ' if ok else 'FAIL ') + name)
    if not ok:
        fail = 1


def get(path, cookie=None, keep=False):
    c = http.client.HTTPConnection('127.0.0.1', PORT, timeout=10)
    c.request('GET', path, headers={'Cookie': cookie} if cookie else {})
    r = c.getresponse()
    if keep:
        return c, r
    body = r.read()
    c.close()
    return r.status, dict(r.getheaders()), body


def mono():
    t = time.clock_gettime_ns(time.CLOCK_MONOTONIC)
    return t // 1_000_000_000, (t // 1000) % 1_000_000


EV = struct.Struct('=QQHHi')  # struct input_event in the host's layout (two unsigned longs, 64-bit)


def frame(sec, usec, *events):
    return b''.join(EV.pack(sec, usec, t, c, v) for t, c, v in events) + EV.pack(sec, usec, 0, 0, 0)


def stats(xs):
    xs = sorted(xs)
    if not xs:
        return None
    return xs[(len(xs) - 1) // 2], xs[int((len(xs) - 1) * 0.9)], xs[-1]


def run(mode, name=None):
    d = os.path.join(OUT, name or mode)
    shutil.rmtree(d, ignore_errors=True)
    os.makedirs(os.path.join(d, 'root'))
    shutil.copytree(os.path.join(HERE, 'fixtures', 'index'), os.path.join(d, 'root'), dirs_exist_ok=True)
    with open(os.path.join(d, 'root', 'settings.ini'), 'a') as f:
        f.write('\nhttp_port=%d\n' % PORT)
    script = open(os.path.join(HERE, 'scenarios', 'live_1_card.qs')).read().split('# live_check holds here')[0]
    with open(os.path.join(d, 'card.qs'), 'w') as f:
        f.write(script + 'hold %s/never\n' % d)
    fifo_main, fifo_viewer = os.path.join(d, 'raw-main'), os.path.join(d, 'raw-viewer')
    os.mkfifo(fifo_main)
    os.mkfifo(fifo_viewer)
    env = {k: v for k, v in os.environ.items() if not k.startswith('GRIMOIRE_')}
    env.update(GRIMOIRE_ROOT=os.path.join(d, 'root'), GRIMOIRE_SCRIPT=os.path.join(d, 'card.qs'),
               GRIMOIRE_SHOTS=os.path.join(d, 'shots'), GRIMOIRE_LANG='en', GRIMOIRE_SIM_ADDRS='en0=127.0.0.1',
               GRIMOIRE_SIM_RAWPEN_MAIN=fifo_main, GRIMOIRE_SIM_RAWPEN=fifo_viewer, GRIMOIRE_SIM_STALL_MS=str(STALL_MS))
    if mode == 'ring':
        env['GRIMOIRE_LIVE_PEN_SOURCE'] = 'ring'
    log = os.path.join(d, 'app.log')
    app = subprocess.Popen([HEADLESS], env=env, stderr=open(log, 'w'), stdout=subprocess.DEVNULL)
    arrivals = []
    try:
        code = None
        for _ in range(200):
            m = re.findall(r'live: \(simulator\) the code is (\d{6})', open(log, errors='replace').read())
            if m:
                code = m[-1]
                break
            time.sleep(0.05)
        if code is None:
            check(mode + ': the card made a code', False)
            return None
        st, hd, _ = get('/live/pair?code=' + code)
        session = hd.get('Set-Cookie', '').split(';')[0]
        conn, r = get('/live/pen', session, keep=True)
        # the hello and its reset, then the feeder starts
        while True:
            n, kind = struct.unpack('<IB', r.read(5))
            payload = r.read(n - 1)
            if kind == 7:
                break

        def reader():
            try:
                while True:
                    head = r.read(5)
                    if len(head) != 5:
                        return
                    n, kind = struct.unpack('<IB', head)
                    payload = r.read(n - 1)
                    if kind != 7:
                        continue
                    now = time.clock_gettime_ns(time.CLOCK_MONOTONIC) // 1000
                    for at in range(8, len(payload), 20):
                        arrivals.append((now, struct.unpack('<I', payload[at:at + 4])[0]))
            except (OSError, http.client.HTTPException, struct.error):
                pass
        t = threading.Thread(target=reader, daemon=True)
        t.start()
        time.sleep(0.3)  # (the viewer has opened its node)
        fds = [os.open(p, os.O_RDWR | os.O_NONBLOCK) for p in (fifo_main, fifo_viewer)]
        sent = set()
        lag = 0.0  # how far the feeder itself fell behind its frames' times (ms): the machine's share
        start = time.monotonic()
        for i in range(FRAMES):
            while time.monotonic() < start + i * FRAME_MS / 1000:
                time.sleep(0.0005)
            lag = max(lag, (time.monotonic() - start) * 1000 - i * FRAME_MS)
            sec, usec = mono()
            x, y = 300 + i, 600 + (i % 50)
            ev = [(3, 0, x), (3, 1, y)]
            if i == 0:
                ev = [(1, 0x140, 1)] + ev
            if i == 50:
                ev += [(1, 0x14a, 1), (3, 0x18, 2000)]
            if i == FRAMES - 50:
                ev += [(1, 0x14a, 0), (3, 0x18, 0)]
            data = frame(sec, usec, *ev)
            for fd in fds:
                try:
                    os.write(fd, data)
                except BlockingIOError:
                    pass  # (the ring's run: nobody reads the viewer's FIFO)
            sent.add((sec * 1_000_000 + usec) & 0xFFFFFFFF)
        time.sleep(STALL_MS / 1000 + 0.5)
        for fd in fds:
            os.close(fd)
        conn.close()
        got = [((now - ident) & 0xFFFFFFFF) / 1000 for now, ident in arrivals if ident in sent]
        return len(sent), len({i for _, i in arrivals if i in sent}), got, lag
    finally:
        app.send_signal(signal.SIGTERM)
        try:
            app.wait(5)
        except subprocess.TimeoutExpired:
            app.kill()


def measure(mode, name=None):
    res = run(mode, name)
    if res is None:
        sys.exit(1)
    n, delivered, delays, _ = res
    s = stats(delays)
    print('     %-5s %d of %d frames delivered; frame to browser p50 / p90 / max %s ms (main loop held %d ms a read)'
          % (mode, delivered, n, ' / '.join('%.1f' % v for v in s) if s else '...', STALL_MS))
    return res


def raw_ok(res, ring):
    n, delivered, delays, _ = res
    raw = stats(delays)
    return delivered >= 0.95 * n and bool(raw) and raw[1] < 25 and bool(ring) and raw[1] < ring[0]


results = {mode: measure(mode) for mode in ('ring', 'raw')}
ring = stats(results['ring'][2])
if not raw_ok(results['raw'], ring):
    print('     (the raw run missed, the feeder itself up to %.1f ms behind its frames: measured once more)'
          % results['raw'][3])
    results['raw'] = measure('raw', 'raw-again')
raw = stats(results['raw'][2])
check('the pen viewer reads the raw node itself: nearly every frame comes (%d of %d)' % results['raw'][1::-1],
      results['raw'][1] >= 0.95 * results['raw'][0])
check('with the main loop busy, its samples still come at once (p90 %.1f ms, under 25)' % (raw[1] if raw else -1),
      bool(raw) and raw[1] < 25)
check('and sooner than the ring\'s, which waits for the main loop (p90 %.1f ms against its median %.1f)'
      % (raw[1] if raw else -1, ring[0] if ring else -1), bool(raw) and bool(ring) and raw[1] < ring[0])
sys.exit(fail)
