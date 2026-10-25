#!/usr/bin/env python3
"""Write a tar or zip whose bytes depend only on the files it holds and the commit.

Every entry gets one time: SOURCE_DATE_EPOCH if set, else the time of this repository's HEAD
commit, else 1980-01-01. Owners are root (uid and gid 0), directories and executable files get
mode 755 and other files 644, and nothing else of the builder's (no extra zip fields, no extended
attributes) goes in. The NAMEs go in the order given, and each directory's contents in name
order. So the same files make the same archive on any host, under any umask or time zone.
usage: archive.py tar|zip OUTPUT -C DIR NAME...
"""
import argparse
import io
import os
from pathlib import Path
import stat
import subprocess
import tarfile
import time
import zipfile

REPO = Path(__file__).resolve().parents[1]
DOS_EPOCH = 315532800  # 1980-01-01 00:00:00 UTC, the earliest time a zip can hold


def source_date_epoch():
    if os.environ.get('SOURCE_DATE_EPOCH'):
        return int(os.environ['SOURCE_DATE_EPOCH'])
    # Only this repository's own HEAD: a source tree without .git inside another repository
    # (a home folder kept in git) must not take that repository's time.
    try:
        top = subprocess.run(['git', '-C', str(REPO), 'rev-parse', '--show-toplevel'],
                             capture_output=True, text=True, check=True).stdout.strip()
        if Path(top).resolve() == REPO:
            return int(subprocess.run(['git', '-C', str(REPO), 'log', '-1', '--format=%ct'],
                                      capture_output=True, text=True, check=True).stdout)
    except (OSError, ValueError, subprocess.CalledProcessError):
        pass
    return DOS_EPOCH


def entries(root, names):
    """(path, archive name, is a directory) for each NAME under ROOT, directories before their
    contents, each directory's contents sorted by name."""
    for name in names:
        path = root / name
        if path.is_symlink() or not (path.is_file() or path.is_dir()):
            raise ValueError(f'{path}: not a regular file or directory')
        yield path, name, path.is_dir()
        if path.is_dir():
            yield from entries(root, [name + '/' + child for child in sorted(os.listdir(path))])


def mode(path, is_dir):
    return 0o755 if is_dir or path.stat().st_mode & stat.S_IXUSR else 0o644


def write_tar(stream, root, names, mtime):
    with tarfile.open(fileobj=stream, mode='w', format=tarfile.USTAR_FORMAT) as tar:
        for path, name, is_dir in entries(root, names):
            info = tarfile.TarInfo(name)
            info.type = tarfile.DIRTYPE if is_dir else tarfile.REGTYPE
            info.mode = mode(path, is_dir)
            info.mtime = mtime
            info.uid = info.gid = 0
            info.uname = info.gname = 'root'
            if is_dir:
                tar.addfile(info)
            else:
                data = path.read_bytes()
                info.size = len(data)
                tar.addfile(info, io.BytesIO(data))


def write_zip(stream, root, names, mtime):
    # A zip holds local time with no zone: write the UTC time, so every host writes the same.
    date_time = time.gmtime(max(mtime, DOS_EPOCH))[:6]
    with zipfile.ZipFile(stream, 'w') as archive:
        for path, name, is_dir in entries(root, names):
            info = zipfile.ZipInfo(name + '/' if is_dir else name, date_time)
            info.create_system = 3  # Unix, so the modes below apply
            info.external_attr = ((stat.S_IFDIR if is_dir else stat.S_IFREG) | mode(path, is_dir)) << 16
            if is_dir:
                info.external_attr |= 0x10  # the MS-DOS directory flag
                archive.writestr(info, b'', compress_type=zipfile.ZIP_STORED)
            else:
                archive.writestr(info, path.read_bytes(), compress_type=zipfile.ZIP_DEFLATED, compresslevel=9)


def main():
    parser = argparse.ArgumentParser(description=__doc__.split('\n', 1)[0])
    parser.add_argument('format', choices=('tar', 'zip'))
    parser.add_argument('output', type=Path)
    parser.add_argument('-C', dest='root', type=Path, required=True, help='the folder NAMEs are in')
    parser.add_argument('names', nargs='+')
    args = parser.parse_args()
    for name in args.names:
        if Path(name).is_absolute() or '..' in Path(name).parts:
            parser.error(f'{name}: must be a path inside {args.root}')
    # Written beside OUTPUT, then renamed: a failed run leaves no half-written archive.
    partial = args.output.with_name(args.output.name + '.tmp')
    try:
        with partial.open('wb') as stream:
            (write_tar if args.format == 'tar' else write_zip)(stream, args.root, args.names, source_date_epoch())
        partial.replace(args.output)
    except (OSError, ValueError) as error:
        partial.unlink(missing_ok=True)
        parser.exit(1, f'archive: {error}\n')


if __name__ == '__main__':
    main()
