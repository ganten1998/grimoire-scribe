#!/usr/bin/env python3
"""Exercise real concurrent PDF jobs while the Share test server is running."""
import concurrent.futures
import json
import sys
import time
import urllib.error
import urllib.request
import uuid
from pathlib import Path

base, notebook, expected_path = sys.argv[1:]
expected = Path(expected_path).read_bytes()


def get(path, method='GET'):
    try:
        with urllib.request.urlopen(urllib.request.Request(base + path, method=method), timeout=40) as r:
            return r.status, r.read(), r.headers
    except urllib.error.HTTPError as e:
        return e.code, e.read(), e.headers


def status(token):
    code, body, _ = get('/api/download/' + token)
    return json.loads(body) if code == 200 else None


tokens = [uuid.uuid4().hex for _ in range(3)]
seen = set()
with concurrent.futures.ThreadPoolExecutor(max_workers=3) as pool:
    jobs = [pool.submit(get, f'/nb/{notebook}.pdf?progress={t}') for t in tokens]
    until = time.monotonic() + 40
    previous = [0] * 3
    while not all(j.done() for j in jobs):
        assert time.monotonic() < until, 'downloads stalled'
        for i, t in enumerate(tokens):
            d = status(t)
            if d:
                seen.add(d['state'])
                assert 0 <= d['pages'] <= d['total']
                assert d['pages'] >= previous[i], 'page progress moved backward'
                previous[i] = d['pages']
        time.sleep(.025)
    for t, j in zip(tokens, jobs):
        code, body, headers = j.result()
        assert code == 200 and body == expected, 'progress changed PDF bytes'
        assert int(headers['Content-Length']) == len(body)
        d = status(t)
        assert d['state'] == 4 and d['pages'] == d['total'] > 0
        assert d['sent'] == d['bytes'] == len(body)
assert 1 in seen and 2 in seen, f'queued/preparing not observed: {seen}'
assert get(f'/nb/{notebook}.pdf?progress={tokens[0]}')[0] == 400
assert get(f'/nb/{notebook}.pdf?progress=invalid')[0] == 400
assert get('/api/download/invalid')[0] == 404
missing = uuid.uuid4().hex
assert get(f'/nb/missing.pdf?progress={missing}')[0] == 404
assert status(missing)['state'] == 5, 'failed export left a stuck progress bar'
code, body, headers = get(f'/nb/{notebook}.pdf', 'HEAD')
assert code == 200 and not body and int(headers['Content-Length']) == len(expected)
print('Download progress: queued, preparation, byte counts, concurrent isolation, failure and HEAD passed')
