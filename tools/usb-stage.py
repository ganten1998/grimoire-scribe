#!/usr/bin/env python3
"""Stage a development bundle on Kindle USB/MTP and verify a fresh device read-back.

This uses the existing KindleLab USB volume bridge. It neither installs nor executes files,
changes the running app, or installs plugins. A normal app exit is still needed to apply a build.
Files go straight to the bridge's WebDAV endpoint (MKCOL and PUT), not through the Finder mount,
so macOS writes no AppleDouble ('._') files beside them. Python 3.9 or newer.
"""
import sys

if sys.version_info < (3, 9):
    sys.exit('usb-stage: needs Python 3.9 or newer (this is %d.%d)' % sys.version_info[:2])

import argparse
import hashlib
import json
from pathlib import Path
import re
import time
import urllib.error
import urllib.parse
import urllib.request
import uuid
import xml.etree.ElementTree as ET

STAGE_DIR = 'kindle-lab/grimoire-dev'
GENERATION = re.compile(r'[0-9a-f]{12}-[0-9a-f]{12}')
SIDECAR = re.compile(r'(\._.*|\.DS_Store)')


def sha256(stream):
    digest = hashlib.sha256()
    for block in iter(lambda: stream.read(1 << 20), b''):
        digest.update(block)
    return digest.hexdigest()


def file_sha256(path):
    with path.open('rb') as stream:
        return sha256(stream)


def bundle_files(bundle):
    source = json.loads((bundle / 'SOURCE.json').read_text())
    if source.get('dirty') or not re.fullmatch(r'[0-9a-f]{40}', source.get('revision', '')):
        raise ValueError('Use a collected bundle from a clean committed source tree.')
    entries = []
    names = set()
    for line in (bundle / 'SHA256SUMS.txt').read_text().splitlines():
        digest, name = line.split('  ', 1)
        if (not re.fullmatch(r'[0-9a-f]{64}', digest) or not name or name in names
                or name in ('.', '..') or '/' in name or '\\' in name):
            raise ValueError('Invalid or duplicate checksum entry.')
        path = bundle / name
        if path.is_symlink() or not path.is_file() or file_sha256(path) != digest:
            raise ValueError('Local checksum mismatch: ' + name)
        names.add(name)
        entries.append((path, digest))
    if ('SOURCE.json' not in names or not any(n.endswith('.kpkg') for n in names)
            or not any(n.startswith('Grimoire-') and n.endswith('.zip') for n in names)):
        raise ValueError('Bundle is missing source provenance or installable artifacts.')
    entries.append((bundle / 'SHA256SUMS.txt', file_sha256(bundle / 'SHA256SUMS.txt')))
    return source, entries


class Bridge:
    """The bridge's token URL. Errors never show the URL: it holds the private loopback token."""
    def __init__(self, base, token):
        self.prefix = base.rstrip('/') + '/' + token + '/Kindle/'

    def request(self, method, relative, data=None, headers=None, ok=(200, 201, 204, 207), timeout=120):
        request = urllib.request.Request(self.prefix + urllib.parse.quote(relative, safe='/'), data=data,
                                         method=method, headers=headers or {})
        try:
            response = urllib.request.urlopen(request, timeout=timeout)
        except urllib.error.HTTPError as error:
            error.close()
            if error.code in ok:
                return None
            raise OSError('USB bridge %s %s failed: HTTP %d' % (method, relative, error.code)) from None
        except urllib.error.URLError:
            raise OSError('USB bridge %s %s failed' % (method, relative)) from None
        return response

    def read(self, relative):
        # The provider's GET always downloads the MTP object. Avoid the mounted filesystem's
        # page cache, so verification cannot just read back the Mac's own recent write buffer.
        return self.request('GET', relative, headers={'Cache-Control': 'no-cache'})

    def mkcol(self, relative, exist_ok):
        # 405: the folder exists already.
        response = self.request('MKCOL', relative, ok=(201, 405) if exist_ok else (201,))
        if response:
            response.close()

    def put(self, relative, path):
        with path.open('rb') as stream:
            self.request('PUT', relative, data=stream,
                         headers={'Content-Length': str(path.stat().st_size),
                                  'Content-Type': 'application/octet-stream'}).close()

    def put_bytes(self, relative, data):
        self.request('PUT', relative, data=data, headers={'Content-Length': str(len(data))}).close()

    def listing(self, relative, raw=False):
        """(name, is_folder, mtime) under a folder; raw shows the Kindle's own sidecars too."""
        headers = {'Depth': '1'}
        if raw:
            headers['X-KindleLab-Raw'] = '1'
        with self.request('PROPFIND', relative, headers=headers) as response:
            tree = ET.fromstring(response.read())
        rows = []
        own = urllib.parse.urlparse(self.prefix + urllib.parse.quote(relative, safe='/')).path.rstrip('/')
        for item in tree.iter('{DAV:}response'):
            href = urllib.parse.urlparse(item.find('{DAV:}href').text).path.rstrip('/')
            if href == own:
                continue
            modified = item.findtext('.//{DAV:}getlastmodified') or ''
            try:
                mtime = time.mktime(time.strptime(modified, '%a, %d %b %Y %H:%M:%S GMT'))
            except ValueError:
                mtime = 0
            rows.append((urllib.parse.unquote(href.rsplit('/', 1)[1]),
                         item.find('.//{DAV:}collection') is not None, mtime))
        return rows

    def delete(self, relative, raw=False):
        headers = {'X-KindleLab-Raw': '1'} if raw else {}
        response = self.request('DELETE', relative, headers=headers)
        if response:
            response.close()


