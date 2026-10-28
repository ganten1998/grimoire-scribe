#!/usr/bin/env python3
# Live view's end-to-end check, against the headless app holding its pairing card open
# (tests/live_check.sh starts it): the routes before pairing, wrong codes, pairing with the code,
# the stream (its hello, the screen as PNG tiles, composed, then the pen the script left hovering
# over the card), a third viewer refused, and the log free of codes and tokens. Standard library only.
# usage: live_stream.py <port> <app log> <out dir>
import http.client, os, re, struct, sys, time, zlib, json, threading

PORT, LOG, OUT = int(sys.argv[1]), sys.argv[2], sys.argv[3]
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

def log_code():
    for _ in range(100):
        m = re.findall(r'live: \(simulator\) the code is (\d{6})', open(LOG, errors='replace').read())
        if m:
            return m[-1]
        time.sleep(0.1)
    return None

# A PNG of 8-bit gray, as stb writes them: rows of (filter byte, pixels).
def png_gray(data):
    assert data[:8] == b'\x89PNG\r\n\x1a\n'
    at, idat, w = 8, b'', 0
    while at < len(data):
        n, kind = struct.unpack('>I4s', data[at:at + 8])
        chunk = data[at + 8:at + 8 + n]
        if kind == b'IHDR':
            w, h, depth, ctype = struct.unpack('>IIBB', chunk[:10])
            assert depth == 8 and ctype == 0
        elif kind == b'IDAT':
            idat += chunk
        at += 12 + n
    raw, rows, prev = zlib.decompress(idat), [], bytearray(w)
    for y in range(h):
        f, line = raw[y * (w + 1)], bytearray(raw[y * (w + 1) + 1:(y + 1) * (w + 1)])
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

# The stream's frames; every byte read is kept in raw, and each frame's (type, length) in seen, for
# tests/live_check.js (the browser's frame reader on the same bytes).
raw, seen = bytearray(), []

def frames(resp):
    buf = b''
    while True:
        while len(buf) >= 5:
            n = struct.unpack('<I', buf[:4])[0]
            if len(buf) < 4 + n:
                break
            seen.append((buf[4], n - 1))
            yield buf[4], buf[5:4 + n]
            buf = buf[4 + n:]
        more = resp.read1(65536) if hasattr(resp, 'read1') else resp.read(65536)
        if not more:
            return
        raw.extend(more)
        buf += more

# Before pairing (once the card is up: Share starts the server a moment before).
code = log_code()
check('the card made a code', code is not None)
st, hd, body = get('/live')
check('/live without a session is the code form', st == 200 and b'name=code' in body)
st, hd, body = get('/live/stream')
check('the stream needs a session (401)', st == 401)
st, hd, body = get('/')
check('the notebooks need Share open (403)', st == 403)
st, hd, body = get('/nb/x.pdf')
check('downloads need Share open (403)', st == 403)
st, hd, body = get('/font/jersey10.ttf')
check('the page fonts answer', st == 200)
# The pages' icons, open without a session as the fonts are: the SVG's path is the bat of bat.h,
# block for block; the phone's is a 180 px square PNG.
rows = re.findall(r'^\s*"([.X]+)",', open(os.path.join(os.path.dirname(__file__), '..', 'src', 'bat.h')).read(), re.M)
st, hd, body = get('/icon.svg?v=1')
drawn = [['.'] * len(rows[0]) for _ in rows] if rows else []
for x, y, n in re.findall(rb'M(\d+) (\d+)h(\d+)v1h-\d+z', body):
    for i in range(int(n)):
        drawn[int(y)][int(x) + i] = 'X'
check('the icon is the bat, block for block (/icon.svg, no session)', st == 200 and bool(rows) and
      dict(hd).get('Content-Type', '').startswith('image/svg+xml') and [''.join(r) for r in drawn] == rows)
