#!/bin/sh
# Name: Grimoire
# Author: ganten
# Icon: @ICON@
# DontUseFBInk
#
# Scriptlet launcher (Vera / WinterBreak-era jailbreaks index .sh files in documents/ as books).
# Self-contained: the app is appended below and unpacked to /mnt/us/grimoire/app; see docs/KINDLE.md.
LOG=/mnt/us/grimoire/grimoire.log
APP=/mnt/us/grimoire/app
BUILD_ID=@BUILD_ID@
log() { echo "$(date '+%F %T') Grimoire.sh: $*" >>"$LOG"; }

# Grimoire used to be called Quire. Its notebooks and settings move over once, before anything
# creates the new folder; the old launcher (which would start the old app) is retired.
OLD=/mnt/us/quire
if [ -d "$OLD" ] && [ ! -e /mnt/us/grimoire ]; then
    p=$(cat /tmp/quire.launcher.pid 2>/dev/null)
    if [ -n "$p" ] && kill -0 "$p" 2>/dev/null; then
        exit 0  # the old app is still running: try again after leaving it
    fi
    if mv "$OLD" /mnt/us/grimoire; then
        [ -f /mnt/us/grimoire/quire.log ] && mv -f /mnt/us/grimoire/quire.log "$LOG"
        log "moved the data from $OLD to /mnt/us/grimoire"
    fi
fi
for f in /mnt/us/documents/Quire.sh "/mnt/us/documents/Quire - reset pen calibration.sh"; do
    if [ -f "$f" ] && grep -q "/mnt/us/quire" "$f" 2>/dev/null; then
        rm -f "$f" && log "removed the old launcher $f"
    fi
done
mkdir -p "$APP"

# This file carries the app (a tar archive after the marker line at the end). Unpack it on first
# run, and again whenever this file is replaced by a newer build.
if [ ! -f "$APP/grimoire" ] || [ "$(cat "$APP/BUILD_ID" 2>/dev/null)" != "$BUILD_ID" ]; then
    LINE=$(awk '/^__GRIMOIRE_APP_ARCHIVE__$/ { print NR + 1; exit }' "$0")
    if [ -z "$LINE" ] || ! tail -n +"$LINE" "$0" | tar -xf - -C "$APP"; then
        log "could not unpack the app from $0 into $APP"
        exit 1
    fi
    echo "$BUILD_ID" >"$APP/BUILD_ID"
    log "unpacked build $BUILD_ID into $APP"
fi
chmod +x "$APP/grimoire" "$APP/grimoire.sh" 2>/dev/null
# Detach, so Grimoire keeps running whatever the scriptlet host does with this process.
if command -v setsid >/dev/null 2>&1; then
    setsid sh "$APP/grimoire.sh" "$@" </dev/null >/dev/null 2>&1 &
else
    nohup sh "$APP/grimoire.sh" "$@" </dev/null >/dev/null 2>&1 &
fi
exit 0
