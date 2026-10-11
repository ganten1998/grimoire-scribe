#!/bin/sh
# Name: Quire
# Author: ganten
# Icon: @ICON@
# DontUseFBInk
#
# Scriptlet launcher (Vera / WinterBreak-era jailbreaks index .sh files in documents/ as books).
# Self-contained: the app is appended below and unpacked to /mnt/us/quire/app; see docs/KINDLE.md.
LOG=/mnt/us/quire/quire.log
APP=/mnt/us/quire/app
BUILD_ID=@BUILD_ID@
mkdir -p "$APP"
log() { echo "$(date '+%F %T') Quire.sh: $*" >>"$LOG"; }

# This file carries the app (a tar archive after the marker line at the end). Unpack it on first
# run, and again whenever this file is replaced by a newer build.
if [ ! -f "$APP/quire" ] || [ "$(cat "$APP/BUILD_ID" 2>/dev/null)" != "$BUILD_ID" ]; then
    LINE=$(awk '/^__QUIRE_APP_ARCHIVE__$/ { print NR + 1; exit }' "$0")
    if [ -z "$LINE" ] || ! tail -n +"$LINE" "$0" | tar -xf - -C "$APP"; then
        log "could not unpack the app from $0 into $APP"
        exit 1
    fi
    echo "$BUILD_ID" >"$APP/BUILD_ID"
    log "unpacked build $BUILD_ID into $APP"
fi
chmod +x "$APP/quire" "$APP/quire.sh" 2>/dev/null
# Detach, so Quire keeps running whatever the scriptlet host does with this process.
if command -v setsid >/dev/null 2>&1; then
    setsid sh "$APP/quire.sh" "$@" </dev/null >/dev/null 2>&1 &
else
    nohup sh "$APP/quire.sh" "$@" </dev/null >/dev/null 2>&1 &
fi
exit 0