st, hd, body = get('/apple-touch-icon.png')
check('the phone icon is a 180 px PNG (/apple-touch-icon.png, no session)', st == 200 and body[:8] == b'\x89PNG\r\n\x1a\n'
      and struct.unpack('>II', body[16:24]) == (180, 180))
wrong = '000000' if code != '000000' else '111111'
st, hd, body = get('/live/pair?code=' + wrong)
check('a wrong code goes back to the form', st == 302 and hd.get('Location') == '/live?e=wrong' and 'Set-Cookie' not in hd)
st, hd, body = get('/live/pair?code=%s%%20%s' % (code[:3], code[3:]))
cookie = hd.get('Set-Cookie', '')
check('the code (typed with a space) pairs', st == 302 and hd.get('Location') == '/live' and
      cookie.startswith('grimoire_session=') and 'HttpOnly' in cookie and 'SameSite=Strict' in cookie)
session = cookie.split(';')[0]
st, hd, body = get('/live/pair?code=' + code)
check('the code works once', st == 302 and 'Set-Cookie' not in hd)
st, hd, body = get('/live', session)
check('/live with the session is the viewer', st == 200 and b'GRIMOIRE_LIVE_WORDS' in body)

# The clock and pen stream share exactly the same authentication rule.
for route in ['/live/time', '/live/pen']:
    st, _, _ = get(route)
    check(route + ' requires pairing', st == 401)
st, _, body = get('/live/time', session)
check('clock returns monotonic milliseconds', st == 200 and isinstance(json.loads(body).get('ms'), int))

# The stream.
conn, r = get('/live/stream', session, keep=True)
check('the stream answers', r.status == 200 and r.getheader('Content-Type') == 'application/octet-stream')
screen, size, hellos, batches, tiles, states, pen = None, None, 0, 0, 0, [], None
# The layers (frame 9) as they come: (id, x, y, w, h, png bytes), on the stream before the pen, then
# (with the tiles' bodies, for the page under the toolbar) in the reader thread below.
layers, later_tiles = [], []
t0 = time.time()
for kind, body in frames(r):
    if kind == 5:
        states.append(bytes(body))
    elif kind == 1:
        ver, w, h, tile = struct.unpack('<HHHH', body[:8])
        size, screen, hellos = (w, h), [bytearray(b'\xff' * w) for _ in range(h)], hellos + 1
    elif kind == 2:
        x, y, w, h = struct.unpack('<HHHH', body[:8])
        pw, ph, rows = png_gray(body[8:])
        assert (pw, ph) == (w, h)
        for i in range(h):
            screen[y + i][x:x + w] = rows[i]
        tiles += 1
    elif kind == 3:
        batches += 1
    elif kind == 9:
        layers.append(struct.unpack('<BxHHHH', body[:10]) + (bytes(body[10:]),))
    elif kind == 6:
        pen = struct.unpack('<HHB', body[:5]) if len(body) >= 5 else ()
        break
    if time.time() - t0 > 20:
        break
check('a hello, then the screen as tiles in a batch', hellos == 1 and tiles > 0 and batches == 1 and size is not None)
# (tests/live_check.sh's script hovers the pen at 900,1500 over the card, before the browser pairs.)
check('then the pen, hovering where the script left it (900,1500)', pen == (900, 1500, 1) and seen[-1] == (6, 5) and
      seen[-2][0] == 3)
check('the state first: awake, the colors inverted (settings.ini invert=1), the simulator\'s warmth (8)',
      bool(seen) and seen[0] == (5, 3) and states[:1] == [bytes([0, 1, 8])])
if screen:
    w, h = size
    dark = sum(1 for row in screen for v in row[::8] if v < 128)
    check('the screen streamed is not blank (%dx%d)' % (w, h), dark > 100)
    with open(os.path.join(OUT, 'live.pgm'), 'wb') as f:
        f.write(b'P5\n%d %d\n255\n' % (w, h))
        for row in screen:
            f.write(bytes(row))
