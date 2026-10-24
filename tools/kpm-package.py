#!/usr/bin/env python3
"""Build a deterministic KPM v2 artifact and repository index from the tested installer zip."""
import argparse
import gzip
import hashlib
import io
import json
import pathlib
import re
import tarfile
import zipfile


def package_version(version):
    """KPM v2 has integer triples, no prerelease field. Preserve beta < rc < final < next patch."""
    m = re.fullmatch(r'(\d+)\.(\d+)\.(\d+)(?:-(beta|rc)\.([1-9]\d{0,2}))?', version)
    if not m:
        raise ValueError('Expected MAJOR.MINOR.PATCH[-beta.N|-rc.N], with N=1..999')
    major, minor, patch = map(int, m.group(1, 2, 3))
    stage = 9999 if not m[4] else (1000 if m[4] == 'beta' else 5000) + int(m[5])
    return [major, minor, patch * 10000 + stage]


def json_bytes(value):
    return (json.dumps(value, ensure_ascii=False, indent=2) + '\n').encode()


def build(installer, version, output, base_url, repo_root):
    encoded = package_version(version)
    output.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(installer) as archive:
        payload = archive.read('documents/Grimoire.sh')
        archive.getinfo('documents/Grimoire - reset pen calibration.sh')
        license_text = archive.read('LICENSE.txt')
        if not payload.startswith(b'#!/bin/sh\n') or b'\n__GRIMOIRE_APP_ARCHIVE__\n' not in payload:
            raise ValueError('Installer zip has no self-contained Grimoire launcher')
        if b'Grimoire ' + version.encode() not in archive.read('README.txt'):
            raise ValueError('Installer zip version does not match requested KPM version')
    icon = next(line for line in payload.split(b'\n', 10) if line.startswith(b'# Icon: ')).decode()
    wrapper = ('#!/bin/sh\n# Name: Grimoire\n# Author: Ganten\n' + icon +
               '\n# DontUseFBInk\n# Capabilities: guard unpack-only hosted\n# Grimoire KPM launcher\n'
               'exec "${GRIMOIRE_KPM:-/var/local/kmc/bin/kpm}" launch grimoire "$@"\n').encode()
    reset_wrapper = (b'#!/bin/sh\n# Name: Grimoire - reset pen calibration\n# Author: Ganten\n'
                     b'# DontUseFBInk\n# Grimoire KPM launcher\n'
                     b'exec "${GRIMOIRE_KPM:-/var/local/kmc/bin/kpm}" launch grimoire --reset-calibration\n')
    manifest = {
        'manifest_version': 2, 'id': 'grimoire', 'name': 'Grimoire', 'author': 'Ganten',
        'description': 'Handwriting notebooks for the jailbroken Kindle Scribe. ' + version,
        'version': encoded, 'grimoire_version': version, 'dependencies': [], 'supported_platforms': ['kindlehf'],
    }
    members = {'manifest.json': json_bytes(manifest), 'payload/Grimoire.sh': payload,
               'scriptlets/Grimoire.sh': wrapper,
               'scriptlets/Grimoire - reset pen calibration.sh': reset_wrapper, 'LICENSE.txt': license_text}
    for name in ('common.sh', 'install.sh', 'uninstall.sh', 'launch.sh'):
        members[name] = (repo_root / 'kindle/kpm' / name).read_bytes()
    # gzip mtime and tar metadata are fixed: identical inputs yield identical bytes on every host.
    blob = io.BytesIO()
    with gzip.GzipFile(fileobj=blob, mode='wb', filename='', mtime=0, compresslevel=9) as gz:
        with tarfile.open(fileobj=gz, mode='w', format=tarfile.USTAR_FORMAT) as tar:
            for name, data in sorted(members.items()):
                info = tarfile.TarInfo(name)
                info.size = len(data)
                info.mode = 0o755 if name.endswith('.sh') else 0o644
                info.mtime = info.uid = info.gid = 0
                tar.addfile(info, io.BytesIO(data))
    filename = 'grimoire_' + '.'.join(map(str, encoded)) + '_kindlehf.kpkg'
    content = blob.getvalue()
    (output / filename).write_bytes(content)
    digest = hashlib.sha256(content).hexdigest()
    channel = 'preview' if '-' in version else 'stable'
    repository = {
        'manifest_version': 2, 'id': 'grimoire' + channel, 'name': 'Grimoire ' + channel,
        'description': 'Grimoire official ' + channel + ' channel',
        'packages': {'grimoire': {k: manifest[k] for k in ('name', 'author', 'description')}},
    }
    repository['packages']['grimoire']['artifacts'] = [{
        'url': base_url.rstrip('/') + '/' + filename, 'version': encoded, 'dependencies': [],
        'supported_platforms': ['kindlehf'], 'grimoire_version': version, 'sha256': digest,
    }]
    for name in ('manifest.json', 'manifest.v2.json'):
        (output / name).write_bytes(json_bytes(repository))
    (output / 'SHA256SUMS.txt').write_text(digest + '  ' + filename + '\n')
    return filename


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--zip', type=pathlib.Path, required=True)
    parser.add_argument('--version', required=True)
    parser.add_argument('--output', type=pathlib.Path, required=True)
    parser.add_argument('--base-url', required=True)
    args = parser.parse_args()
    if not args.base_url.startswith('https://'):
        parser.error('Repository artifacts require an HTTPS base URL')
    print(build(args.zip, args.version, args.output, args.base_url, pathlib.Path(__file__).resolve().parent.parent))


if __name__ == '__main__':
    main()
