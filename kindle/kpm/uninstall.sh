#!/bin/sh
set -eu
. "$(dirname "$0")/common.sh"
require_closed
# Keep launchers in place during KPM's upgrade sequence. The new install replaces them atomically.
[ "${1:-}" = upgrade ] && exit 0
for name in Grimoire.sh 'Grimoire - reset pen calibration.sh'; do
    if cmp -s "$DIR/scriptlets/$name" "$DOCS/$name"; then rm -f "$DOCS/$name"; fi
done
# app/ is an unpack cache, also used by a manual installation or an explicitly installed plugin.
# Keep it and ALL user data. KPM itself removes this package directory.
echo 'KPM launchers removed. grimoire/ notebooks, settings and app cache were kept.'
