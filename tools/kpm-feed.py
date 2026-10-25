#!/usr/bin/env python3
"""Update the KPM channels without downgrading either one.

A prerelease goes to preview only. A stable release goes to stable and is mirrored into preview,
so a preview subscriber is offered it too, unless preview already holds a newer version."""
import argparse
import copy
import json
from pathlib import Path


def channel_manifest(incoming, channel):
    """The repository manifest as CHANNEL publishes it: that channel's id, name and description
    (as tools/kpm-package.py writes them), with the same package and artifacts."""
    manifest = copy.deepcopy(incoming)
    manifest['id'] = 'grimoire' + channel
    manifest['name'] = 'Grimoire ' + channel
    manifest['description'] = 'Grimoire official ' + channel + ' channel'
    return manifest


def plan(target, data, artifact):
    """What writing DATA (holding ARTIFACT) into the channel folder TARGET does: 'updated',
    'unchanged' or 'kept newer'. The same version with other bytes or metadata is refused."""
    current = target / 'manifest.json'
    if not current.exists():
        return 'updated'
    old = json.loads(current.read_text())['packages']['grimoire']['artifacts'][0]
    if old['version'] > artifact['version']:
        return 'kept newer'
    if old['version'] == artifact['version']:
        if old != artifact:
            raise ValueError('An existing version cannot be replaced with different package bytes or metadata')
        if all((target / name).exists() and (target / name).read_text() == data
               for name in ('manifest.json', 'manifest.v2.json')):
            return 'unchanged'
    return 'updated'


def update(source, destination):
    """Write SOURCE (a channel manifest from tools/kpm-package.py) into the feed folder
    DESTINATION. Returns {channel: 'updated' | 'unchanged' | 'kept newer'}."""
    incoming = json.loads(source.read_text())
    artifact = incoming['packages']['grimoire']['artifacts'][0]
    version = artifact['grimoire_version']
    channel = 'preview' if '-' in version else 'stable'
    if incoming['id'] != 'grimoire' + channel or incoming['manifest_version'] != 2:
        raise ValueError('Invalid Grimoire channel manifest')
    manifests = {channel: incoming}
    if channel == 'stable':
        manifests['preview'] = channel_manifest(incoming, 'preview')
    data = {name: json.dumps(manifest, ensure_ascii=False, indent=2) + '\n' for name, manifest in manifests.items()}
    # Every channel is checked before any is written: a refusal leaves both as they were.
    results = {name: plan(destination / name, data[name], artifact) for name in manifests}
    for name, result in results.items():
        if result != 'updated':
            continue
        target = destination / name
        target.mkdir(parents=True, exist_ok=True)
        for filename in ('manifest.json', 'manifest.v2.json'):
            (target / filename).write_text(data[name])
    return results


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('source', type=Path)
    parser.add_argument('destination', type=Path)
    args = parser.parse_args()
    for name, result in update(args.source, args.destination).items():
        print(name + ': ' + result)
