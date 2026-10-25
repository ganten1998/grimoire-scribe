#!/usr/bin/env python3
"""Explicitly install the independent KindleLab SSH bridge over an already trusted SSH connection."""
import argparse
import ipaddress
from pathlib import Path
import re
import subprocess
import sys

REPO = Path(__file__).resolve().parents[1]


def host_name(value):
    try:
        ipaddress.ip_address(value)
    except ValueError:
        if not re.fullmatch(r'[A-Za-z0-9](?:[A-Za-z0-9.-]*[A-Za-z0-9])?', value) or '..' in value:
            raise argparse.ArgumentTypeError('Use an IP address or hostname, without a user, port or URL.')
    return value


def install_script(start=False):
    helper = (REPO / 'kindle/lab/ssh-bridge.sh').read_text()
    wrapper = (REPO / 'kindle/lab/KindleLab SSH.sh').read_text()
    tag = 'KINDLELAB_BRIDGE_CODE_74b956e2'
    assert tag not in helper and tag not in wrapper
    prefix = '''set -eu
umask 077
DIR=/mnt/us/kindle-lab/ssh
STATE=/var/local/kindle-lab-ssh
DOC='/mnt/us/documents/KindleLab SSH.sh'
p=$(cat "$STATE/listener.pid" 2>/dev/null || true)
case "$p" in ''|*[!0-9]*|0|1) ;; *) if kill -0 "$p" 2>/dev/null; then echo 'Stop the existing bridge before refreshing it.' >&2; exit 1; fi;; esac
if [ -e "$DIR/bridge.sh" ] && ! grep -q '^TAG=kindlelab-ssh$' "$DIR/bridge.sh"; then echo 'Existing bridge.sh is not ours; keeping it.' >&2; exit 1; fi
if [ -e "$DOC" ] && ! head -n 6 "$DOC" | grep -qx '# Author: KindleLab'; then echo 'Existing Library item is not ours; keeping it.' >&2; exit 1; fi
KO=
for candidate in /mnt/us/koreader /mnt/us/extensions/koreader /mnt/us/documents/koreader; do
    if [ -f "$candidate/reader.lua" ]; then KO=$candidate; break; fi
done
[ -n "$KO" ] || { echo 'KOReader not found; no files changed.' >&2; exit 1; }
mkdir -p "$DIR"
'''
    result = prefix + f'cat > "$DIR/bridge.sh.new" <<\'{tag}\'\n{helper}{tag}\n'
    result += '''chmod 700 "$DIR/bridge.sh.new"
sh "$DIR/bridge.sh.new" install "$KO"
mv -f "$DIR/bridge.sh.new" "$DIR/bridge.sh"
mkdir -p /mnt/us/documents
'''
    result += f'cat > "$DOC.new" <<\'{tag}\'\n{wrapper}{tag}\n'
    result += 'chmod 755 "$DOC.new"\nmv -f "$DOC.new" "$DOC"\n'
    if start:
        result += 'sh "$DIR/bridge.sh" start\n'
    return result.encode()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('host', type=host_name)
    parser.add_argument('--port', type=int, default=2222, help='Existing trusted bootstrap SSH port')
    parser.add_argument('--identity', type=Path, required=True, help='Private key path; its contents are never read by this tool')
    parser.add_argument('--known-hosts', type=Path, required=True, help='Existing trusted known_hosts file')
    parser.add_argument('--start', action='store_true', help='Start the key-only listener after installation')
    args = parser.parse_args()
    if not 1 <= args.port <= 65535 or not args.identity.is_file() or not args.known_hosts.is_file():
        parser.error('Use a valid port and existing identity/known_hosts files.')
    command = ['ssh', '-F', '/dev/null', '-T', '-p', str(args.port), '-i', str(args.identity),
               '-o', 'IdentitiesOnly=yes', '-o', 'IdentityAgent=none', '-o', 'BatchMode=yes',
               '-o', 'PasswordAuthentication=no', '-o', 'KbdInteractiveAuthentication=no',
               '-o', 'StrictHostKeyChecking=yes', '-o', 'ConnectTimeout=5',
               '-o', f'UserKnownHostsFile={args.known_hosts}', '--', f'root@{args.host}', 'sh -s']
    try:
        run = subprocess.run(command, input=install_script(args.start), timeout=30)
    except (OSError, subprocess.TimeoutExpired) as error:
        print(f'Bridge installation failed: {error}', file=sys.stderr)
        return 1
    return run.returncode


if __name__ == '__main__':
    raise SystemExit(main())