# (Only whole frames: the bytes read past the batch end are left out.)
whole = sum(5 + n for _, n in seen)
with open(os.path.join(OUT, 'stream.bin'), 'wb') as f:
    f.write(bytes(raw[:whole]))
with open(os.path.join(OUT, 'stream.txt'), 'w') as f:
    f.write(''.join('%d %d\n' % s for s in seen))
with open(os.path.join(OUT, 'pen.txt'), 'w') as f:
    f.write('%d %d %d\n' % pen if pen else '')
c2, r2 = get('/live/stream', session, keep=True)
st3, hd3, body3 = get('/live/stream', session)
check('two viewers at once, not three (503)', r2.status == 200 and st3 == 503)
c2.close()
# Keep consuming the authoritative stream while the dedicated pen stream carries early samples.
# Each batch end vouches for a sample and its ink epoch (0, 0 for none).
acks = []
def tile_ack_reader():
    try:
        while True:
            hdr = r.read(5)
            if len(hdr) != 5: return
            n, kind = struct.unpack('<IB', hdr)
            payload = r.read(n - 1)
            if kind == 3 and len(payload) == 20: acks.append(struct.unpack('<IIIII', payload)[2::2])
            if kind == 9: layers.append(struct.unpack('<BxHHHH', payload[:10]) + (bytes(payload[10:]),))
            if kind == 2: later_tiles.append(bytes(payload))
    except (OSError, http.client.HTTPException): pass
thread = threading.Thread(target=tile_ack_reader, daemon=True)
thread.start()
cp, rp = get('/live/pen', session, keep=True)
check('dedicated pen stream answers', rp.status == 200)
fast_raw, fast_frames, samples, styles = bytearray(), [], [], []
style = None
# Read the initial reset before releasing the script; no timing guess under a busy make -j.
def fast_frame():
    head = rp.read(5)
    n, kind = struct.unpack('<IB', head)
    payload = rp.read(n - 1)
    fast_raw.extend(head + payload); fast_frames.append((kind, len(payload)))
    return kind, payload
pen_epoch, pen_hellos = None, []
while True:
    kind, payload = fast_frame()
    if kind == 1: pen_hellos.append(payload)
    if kind == 7: break
check('the pen stream says hello (version 3, the screen, its ink epoch), then a reset with no samples',
      len(pen_hellos) == 1 and len(pen_hellos[0]) == 12 and struct.unpack('<HHHH', pen_hellos[0][:8]) == (3, 1860, 2480, 64)
      and len(payload) == 8 and struct.unpack('<II', payload) == (struct.unpack('<I', pen_hellos[0][8:])[0], 1))
pen_epoch = struct.unpack('<I', pen_hellos[0][8:])[0]
open(os.path.join(OUT, 'fast-start'), 'w').close()
last_seq, last_exclude, stale_frames, pen_hellos_after, ui_downs = 0, None, 0, 0, []
while True:
    kind, payload = fast_frame()
    if kind == 1:
        pen_epoch = struct.unpack('<I', payload[8:12])[0]
        pen_hellos_after += 1
    elif kind == 8:
        style = payload
        styles.append(payload)
        if style[0]: last_exclude = struct.unpack('<4i', style[48:64])
    elif kind == 7 and len(payload) >= 28:
        stale_frames += struct.unpack('<I', payload[:4])[0] != pen_epoch
        for at in range(8, len(payload), 20):
            seq, ms, x, y, pressure, tx, ty, state, flags, reserved = struct.unpack('<IIHHHbbBBH', payload[at:at + 20])
            samples.append((seq, x, y, pressure, state, flags, bool(style and style[0])))
            last_seq = seq
            # A pen-down on the panel (the exclusion of the last style that drew) is UI: no quick ink.
            if flags & 4 and last_exclude and last_exclude[0] <= x < last_exclude[0] + last_exclude[2] and \
                    last_exclude[1] <= y < last_exclude[1] + last_exclude[3]:
                ui_downs.append(flags)
        if samples[-1][1:3] == (902, 1502): break