def stage(bridge, bundle, state, receipt):
    source, entries = bundle_files(bundle)
    with bridge.read('grimoire/app/BUILD_ID') as stream:
        installed_before = stream.read(128).decode().strip()
    generation = source['revision'][:12] + '-' + uuid.uuid4().hex[:12]
    relative = STAGE_DIR + '/' + generation
    bridge.mkcol('kindle-lab', exist_ok=True)
    bridge.mkcol(STAGE_DIR, exist_ok=True)
    bridge.mkcol(relative, exist_ok=False)
    started = time.monotonic()
    files = []
    for path, expected in entries:
        transfer_start = time.monotonic()
        bridge.put(relative + '/' + path.name, path)
        with bridge.read(relative + '/' + path.name) as stream:
            actual = sha256(stream)
        if actual != expected:
            raise ValueError('USB checksum mismatch; unready stage retained: ' + path.name)
        files.append({'name': path.name, 'bytes': path.stat().st_size, 'sha256': actual,
                      'seconds': round(time.monotonic() - transfer_start, 3)})
        print('verified', path.name, path.stat().st_size, 'bytes', flush=True)
    with bridge.read('grimoire/app/BUILD_ID') as stream:
        installed_after = stream.read(128).decode().strip()
    if installed_before != installed_after:
        raise ValueError('Installed build changed during staging; investigate before applying.')
    result = {'schema': 1, 'purpose': 'development-only; staged, not installed',
              'transport': 'usb-mtp', 'device_serial': state['device_serial'],
              'source': source, 'device_directory': '/mnt/us/' + relative, 'files': files,
              'installed_build_id_before': installed_before, 'installed_build_id_after': installed_after,
              'transfer_and_verify_seconds': round(time.monotonic() - started, 3),
              'verified_utc': time.strftime('%Y-%m-%dT%H:%M:%SZ', time.gmtime()),
              'installed': False, 'app_restart_requested': False}
    ready = json.dumps(result, indent=2) + '\n'
    bridge.put_bytes(relative + '/READY.json', ready.encode())
    with bridge.read(relative + '/READY.json') as stream:
        if stream.read() != ready.encode():
            raise ValueError('The stage readiness receipt did not read back correctly.')
    receipt.parent.mkdir(parents=True, exist_ok=True)
    receipt.write_text(ready)
    print('USB stage verified: /mnt/us/' + relative)
    print('Installed build unchanged:', installed_after)
    print('Transfer and fresh USB verification:', result['transfer_and_verify_seconds'], 'seconds')
    return generation


def prune(bridge, keep, current=None):
    """Removes all but the newest KEEP staged generations (the one just staged always stays)."""
    rows = [r for r in bridge.listing(STAGE_DIR) if r[1] and GENERATION.fullmatch(r[0])]
    rows.sort(key=lambda r: (r[0] == current, r[2]), reverse=True)
    for name, _, _ in rows[keep:]:
        bridge.delete(STAGE_DIR + '/' + name)
        print('removed old stage', name, flush=True)


def clean_sidecars(bridge, relative=STAGE_DIR):
    """Deletes macOS AppleDouble and .DS_Store files that copies through the Finder mount left on
    the Kindle (older usb-stage.py versions made one per staged file)."""
    for name, folder, _ in bridge.listing(relative, raw=True):
        path = relative + '/' + name
        if SIDECAR.fullmatch(name):
            bridge.delete(path, raw=True)
            print('removed', path, flush=True)
        elif folder:
            clean_sidecars(bridge, path)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('bundle', type=Path, nargs='?', help='a collected dist bundle to stage')
    parser.add_argument('--bridge-state', type=Path, default=Path.home() / 'KindleLab/volume/state')
    parser.add_argument('--bridge', default='http://127.0.0.1:18763', help=argparse.SUPPRESS)
    parser.add_argument('--mount', type=Path, default=Path('/Volumes/Kindle'),
                        help='the Finder mount, which the receipt must not be saved on')
    parser.add_argument('--receipt', type=Path, help='Local JSON result; never on the Kindle')
    parser.add_argument('--prune-keep', type=int, metavar='N',
                        help='afterwards (or alone) keep only the newest N staged generations')
    parser.add_argument('--clean-sidecars', action='store_true',
                        help='remove ._ and .DS_Store files from %s on the Kindle' % STAGE_DIR)
    args = parser.parse_args()
    if args.bundle and not args.receipt:
        parser.error('--receipt is required to stage a bundle')
    if not args.bundle and args.prune_keep is None and not args.clean_sidecars:
        parser.error('nothing to do: give a bundle, --prune-keep or --clean-sidecars')
    if args.prune_keep is not None and args.prune_keep < 1:
        parser.error('--prune-keep needs at least 1')
    state = json.loads((args.bridge_state / 'status.json').read_text())
    if not state.get('connected') or state.get('transport') != 'usb-mtp' or not state.get('device_serial'):
        raise ValueError('The serial-verified Kindle USB/MTP bridge is not connected.')
    token = (args.bridge_state / 'url-token').read_text().strip()
    if not re.fullmatch(r'[A-Za-z0-9_-]{20,128}', token):
        raise ValueError('Unexpected local bridge token format.')
    bridge = Bridge(args.bridge, token)
    generation = None
    if args.bundle:
        receipt = args.receipt.resolve()
        if args.mount.exists() and receipt.is_relative_to(args.mount.resolve()):
            raise ValueError('The verification receipt must be saved on this computer.')
        generation = stage(bridge, args.bundle.resolve(strict=True), state, receipt)
    if args.clean_sidecars:
        clean_sidecars(bridge)
    if args.prune_keep is not None:
        prune(bridge, args.prune_keep, generation)


if __name__ == '__main__':
    try:
        main()
    except (OSError, ValueError, KeyError) as error:
        sys.exit('usb-stage: ' + str(error))
