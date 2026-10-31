#!/usr/bin/env python3
# Live view on a busy link: a browser that reads its tile stream slowly (about 50 KB/s: 1 KiB every
# 20 ms) while the Kindle turns again and again, each turn a hello and the whole screen. A pen
# viewer reads its own stand-in raw digitizer meanwhile (GRIMOIRE_SIM_RAWPEN, as tests/
# live_pen_stall.py feeds it: a hovering frame every 4 ms, stamped with CLOCK_MONOTONIC), and each
# sample is timed from its frame to its arrival here. Once the turns are over the browser reads at
# full speed again.
# Fails unless the pen's samples kept coming at once (nearly every frame, p90 under 25 ms), the tile
# stream lived through it, and the picture it ends with is the app's last screen (its shot, but
# for the overlays never in the frame: the live chip and the clock and battery corner). Prints the
# tile bytes the turns cost and each batch's age (the Kindle's ms against its arrival, less the
# smallest such gap); on macOS's loopback the browser's own receive buffer grows to hundreds of KiB
# whatever SO_RCVBUF says, so that age is mostly that buffer's and is not checked here (the app's
# side of it is: rust/tests/test_live.rs, a_slow_browser_gets_the_newest_tiles_and_the_pen_never_waits).
# Standard library only.
# usage: live_slow_reader.py <port> <grimoire-headless> <out dir>
import http.client, itertools, os, re, select, shutil, signal, socket, struct, subprocess, sys, threading, time, zlib

PORT, HEADLESS, OUT = int(sys.argv[1]), sys.argv[2], sys.argv[3]
TURNS, TURN_MS, FRAME_MS = 12, 250, 4
SLOW_BYTES, SLOW_MS = 1024, 20
HERE = os.path.dirname(os.path.abspath(__file__))
fail = 0


def check(name, ok):
    global fail
    print(('ok   ' if ok else 'FAIL ') + name)
    if not ok:
        fail = 1


def get(path, cookie=None):
    c = http.client.HTTPConnection('127.0.0.1', PORT, timeout=10)
    c.request('GET', path, headers={'Cookie': cookie} if cookie else {})
    r = c.getresponse()
    body = r.read()
    c.close()
    return r.status, dict(r.getheaders()), body


def mono_us():
    return time.clock_gettime_ns(time.CLOCK_MONOTONIC) // 1000


EV = struct.Struct('=QQHHi')  # struct input_event in the host's layout (two unsigned longs, 64-bit)


def frame(t_us, *events):
    sec, usec = t_us // 1_000_000, t_us % 1_000_000
    return b''.join(EV.pack(sec, usec, t, c, v) for t, c, v in events) + EV.pack(sec, usec, 0, 0, 0)


