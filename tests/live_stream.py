#!/usr/bin/env python3
# Live view's end-to-end check, against the headless app holding its pairing card open
# (tests/live_check.sh starts it): the routes before pairing, wrong codes, pairing with the code,
# the stream (its hello, the screen as PNG tiles, composed), a third viewer refused, and the log
# free of codes and tokens. Standard library only.
# usage: live_stream.py <port> <app log> <out dir>
import http.client, os, re, struct, sys, time, zlib

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

# The stream.
conn, r = get('/live/stream', session, keep=True)
check('the stream answers', r.status == 200 and r.getheader('Content-Type') == 'application/octet-stream')
screen, size, hellos, batches, tiles = None, None, 0, 0, 0
t0 = time.time()
for kind, body in frames(r):
    if kind == 1:
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
        break
    if time.time() - t0 > 20:
        break
check('a hello, then the screen as tiles in a batch', hellos == 1 and tiles > 0 and batches == 1 and size is not None)
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
c2, r2 = get('/live/stream', session, keep=True)
st3, hd3, body3 = get('/live/stream', session)
check('two viewers at once, not three (503)', r2.status == 200 and st3 == 503)
c2.close()
conn.close()
time.sleep(0.5)
# (The simulator's own lines name the code and the QR code's address, for tests like this one.)
text = ''.join(l for l in open(LOG, errors='replace') if '(simulator)' not in l)
check('the log never holds a code or token', '/live/pair?' not in text and code not in text)
check('the log says a browser paired', 'live: a browser paired' in text)
sys.exit(fail)
