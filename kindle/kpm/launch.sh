#!/bin/sh
set -eu
DIR=$(cd "$(dirname "$0")" && pwd) || exit 1
# Forward --unpack-only, --reset-calibration and --hosted as well as an ordinary Library launch.
exec sh "$DIR/payload/Grimoire.sh" "$@"
