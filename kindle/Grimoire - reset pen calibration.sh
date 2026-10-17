#!/bin/sh
# Name: Grimoire - reset pen calibration
# Author: ganten
#
# Forgets Grimoire's pen + touch calibration (use it if strokes land far from the pen tip and
# the in-app calibration is hard to reach). Output is shown on screen.
S=/mnt/us/grimoire/settings.ini
if [ -f "$S" ]; then
    sed -i -e '/^pen\./d' -e '/^touch\./d' "$S"
    echo "Grimoire: pen and touch calibration reset."
else
    echo "Grimoire: nothing to reset (no settings yet)."
fi
