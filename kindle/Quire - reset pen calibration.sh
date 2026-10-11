#!/bin/sh
# Name: Quire - reset pen calibration
# Author: ganten
#
# Forgets Quire's pen + touch calibration (use it if strokes land far from the pen tip and
# the in-app calibration is hard to reach). Output is shown on screen.
S=/mnt/us/quire/settings.ini
if [ -f "$S" ]; then
    sed -i -e '/^pen\./d' -e '/^touch\./d' "$S"
    echo "Quire: pen and touch calibration reset."
else
    echo "Quire: nothing to reset (no settings yet)."
fi
