#!/usr/bin/env python3
"""Stage a development bundle on Kindle USB/MTP and verify a fresh device read-back.

This uses the existing KindleLab USB volume bridge. It neither installs nor executes files,
changes the running app, or installs plugins. A normal app exit is still needed to apply a build.
"""
import argparse
import hashlib
import json
from pathlib import Path
import re
import shutil
import sys
import time
import urllib.error
import urllib.parse
import urllib.request
import uuid


def sha256(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


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
        if path.is_symlink() or not path.is_file() or sha256(path) != digest:
            raise ValueError('Local checksum mismatch: ' + name)
        names.add(name)
        entries.append((path, digest))
    if ('SOURCE.json' not in names or not any(n.endswith('.kpkg') for n in names)
            or not any(n.startswith('Grimoire-') and n.endswith('.zip') for n in names)):
        raise ValueError('Bundle is missing source provenance or installable artifacts.')
    entries.append((bundle / 'SHA256SUMS.txt', sha256(bundle / 'SHA256SUMS.txt')))
    return source, entries


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('bundle', type=Path)
    parser.add_argument('--mount', type=Path, default=Path('/Volumes/Kindle'))
    parser.add_argument('--bridge-state', type=Path, default=Path.home() / 'KindleLab/volume/state')
    parser.add_argument('--receipt', type=Path, required=True, help='Local JSON result; never on the Kindle')
    args = parser.parse_args()
    bundle = args.bundle.resolve(strict=True)
    source, entries = bundle_files(bundle)
    state = json.loads((args.bridge_state / 'status.json').read_text())
    if not state.get('connected') or state.get('transport') != 'usb-mtp' or not state.get('device_serial'):
        raise ValueError('The serial-verified Kindle USB/MTP bridge is not connected.')
    if args.mount.resolve(strict=True) != Path(state['mount']).resolve(strict=True) or not args.mount.is_mount():
        raise ValueError('Mount does not match the connected Kindle bridge.')
    if args.receipt.resolve().is_relative_to(args.mount.resolve()):
        raise ValueError('The verification receipt must be saved on this computer.')
    token = (args.bridge_state / 'url-token').read_text().strip()
    if not re.fullmatch(r'[A-Za-z0-9_-]{20,128}', token):
        raise ValueError('Unexpected local bridge token format.')
    prefix = 'http://127.0.0.1:18763/' + token + '/Kindle/'

    def read_device(relative):
        # The provider's GET always downloads the MTP object. Avoid the mounted filesystem's
        # page cache, so verification cannot just read back the Mac's own recent write buffer.
        request = urllib.request.Request(prefix + urllib.parse.quote(relative, safe='/'),
                                         headers={'Cache-Control': 'no-cache'})
        try:
            return urllib.request.urlopen(request, timeout=120)
        except urllib.error.URLError:
            # Never print a URL containing the private loopback bridge token.
            raise OSError('Fresh USB read-back failed: ' + relative) from None

    with read_device('grimoire/app/BUILD_ID') as stream:
        installed_before = stream.read(128).decode().strip()
    generation = source['revision'][:12] + '-' + uuid.uuid4().hex[:12]
    relative = 'kindle-lab/grimoire-dev/' + generation
    destination = args.mount / relative
    destination.mkdir(parents=True, exist_ok=False)
    started = time.monotonic()
    files = []
    for path, expected in entries:
        transfer_start = time.monotonic()
        shutil.copyfile(path, destination / path.name)
        with read_device(relative + '/' + path.name) as stream:
            actual = hashlib.file_digest(stream, 'sha256').hexdigest()
        if actual != expected:
            raise ValueError('USB checksum mismatch; unready stage retained: ' + path.name)
        files.append({'name': path.name, 'bytes': path.stat().st_size, 'sha256': actual,
                      'seconds': round(time.monotonic() - transfer_start, 3)})
        print('verified', path.name, path.stat().st_size, 'bytes', flush=True)
    with read_device('grimoire/app/BUILD_ID') as stream:
        installed_after = stream.read(128).decode().strip()
    if installed_before != installed_after:
        raise ValueError('Installed build changed during staging; investigate before applying.')
    result = {'schema': 1, 'purpose': 'development-only; staged, not installed',
              'transport': 'usb-mtp', 'device_serial': state['device_serial'],
              'source': source, 'device_directory': '/mnt/us/' + relative,
              'mounted_directory': str(destination), 'files': files,
              'installed_build_id_before': installed_before, 'installed_build_id_after': installed_after,
              'transfer_and_verify_seconds': round(time.monotonic() - started, 3),
              'verified_utc': time.strftime('%Y-%m-%dT%H:%M:%SZ', time.gmtime()),
              'installed': False, 'app_restart_requested': False}
    ready = json.dumps(result, indent=2) + '\n'
    (destination / 'READY.json').write_text(ready)
    with read_device(relative + '/READY.json') as stream:
        if stream.read() != ready.encode():
            raise ValueError('The stage readiness receipt did not read back correctly.')
    args.receipt.parent.mkdir(parents=True, exist_ok=True)
    args.receipt.write_text(ready)
    print('USB stage verified:', destination)
    print('Installed build unchanged:', installed_after)
    print('Transfer and fresh USB verification:', result['transfer_and_verify_seconds'], 'seconds')


if __name__ == '__main__':
    try:
        main()
    except (OSError, ValueError, KeyError) as error:
        sys.exit('usb-stage: ' + str(error))
