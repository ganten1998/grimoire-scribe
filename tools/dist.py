#!/usr/bin/env python3
"""Collect installable builds in a version/commit folder, without overwriting older builds."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import shutil
import subprocess
import tempfile
from urllib.parse import urlparse

REPO = Path(__file__).resolve().parents[1]


def digest(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def git(*args):
    return subprocess.check_output(['git', '-C', str(REPO), *args], text=True).strip()


def collect(build, output):
    version = (REPO / 'VERSION').read_text().strip()
    if not re.fullmatch(r'\d+\.\d+\.\d+(?:-(?:beta|rc)\.\d+)?', version):
        raise ValueError('Unexpected version; refusing to construct an output path.')
    manifest = json.loads((build / 'kpm/manifest.json').read_text())
    artifact, = manifest['packages']['grimoire']['artifacts']
    filename = Path(urlparse(artifact['url']).path).name
    if artifact['grimoire_version'] != version or not re.fullmatch(r'grimoire_\d+\.\d+\.\d+_kindlehf\.kpkg', filename):
        raise ValueError('KPM manifest does not match this build.')
    if digest(build / 'kpm' / filename) != artifact['sha256']:
        raise ValueError('KPM package checksum does not match its manifest.')
    revision = git('rev-parse', 'HEAD')
    dirty = bool(git('status', '--porcelain', '--untracked-files=no'))
    name = f'Grimoire-{version}-{revision[:12]}' + ('-dirty' if dirty else '')
    output.mkdir(parents=True, exist_ok=True)
    stage = Path(tempfile.mkdtemp(prefix='.grimoire-', dir=output))
    try:
        for source in (build / 'dist' / f'Grimoire-{version}.zip', build / 'dist/KOReader-patches.zip',
                       build / 'kpm' / filename, build / 'kpm/manifest.json', build / 'kpm/manifest.v2.json'):
            shutil.copy2(source, stage / source.name)
        (stage / 'SOURCE.json').write_text(json.dumps({
            'version': version, 'revision': revision, 'dirty': dirty,
            'branch': git('branch', '--show-current'),
            'notice': 'Local build; collection does not publish a release or a KPM feed.',
        }, indent=2) + '\n')
        (stage / 'SHA256SUMS.txt').write_text(''.join(
            f'{digest(path)}  {path.name}\n' for path in sorted(stage.iterdir())))
        destination = output / name
        if destination.exists():
            if destination.is_symlink() or not destination.is_dir() or sorted(p.name for p in destination.iterdir()) != sorted(p.name for p in stage.iterdir()):
                raise FileExistsError(f'Keeping different existing output: {destination}')
            if any(not (destination / p.name).is_file() or (destination / p.name).is_symlink() or digest(destination / p.name) != digest(p) for p in stage.iterdir()):
                raise FileExistsError(f'Keeping different existing output: {destination}')
        else:
            stage.rename(destination)
        return destination
    finally:
        if stage.exists():
            shutil.rmtree(stage)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--build', type=Path, default=Path('build'))
    parser.add_argument('--output', type=Path, default=Path('dist'))
    args = parser.parse_args()
    try:
        print(collect(args.build, args.output))
    except (OSError, ValueError, KeyError, subprocess.SubprocessError) as error:
        parser.exit(1, f'dist: {error}\n')


if __name__ == '__main__':
    main()
