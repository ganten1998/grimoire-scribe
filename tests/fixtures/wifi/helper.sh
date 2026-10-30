#!/bin/sh
# Scenario-only radio, private to the synthetic data root.
set -eu
file=$GRIMOIRE_ROOT/wifi-state
state=$(cat "$file" 2>/dev/null || echo 0)
case "$1" in on) state=1; echo "$state" > "$file";; off) state=0; echo "$state" > "$file";; status) ;; *) exit 1;; esac
printf 'wireless=%s\nenabled=%s\nconnected=%s\n' "$state" "$state" "$state"