def stats(xs):
    xs = sorted(xs)
    if not xs:
        return None
    return xs[(len(xs) - 1) // 2], xs[int((len(xs) - 1) * 0.9)], xs[-1]


# A PNG of 8-bit gray, as stb writes them: (w, h, rows).
def png_gray(data):
    at, idat, w, h = 8, b'', 0, 0
    while at < len(data):
        n, kind = struct.unpack('>I4s', data[at:at + 8])
        chunk = data[at + 8:at + 8 + n]
        if kind == b'IHDR':
            w, h, depth, ctype = struct.unpack('>IIBB', chunk[:10])
            assert depth == 8 and ctype == 0, (depth, ctype)
        elif kind == b'IDAT':
            idat += chunk
        at += 12 + n
    raw, rows, prev = zlib.decompress(idat), [], bytearray(w)
    for y in range(h):
        f, line = raw[y * (w + 1)], bytearray(raw[y * (w + 1) + 1:(y + 1) * (w + 1)])
        if f == 0 or f == 1:  # (stb's forced filter: each byte the sum of the ones before, mod 256)
            if f == 1:
                line = bytearray(v & 255 for v in itertools.accumulate(line))
            rows.append(bytes(line))
            prev = line
            continue
        for x in range(w):
            a = line[x - 1] if x else 0
            b, c = prev[x], prev[x - 1] if x else 0
            if f == 1: line[x] = (line[x] + a) & 255
            elif f == 2: line[x] = (line[x] + b) & 255
            elif f == 3: line[x] = (line[x] + (a + b) // 2) & 255
            elif f == 4:
                p = a + b - c
                pa, pb, pc = abs(p - a), abs(p - b), abs(p - c)
                line[x] = (line[x] + (a if pa <= pb and pa <= pc else b if pb <= pc else c)) & 255
        rows.append(bytes(line))
        prev = line
    return w, h, rows


# A stream read over a plain socket, so the test decides how fast: its frames from what was read.
class Stream:
    def __init__(self, path, session):
        self.s = socket.create_connection(('127.0.0.1', PORT), timeout=10)
        self.s.sendall(b'GET %s HTTP/1.0\r\nCookie: %s\r\n\r\n' % (path.encode(), session.encode()))
        self.buf = b''
        while b'\r\n\r\n' not in self.buf:
            self.buf += self.s.recv(4096)
        head, self.buf = self.buf.split(b'\r\n\r\n', 1)
        self.status = int(head.split()[1])

    def read(self, n=65536):
        try:
            more = self.s.recv(n)
        except (socket.timeout, OSError):
            return False
        self.buf += more
        return bool(more)

    def frames(self):
        out = []
        while len(self.buf) >= 5:
            n = struct.unpack('<I', self.buf[:4])[0]
            if len(self.buf) < 4 + n:
                break
            out.append((self.buf[4], self.buf[5:4 + n]))
            self.buf = self.buf[4 + n:]
        return out


d = OUT
shutil.rmtree(d, ignore_errors=True)
os.makedirs(os.path.join(d, 'root'))
shutil.copytree(os.path.join(HERE, 'fixtures', 'index'), os.path.join(d, 'root'), dirs_exist_ok=True)
with open(os.path.join(d, 'root', 'settings.ini'), 'a') as f:
    f.write('\nhttp_port=%d\n' % PORT)
script = open(os.path.join(HERE, 'scenarios', 'live_1_card.qs')).read().split('# live_check holds here')[0]
turns = ''.join('rotate %d\nwait %d\n' % ((i + 1) % 2, TURN_MS) for i in range(TURNS))
with open(os.path.join(d, 'card.qs'), 'w') as f:
    # paired: the card's Done, the library; then the turns, the last screen's shot, and a hold
    f.write(script + 'hold %s/paired\ntap 1538 2100\nwait 500\nhold %s/go\n%swait 500\nshot final\nhold %s/never\n'
            % (d, d, turns, d))
fifo_main, fifo_viewer = os.path.join(d, 'raw-main'), os.path.join(d, 'raw-viewer')
os.mkfifo(fifo_main)
os.mkfifo(fifo_viewer)
env = {k: v for k, v in os.environ.items() if not k.startswith('GRIMOIRE_')}
env.update(GRIMOIRE_ROOT=os.path.join(d, 'root'), GRIMOIRE_SCRIPT=os.path.join(d, 'card.qs'),
           GRIMOIRE_SHOTS=os.path.join(d, 'shots'), GRIMOIRE_LANG='en', GRIMOIRE_SIM_ADDRS='en0=127.0.0.1',
           GRIMOIRE_SIM_RAWPEN_MAIN=fifo_main, GRIMOIRE_SIM_RAWPEN=fifo_viewer)
log = os.path.join(d, 'app.log')
app = subprocess.Popen([HEADLESS], env=env, stderr=open(log, 'w'), stdout=subprocess.DEVNULL)
arrivals, sent = [], set()
stop = threading.Event()
try:
    code = None
    for _ in range(200):
        m = re.findall(r'live: \(simulator\) the code is (\d{6})', open(log, errors='replace').read())
        if m:
            code = m[-1]
            break
        time.sleep(0.05)
    if code is None:
        check('the card made a code', False)
        sys.exit(1)
    st, hd, _ = get('/live/pair?code=' + code)
    session = hd.get('Set-Cookie', '').split(';')[0]
    tiles = Stream('/live/stream', session)
    pen = Stream('/live/pen', session)
    check('both streams answer', tiles.status == 200 and pen.status == 200)
    open(os.path.join(d, 'paired'), 'w').close()

    # The picture as the browser has it (the tiles since the last hello, drawn at the end), and what
    # the stream cost.
    size, screen, kept = None, None, []
    hellos, batches, tile_bytes, gaps, layers = 0, [], 0, [], {}

    def take(fr):
        global size, kept, hellos, tile_bytes
        now = time.monotonic() * 1000
        for kind, body in fr:
            if kind == 1:
                _, w, h, _ = struct.unpack('<HHHH', body[:8])
                size, kept, hellos = (w, h), [], hellos + 1
            elif kind == 2:
                kept.append(bytes(body))
                tile_bytes += 5 + len(body)
            elif kind == 3:
                batches.append(now)
                gaps.append(now - struct.unpack('<I', body[4:8])[0])
            elif kind == 9:
                layers[body[0]] = struct.unpack('<HHHH', body[2:10])

    # At full speed until the library's screen is in.
    t0 = time.monotonic()
    while time.monotonic() - t0 < 1.5:
        if select.select([tiles.s], [], [], 0.05)[0]:
            tiles.read()
        take(tiles.frames())
    calm = len(gaps)

    def pen_reader():
        while not stop.is_set():
            if not select.select([pen.s], [], [], 0.1)[0]:
                continue
            if not pen.read():
                return
            now = mono_us()
            for kind, body in pen.frames():
                if kind == 7:
                    for at in range(8, len(body), 20):
                        arrivals.append((now, struct.unpack('<I', body[at:at + 4])[0]))

    def feeder():
        fds = [os.open(p, os.O_RDWR | os.O_NONBLOCK) for p in (fifo_main, fifo_viewer)]
        i = 0
        start = time.monotonic()
        while not stop.is_set():
            while time.monotonic() < start + i * FRAME_MS / 1000:
                time.sleep(0.0005)
            t = mono_us()
            ev = [(3, 0, 300 + i % 1000), (3, 1, 600 + i % 50)]  # hovering: never a touch on the UI
            if i == 0:
                ev = [(1, 0x140, 1)] + ev
            data = frame(t, *ev)
            for fd in fds:
                try:
                    os.write(fd, data)
                except BlockingIOError:
                    pass
            sent.add(t & 0xFFFFFFFF)
            i += 1
        for fd in fds:
            os.close(fd)

    threads = [threading.Thread(target=pen_reader, daemon=True), threading.Thread(target=feeder, daemon=True)]
    for t in threads:
        t.start()
    time.sleep(0.3)
    # The turns, read slowly.
    open(os.path.join(d, 'go'), 'w').close()
    shot = os.path.join(d, 'shots', 'final.png')
    turned = time.monotonic()
    slow_from = len(gaps)
    while not os.path.exists(shot) and time.monotonic() - turned < 30:
        if select.select([tiles.s], [], [], SLOW_MS / 1000)[0]:
            tiles.read(SLOW_BYTES)
        take(tiles.frames())
        time.sleep(SLOW_MS / 1000)
    # The pen put away, then full speed again until no tile has come for a second (the hovering
    # pen's frames on this stream do not count).
    stop.set()
    for t in threads:
        t.join(2)
    alive, last = True, time.monotonic()
    while time.monotonic() - last < 1.0:
        if not select.select([tiles.s], [], [], 0.1)[0]:
            continue
        if not tiles.read():
            alive = False
            break
        fr = tiles.frames()
        if any(kind in (1, 2, 9) for kind, _ in fr):
            last = time.monotonic()
        take(fr)

    got = [((now - ident) & 0xFFFFFFFF) / 1000 for now, ident in arrivals if ident in sent]
    delivered = len({i for _, i in arrivals if i in sent})
    s = stats(got)
    print('     pen: %d of %d frames delivered; frame to browser p50 / p90 / max %s ms'
          % (delivered, len(sent), ' / '.join('%.1f' % v for v in s) if s else '...'))
    base = min(gaps[:calm]) if calm else (min(gaps) if gaps else 0)
    age = stats([g - base for g in gaps[slow_from:]])
    print('     tiles: %d turns cost %d bytes of tiles in %d batches, %d hellos; batch age p50 / p90 / max %s ms'
          ' (mostly the browser\'s own receive buffer on loopback)'
          % (TURNS, tile_bytes, len(gaps) - slow_from, hellos, ' / '.join('%.0f' % v for v in age) if age else '...'))
    check('the pen viewer\'s samples kept coming while the tiles waited (%d of %d)' % (delivered, len(sent)),
          len(sent) > 0 and delivered >= 0.95 * len(sent))
    check('and at once (p90 %.1f ms, under 25)' % (s[1] if s else -1), bool(s) and s[1] < 25)
    check('the tile stream lived through the slow reads', alive and len(gaps) > slow_from)
    # The last screen, but for the overlays never in the frame (the chip, the clock and battery corner).
    if size is not None:
        screen = [bytearray(b'\xff' * size[0]) for _ in range(size[1])]
        for body in kept:
            x, y, w, h = struct.unpack('<HHHH', body[:8])
            pw, ph, rows = png_gray(body[8:])
            for i in range(min(h, ph)):
                screen[y + i][x:x + w] = rows[i][:w]
    same = False
    if os.path.exists(shot) and screen is not None:
        sw, sh, srows = png_gray(open(shot, 'rb').read())
        skip = [r for r in layers.values() if r[2]]
        skip.append((sw - 100, 0, 100, 60))  # the live chip, top right, where Grimoire draws it
        bad = 0
        if (sw, sh) == size:
            for y in range(sh):
                row, want = screen[y], srows[y]
                if row == want:
                    continue
                for x in range(sw):
                    if row[x] != want[x] and not any(rx <= x < rx + rw and ry <= y < ry + rh for rx, ry, rw, rh in skip):
                        bad += 1
        same = (sw, sh) == size and bad == 0
        print('     the picture: %s, the shot %dx%d; %d pixels differ outside the overlays' % (size, sw, sh, bad))
    check('the picture it ends with is the app\'s last screen', same)
finally:
    stop.set()
    app.send_signal(signal.SIGTERM)
    try:
        app.wait(5)
    except subprocess.TimeoutExpired:
        app.kill()
sys.exit(fail)
