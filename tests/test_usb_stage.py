#!/usr/bin/env python3
"""tools/usb-stage.py against a stand-in for KindleLab's USB bridge (a WebDAV server over an
in-memory Kindle on a loopback port): staging writes with MKCOL and PUT and never an AppleDouble
file, verifies with fresh reads, prunes old generations, removes old sidecars, and runs under the
Mac's own /usr/bin/python3 (3.9) as well as this Python."""
from email.utils import formatdate
import hashlib
import http.server
import json
import os
import shutil
import subprocess
import sys
import tempfile
import threading
import unittest
import urllib.parse
from xml.sax.saxutils import escape

HERE = os.path.dirname(os.path.abspath(__file__))
SCRIPT = os.path.join(HERE, '..', 'tools', 'usb-stage.py')
TOKEN = 'StageTestToken_0123456789abcdef'
PREFIX = '/' + TOKEN + '/Kindle'


class FakeBridge(http.server.BaseHTTPRequestHandler):
    """Folders are paths ending in '/'; files map to (bytes, mtime)."""
    tree = {}
    log = []

    def log_message(self, *args):
        pass

    def path_of(self):
        path = urllib.parse.unquote(urllib.parse.urlparse(self.path).path)
        assert path.startswith(PREFIX), path
        return path[len(PREFIX):] or '/'

    def reply(self, code, body=b'', headers=()):
        self.send_response(code)
        for k, v in headers:
            self.send_header(k, v)
        self.send_header('Content-Length', str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def do_MKCOL(self):
        p = self.path_of().rstrip('/') + '/'
        self.log.append(('MKCOL', p))
        if p in self.tree:
            return self.reply(405)
        self.tree[p] = (None, 1700000000)
        self.reply(201)

    def do_PUT(self):
        p = self.path_of()
        data = self.rfile.read(int(self.headers['Content-Length']))
        self.log.append(('PUT', p))
        self.tree[p] = (data, 1700000000)
        self.reply(201)

    def do_GET(self):
        p = self.path_of()
        self.log.append(('GET', p))
        if p not in self.tree or self.tree[p][0] is None:
            return self.reply(404)
        self.reply(200, self.tree[p][0])

    def do_DELETE(self):
        p = self.path_of().rstrip('/')
        self.log.append(('DELETE', p))
        for k in [k for k in self.tree if k.rstrip('/') == p or k.startswith(p + '/')]:
            del self.tree[k]
        self.reply(204)

    def do_PROPFIND(self):
        p = self.path_of().rstrip('/') + '/'
        self.log.append(('PROPFIND', p))
        if p not in self.tree:
            return self.reply(404)
        items = [(p, True, self.tree[p][1])]
        for k, (data, mtime) in sorted(self.tree.items()):
            rest = k[len(p):]
            if k.startswith(p) and rest and '/' not in rest.rstrip('/'):
                items.append((k, data is None, mtime))
        body = '<?xml version="1.0"?><d:multistatus xmlns:d="DAV:">' + ''.join(
            '<d:response><d:href>%s</d:href><d:propstat><d:prop>%s<d:getlastmodified>%s</d:getlastmodified>'
            '</d:prop></d:propstat></d:response>' % (
                escape(urllib.parse.quote(PREFIX + k)),
                '<d:resourcetype><d:collection/></d:resourcetype>' if folder else '<d:resourcetype/>',
                formatdate(mtime, usegmt=True)) for k, folder, mtime in items) + '</d:multistatus>'
        self.reply(207, body.encode(), [('Content-Type', 'application/xml')])


class UsbStage(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp(prefix='usb-stage-')
        FakeBridge.tree = {'/': (None, 0), '/grimoire/': (None, 0), '/grimoire/app/': (None, 0),
                           '/grimoire/app/BUILD_ID': (b'967cfb807c90\n', 0)}
        FakeBridge.log = []
        self.server = http.server.ThreadingHTTPServer(('127.0.0.1', 0), FakeBridge)
        threading.Thread(target=self.server.serve_forever, daemon=True).start()
        self.state = os.path.join(self.tmp, 'state')
        os.makedirs(self.state)
        with open(os.path.join(self.state, 'status.json'), 'w') as f:
            json.dump({'connected': True, 'transport': 'usb-mtp', 'device_serial': 'G0TEST'}, f)
        with open(os.path.join(self.state, 'url-token'), 'w') as f:
            f.write(TOKEN + '\n')
        self.bundle = os.path.join(self.tmp, 'Grimoire-2.0.0-abc')
        os.makedirs(self.bundle)
        files = {'SOURCE.json': json.dumps({'revision': 'a' * 40, 'dirty': False}).encode(),
                 'Grimoire-2.0.0.zip': os.urandom(3000), 'grimoire_2.0.0_kindlehf.kpkg': os.urandom(2000)}
        sums = ''
        for name, data in files.items():
            with open(os.path.join(self.bundle, name), 'wb') as f:
                f.write(data)
            sums += '%s  %s\n' % (hashlib.sha256(data).hexdigest(), name)
        with open(os.path.join(self.bundle, 'SHA256SUMS.txt'), 'w') as f:
            f.write(sums)

    def tearDown(self):
        self.server.shutdown()
        self.server.server_close()
        shutil.rmtree(self.tmp, ignore_errors=True)

    def run_stage(self, *args, python=sys.executable):
        return subprocess.run([python, SCRIPT, '--bridge', 'http://127.0.0.1:%d' % self.server.server_port,
                               '--bridge-state', self.state, '--mount', os.path.join(self.tmp, 'no-mount'),
                               *args], capture_output=True, text=True, timeout=60)

    def staged(self):
        return sorted(k for k in FakeBridge.tree if k.startswith('/kindle-lab/grimoire-dev/') and k.endswith('/') and k.count('/') == 4)

    def check_stage(self, python):
        receipt = os.path.join(self.tmp, 'receipt.json')
        p = self.run_stage(self.bundle, '--receipt', receipt, python=python)
        self.assertEqual(p.returncode, 0, p.stdout + p.stderr)
        self.assertNotIn(TOKEN, p.stdout + p.stderr)
        with open(receipt) as f:
            result = json.load(f)
        generation = result['device_directory'].rsplit('/', 1)[1]
        names = sorted(k.rsplit('/', 1)[1] for k in FakeBridge.tree
                       if k.startswith('/kindle-lab/grimoire-dev/' + generation + '/') and not k.endswith('/'))
        self.assertEqual(names, ['Grimoire-2.0.0.zip', 'READY.json', 'SHA256SUMS.txt', 'SOURCE.json',
                                 'grimoire_2.0.0_kindlehf.kpkg'])
        self.assertFalse([k for k in FakeBridge.tree if '/._' in k or k.endswith('.DS_Store')])
        self.assertIn(('MKCOL', '/kindle-lab/grimoire-dev/' + generation + '/'), FakeBridge.log)
        puts = [p for m, p in FakeBridge.log if m == 'PUT']
        gets = [p for m, p in FakeBridge.log if m == 'GET']
        self.assertTrue(all(p in gets for p in puts))  # every write read back fresh
        self.assertEqual(result['installed_build_id_after'], '967cfb807c90')

    def test_stage_writes_over_http_and_verifies(self):
        self.check_stage(sys.executable)

    @unittest.skipUnless(os.path.exists('/usr/bin/python3'), 'no /usr/bin/python3')
    def test_stage_under_the_system_python(self):
        version = subprocess.run(['/usr/bin/python3', '-c', 'import sys; print(sys.version_info[:2] >= (3, 9))'],
                                 capture_output=True, text=True).stdout.strip()
        if version != 'True':
            self.skipTest('/usr/bin/python3 is older than 3.9')
        self.check_stage('/usr/bin/python3')

    def test_prune_keeps_the_newest_generations(self):
        for i, gen in enumerate(['111111111111-aaaaaaaaaaaa', '222222222222-bbbbbbbbbbbb', '333333333333-cccccccccccc']):
            FakeBridge.tree['/kindle-lab/grimoire-dev/%s/' % gen] = (None, 1700000000 + i * 100)
            FakeBridge.tree['/kindle-lab/grimoire-dev/%s/READY.json' % gen] = (b'{}', 0)
        FakeBridge.tree['/kindle-lab/'] = (None, 0)
        FakeBridge.tree['/kindle-lab/grimoire-dev/'] = (None, 0)
        FakeBridge.tree['/kindle-lab/grimoire-dev/notes/'] = (None, 0)  # not a generation: kept
        p = self.run_stage('--prune-keep', '2')
        self.assertEqual(p.returncode, 0, p.stderr)
        self.assertEqual(self.staged(), ['/kindle-lab/grimoire-dev/222222222222-bbbbbbbbbbbb/',
                                         '/kindle-lab/grimoire-dev/333333333333-cccccccccccc/',
                                         '/kindle-lab/grimoire-dev/notes/'])
        p = self.run_stage(self.bundle, '--receipt', os.path.join(self.tmp, 'r.json'), '--prune-keep', '1')
        self.assertEqual(p.returncode, 0, p.stderr)
        with open(os.path.join(self.tmp, 'r.json')) as f:
            new = json.load(f)['device_directory'].rsplit('/', 1)[1]
        self.assertEqual(self.staged(), ['/kindle-lab/grimoire-dev/%s/' % new, '/kindle-lab/grimoire-dev/notes/'])

    def test_clean_sidecars_removes_old_appledouble_files(self):
        FakeBridge.tree.update({'/kindle-lab/': (None, 0), '/kindle-lab/grimoire-dev/': (None, 0),
                                '/kindle-lab/grimoire-dev/111111111111-aaaaaaaaaaaa/': (None, 0),
                                '/kindle-lab/grimoire-dev/111111111111-aaaaaaaaaaaa/a.kpkg': (b'k', 0),
                                '/kindle-lab/grimoire-dev/111111111111-aaaaaaaaaaaa/._a.kpkg': (b'x', 0),
                                '/kindle-lab/grimoire-dev/._111111111111-aaaaaaaaaaaa': (b'x', 0),
                                '/kindle-lab/grimoire-dev/.DS_Store': (b'x', 0)})
        p = self.run_stage('--clean-sidecars')
        self.assertEqual(p.returncode, 0, p.stderr)
        left = sorted(k for k in FakeBridge.tree if k.startswith('/kindle-lab/grimoire-dev/'))
        self.assertEqual(left, ['/kindle-lab/grimoire-dev/', '/kindle-lab/grimoire-dev/111111111111-aaaaaaaaaaaa/',
                                '/kindle-lab/grimoire-dev/111111111111-aaaaaaaaaaaa/a.kpkg'])

    def test_a_disconnected_bridge_is_refused(self):
        with open(os.path.join(self.state, 'status.json'), 'w') as f:
            json.dump({'connected': False, 'transport': 'usb-mtp'}, f)
        p = self.run_stage(self.bundle, '--receipt', os.path.join(self.tmp, 'r.json'))
        self.assertNotEqual(p.returncode, 0)
        self.assertIn('not connected', p.stderr)
        self.assertFalse([m for m, _ in FakeBridge.log if m in ('PUT', 'MKCOL')])


if __name__ == '__main__':
    unittest.main()
