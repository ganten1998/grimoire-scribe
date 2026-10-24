#!/bin/sh
# Sourced by the KPM hooks. Never changes rootfs, PIN flags, KOReader or notebook data.
US=${GRIMOIRE_US:-/mnt/us}
TMP=${GRIMOIRE_TMP:-/tmp}
ROOT=$US/grimoire
DOCS=$US/documents
DIR=$(cd "$(dirname "$0")" && pwd) || exit 1
busy() {
    p=$(cat "$TMP/grimoire.launcher.pid" 2>/dev/null)
    case "$p" in '' | *[!0-9]*) return 1 ;; esac
    kill -0 "$p" 2>/dev/null
}
require_closed() {
    if busy; then
        echo 'Close Grimoire before installing, upgrading or removing it.' >&2
        exit 1
    fi
}
platform() {
    case "${KPM_PLATFORM:-kindlehf}" in kindlehf) ;; *) echo 'Grimoire requires kindlehf (a supported Scribe).' >&2; exit 1 ;; esac
}
