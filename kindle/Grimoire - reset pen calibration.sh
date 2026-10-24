#!/bin/sh
# Name: Grimoire - reset pen calibration
# Author: Ganten
#
# Forgets Grimoire's pen + touch calibration (use it if strokes land far from the pen tip and
# the in-app calibration is hard to reach). Output is shown on screen.
# A copy and a rename rather than sed -i, which busybox and BSD sed read differently.
# GRIMOIRE_US is only ever set by tests/launcher_check.sh.
p=$(cat "${GRIMOIRE_TMP:-/tmp}/grimoire.launcher.pid" 2>/dev/null)
case "$p" in
'' | *[!0-9]*) ;;
*) if kill -0 "$p" 2>/dev/null; then
    echo 'Grimoire: close Grimoire before resetting calibration.' >&2
    exit 2
fi ;;
esac
S=${GRIMOIRE_US:-/mnt/us}/grimoire/settings.ini
if [ ! -f "$S" ]; then
    echo "Grimoire: nothing to reset (no settings yet)."
elif sed -e '/^pen\./d' -e '/^touch\./d' "$S" >"$S.reset" && mv -f "$S.reset" "$S"; then
    echo "Grimoire: pen and touch calibration reset."
else
    rm -f "$S.reset"
    echo "Grimoire: could not reset the calibration."
fi
