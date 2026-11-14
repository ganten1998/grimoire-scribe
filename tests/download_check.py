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


# The server renders two PDFs at a time (httpd.c MAX_CHILDREN), so of 8 at once the last ones wait
# at least three renders: queued long enough to be seen even where one render is quick (a small
# notebook on a fast CI machine missed a single waiting download between two polls). One watcher a
# download, each asking about its own for as long as it runs: asked one after another, a status
# request a round of the app's idle loop each (about 40 ms), the eight were seen only three times
# in the second the batch takes, and on a slower machine no request found a download preparing.
tokens = [uuid.uuid4().hex for _ in range(8)]


def watch(token, job):
    states, previous = set(), 0
    while not job.done():
        d = status(token)
        if d:
            states.add(d['state'])
            assert 0 <= d['pages'] <= d['total']
            assert d['pages'] >= previous, 'page progress moved backward'
            previous = d['pages']
    return states


seen = set()
with concurrent.futures.ThreadPoolExecutor(max_workers=2 * len(tokens)) as pool:
    jobs = [pool.submit(get, f'/nb/{notebook}.pdf?progress={t}') for t in tokens]
    watchers = [pool.submit(watch, t, j) for t, j in zip(tokens, jobs)]
    until = time.monotonic() + 40
    for w in watchers:
        try:
            # (a watcher's failed assertion is raised here)
            seen |= w.result(timeout=max(0, until - time.monotonic()))
        except concurrent.futures.TimeoutError:
            raise AssertionError('downloads stalled') from None
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
