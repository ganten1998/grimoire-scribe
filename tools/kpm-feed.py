#!/usr/bin/env python3
"""Update one KPM channel without downgrading it or changing the other channel."""
import argparse
import json
from pathlib import Path


def update(source, destination):
    incoming = json.loads(source.read_text())
    artifact = incoming['packages']['grimoire']['artifacts'][0]
    version = artifact['grimoire_version']
    channel = 'preview' if '-' in version else 'stable'
    if incoming['id'] != 'grimoire' + channel or incoming['manifest_version'] != 2:
        raise ValueError('Invalid Grimoire channel manifest')
    target = destination / channel
    current = target / 'manifest.json'
    if current.exists():
        old = json.loads(current.read_text())['packages']['grimoire']['artifacts'][0]
        if old['version'] > artifact['version']:
            return False
        if old['version'] == artifact['version'] and old != artifact:
            raise ValueError('An existing version cannot be replaced with different package bytes or metadata')
    target.mkdir(parents=True, exist_ok=True)
    data = json.dumps(incoming, ensure_ascii=False, indent=2) + '\n'
    for name in ('manifest.json', 'manifest.v2.json'):
        (target / name).write_text(data)
    return True


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('source', type=Path)
    parser.add_argument('destination', type=Path)
    args = parser.parse_args()
    print('updated' if update(args.source, args.destination) else 'kept newer release')