ink_samples = [s for s in samples if s[5] & 16 and s[4] == 2]
check('complete stroke arrives with pressure and a start flag', len(ink_samples) >= 33 and
      any(s[5] & 4 for s in ink_samples) and max(s[3] for s in ink_samples) > min(s[3] for s in ink_samples))
menu_samples = [s for s in samples if 800 <= s[1] <= 1000 and 400 <= s[2] <= 450 and s[4] == 2]
check('menu contacts never request provisional ink', len(menu_samples) >= 17 and all(not s[6] and not s[5] & 16 for s in menu_samples))
check('a pen-down on the panel asks for no quick ink (%d of them)' % len(ui_downs),
      len(ui_downs) >= 1 and all(not f & 16 for f in ui_downs))
# (Opening the new notebook switched screens, which moved the ink epoch on: a hello said so.)
check('each samples frame carries the ink epoch of the hello before it', stale_frames == 0 and pen_hellos_after >= 1)
check('fast connection sends no tiles', all(kind != 2 for kind, _ in fast_frames))
for _ in range(100):
    if any(seq >= last_seq for seq, _ in acks): break
    time.sleep(.02)
check('tiles acknowledge the processed samples, in the ink epoch they came in',
      any(seq >= last_seq and ink == pen_epoch for seq, ink in acks))
# The layers: the clock and battery corner (settings.ini status_on=1), never in the tiles, and the
# editor's toolbar, whose tiles hold the page under it: down the middle of its left edge the layer
# has its black frame, and the page there is paper.
for _ in range(100):
    if any(l[0] == 0 and l[3] for l in layers): break
    time.sleep(.05)
corner = [l for l in layers if l[0] == 1 and l[3]]
check('the clock and battery corner comes as a layer of its own (%d)' % len(corner), len(corner) >= 1 and
      corner[-1][5][:8] == b'\x89PNG\r\n\x1a\n' and png_gray(corner[-1][5])[:2] == corner[-1][3:5])
bars = [l for l in layers if l[0] == 0 and l[3]]
if bars:
    _, bx, by, bw, bh, bpng = bars[-1]
    lw, lh, lrows = png_gray(bpng)
    px, mid = bx + 3, by + bh // 2
    in_layer = sum(1 for y in range(mid - 10, mid + 10) if lrows[y - by][px - bx] < 128)
    under = [row[:] for row in screen] if screen else None
    for body in later_tiles:  # (only the runs that hold the column, decoded)
        x, y, w, h = struct.unpack('<HHHH', body[:8])
        if x <= px < x + w and y < mid + 10 and mid - 10 < y + h:
            _, _, rows = png_gray(body[8:])
            for i in range(h):
                under[y + i][x:x + w] = rows[i]
    in_page = sum(1 for y in range(mid - 10, mid + 10) if under[y][px] < 128) if under else 99
    check('the toolbar comes as a layer, and the tiles under it hold the page (%d of 20 dark in the layer, %d in the page)'
          % (in_layer, in_page), lw == bw and lh == bh and in_layer >= 18 and in_page <= 4)
else:
    check('the toolbar comes as a layer of its own', False)
with open(os.path.join(OUT, 'fast.bin'), 'wb') as f: f.write(fast_raw)
with open(os.path.join(OUT, 'fast.txt'), 'w') as f: f.write(''.join('%d %d\n' % item for item in fast_frames))
cp.close()
conn.close()
time.sleep(0.5)
# (The simulator's own lines name the code and the QR code's address, for tests like this one.)
text = ''.join(l for l in open(LOG, errors='replace') if '(simulator)' not in l)
check('the log never holds a code or token', '/live/pair?' not in text and code not in text)
check('the log says a browser paired', 'live: a browser paired' in text)
sys.exit(fail)
