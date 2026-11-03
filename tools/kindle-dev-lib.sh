# kindle-dev-lib.sh: what tools/kindle-dev and tools/kindle-dev-ssh share: the dev copy's name and
# root, where make package put the zip, the --name/--root checks, and the patch that turns the
# release zip's documents/Grimoire.sh into the dev launcher. Sourced by both, not run on its own.
#
# The patched launcher keeps every line of the release's but three: "# Name:" names the dev copy,
# GRIMOIRE_US roots its data at the dev root (overridable with GRIMOIRE_DEV_US when the launcher
# runs, so a test can point it at a scratch tree), and GRIMOIRE_DEV=1 tells kindle/app/grimoire.sh
# to turn updates off: an update from the dev copy would run KPM over the release.

KINDLE_DEV_NAME='Grimoire (dev)'
KINDLE_DEV_ROOT=/mnt/us/grimoire-dev

# kindle_dev_zip ROOT BUILD: the zip make package writes, for make's BUILD folder (absolute, or
# relative to the repository root).
kindle_dev_zip() {
    case "$2" in
    /*) printf '%s/dist/Grimoire.zip\n' "$2" ;;
    *) printf '%s/%s/dist/Grimoire.zip\n' "$1" "$2" ;;
    esac
}

# kindle_dev_validate TOOL NAME ROOT: refuse a --name that would write over a release launcher
# (documents/Grimoire.sh or the calibration reset, which FAT's case-insensitive names make any
# spelling of), and a --root that is not strictly under /mnt/us (the dev copy must never point at
# the release's data, nor at /tmp or the rootfs). Prints the reason and fails.
kindle_dev_validate() {
    python3 -I - "$1" "$2" "$3" <<'PY'
import re
import sys

tool, name, root = sys.argv[1:4]


def die(msg):
    raise SystemExit(f"{tool}: {msg}")


if not name or "/" in name or name.startswith(".") or any(c in name for c in "\r\n\t"):
    die("--name must be a single path-free name")
if name.strip().lower() in ("grimoire", "grimoire - reset pen calibration"):
    die(f"--name {name!r} would write over the release launcher")
if not re.fullmatch(r"/[A-Za-z0-9._/-]+", root) or root.endswith("/"):
    die("--root must be an absolute path with letters, digits, . _ - or /")
if ".." in root.split("/"):
    die("--root must not contain ..")
if not root.startswith("/mnt/us/"):
    die("--root must be strictly under /mnt/us/")
PY
}

# kindle_dev_patch ZIP OUT NAME ROOT: write the dev launcher to OUT (the zip's own
# documents/Grimoire.sh: renamed, rooted at ROOT, updates off) and check that its head parses.
# Everything behind the patch, the appended app archive, comes from the zip untouched.
kindle_dev_patch() {
    python3 -I - "$1" "$2" "$3" "$4" <<'PY'
import subprocess
import sys
import tempfile
import zipfile

zip_path, out_path, name, dev_root = sys.argv[1:5]
with zipfile.ZipFile(zip_path) as archive:
    data = archive.read("documents/Grimoire.sh")
marker = b"\n__GRIMOIRE_APP_ARCHIVE__\n"
head, sep, tail = data.partition(marker)
if not sep or not head.startswith(b"#!/bin/sh\n# Name: Grimoire\n"):
    raise SystemExit("not a Grimoire installer")
if head.count(b"# Name: Grimoire\n") != 1:
    raise SystemExit("launcher name line is not unique")
needle = b"US=${GRIMOIRE_US:-/mnt/us}\n"
if head.count(needle) != 1:
    raise SystemExit("launcher data-root line is not unique")
head = head.replace(b"# Name: Grimoire\n", b"# Name: " + name.encode() + b"\n", 1)
insert = (
    b"GRIMOIRE_US=${GRIMOIRE_DEV_US:-" + dev_root.encode() + b"}\n"
    b"GRIMOIRE_DEV=1\n"
    b"export GRIMOIRE_US GRIMOIRE_DEV\n"
)
head = head.replace(needle, insert + needle, 1)
with open(out_path, "wb") as out:
    out.write(head + sep + tail)
# The launcher code before the appended archive is the part a syntax check can see.
with tempfile.NamedTemporaryFile() as tmp:
    tmp.write(head + b"\n")
    tmp.flush()
    if subprocess.run(["sh", "-n", tmp.name]).returncode != 0:
        raise SystemExit("the generated launcher does not parse")
PY
}
