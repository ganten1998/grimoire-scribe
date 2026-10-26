#!/bin/sh
# update.sh: Grimoire's updates through KPM (Settings > Updates; docs/KPM.md, Updates).
#   check preview|stable  puts that channel's feed in KPM (and takes the other channel's away: KPM
#                         installs the newest Grimoire any feed it knows lists), refreshes KPM's
#                         index, and prints `kpm search grimoire`. Exit 3: no KPM; 4: the feed could
#                         not be added; 5: the index could not be refreshed (no network, mostly).
#   install PID [hosted]  run by the launcher as it leaves (from a copy in /tmp, as the update
#                         replaces this file): once the launcher PID is gone (KPM's hooks refuse
#                         while it runs), installs the newest Grimoire with KPM, showing its
#                         progress on the screen, and starts Grimoire again, unless KOReader had
#                         started it (KOReader is back by then). A failed install writes
#                         grimoire/.update-failed, which Grimoire tells about at its next start.
# GRIMOIRE_KPM (tests only): the kpm to use. GRIMOIRE_ROOT: the data folder (default /mnt/us/grimoire).
set -u
KPM=${GRIMOIRE_KPM:-/var/local/kmc/bin/kpm}
ROOT=${GRIMOIRE_ROOT:-/mnt/us/grimoire}
PREVIEW=https://raw.githubusercontent.com/ganten1998/grimoire-scribe/kpm/preview/manifest.json
STABLE=https://raw.githubusercontent.com/ganten1998/grimoire-scribe/kpm/stable/manifest.json

upd_check() {
    case "${1:-}" in
    preview) url=$PREVIEW other=grimoirestable ;;
    stable) url=$STABLE other=grimoirepreview ;;
    *) echo "usage: update.sh check preview|stable" >&2; exit 2 ;;
    esac
    [ -x "$KPM" ] || { echo "update.sh: no KPM at $KPM" >&2; exit 3; }
    repos=$("$KPM" list-repo 2>&1)
    case $repos in
    *"($url)"*) ;;
    *) "$KPM" add-repo "$url" >&2 || exit 4 ;;
    esac
    # (The preview feed carries the stable releases too: on preview, the stable feed may stay.)
    if [ "$other" = grimoirepreview ]; then
        case $repos in *"- grimoirepreview -"*) "$KPM" remove-repo grimoirepreview >&2 ;; esac
    fi
    "$KPM" update >&2 || exit 5
    "$KPM" search grimoire
}

upd_install() {
    pid=${1:-}
    hosted=${2:-}
    case $pid in '' | *[!0-9]*) echo "usage: update.sh install PID [hosted]" >&2; exit 2 ;; esac
    echo "update.sh: waiting for the launcher to finish"
    i=0
    while kill -0 "$pid" 2>/dev/null && [ $i -lt 300 ]; do
        i=$((i + 1))
        sleep 0.2
    done
    echo "update.sh: installing the newest Grimoire"
    rm -f "$ROOT/.update-failed"
    if ! "$KPM" --fbink -y install grimoire > /tmp/grimoire-update.log 2>&1; then
        echo "update.sh: KPM could not install it (its output: /tmp/grimoire-update.log)"
        tail -n 20 /tmp/grimoire-update.log > "$ROOT/.update-failed" 2>/dev/null
    fi
    cat /tmp/grimoire-update.log 2>/dev/null
    if [ -n "$hosted" ]; then
        echo "update.sh: KOReader started Grimoire; it opens the new one next time"
        exit 0
    fi
    exec "$KPM" launch grimoire
}

case "${1:-}" in
check) shift; upd_check "$@" ;;
install) shift; upd_install "$@" ;;
*) echo "usage: update.sh check preview|stable | install PID [hosted]" >&2; exit 2 ;;
esac
