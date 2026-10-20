#!/bin/sh
# Name: Grimoire
# Author: Ganten
# Icon: @ICON@
# DontUseFBInk
# Capabilities: guard unpack-only hosted
#
# Scriptlet launcher (Vera / WinterBreak-era jailbreaks index .sh files in documents/ as books).
# Self-contained: the app is appended below and unpacked to /mnt/us/grimoire/app; see docs/KINDLE.md.
# SH_Integration reads its keys (Name, Author, Icon, DontUseFBInk) from the first six lines only,
# so those four stay on lines 2 to 5.
#
#   Grimoire.sh                      unpack the app if this file is newer, then start Grimoire
#   Grimoire.sh --unpack-only        only unpack, start nothing
#   Grimoire.sh --reset-calibration  forget the pen and touch calibration
#   Grimoire.sh --hosted koreader    unpack, then run Grimoire inside KOReader's session, in the
#                                    foreground (app/grimoire.sh says how); only inside KOReader
#   Other arguments go on to app/grimoire.sh (--stop-framework).
# Exit status: 0 done (for a start: Grimoire is on its way, or the old Quire app is still running
# and nothing happens); 1 the app could not be unpacked (or the calibration not reset); 2
# --unpack-only or --reset-calibration did nothing because the old Quire app is still running;
# 3 refused inside KOReader, or --hosted outside it. Past the unpack, --hosted answers with
# app/grimoire.sh's status.
#
# Capabilities lists what this launcher supports, for a tool (the KOReader plugin) that must
# never run a launcher to find out: an older one ignores a flag it does not know and starts
# Grimoire. guard: it refuses to start inside KOReader (exit 3); unpack-only: --unpack-only;
# hosted: --hosted koreader. A tool reads the line in the very documents/Grimoire.sh it is going
# to run, from its first ten lines and never by searching the whole file (the app appended below
# holds app/grimoire.sh, whose own Capabilities line is about that file):
#   head -n 10 documents/Grimoire.sh | sed -n 's/^# Capabilities: //p'
# and runs --unpack-only only when the line lists unpack-only. The payload carries the same
# words in app/CAPABILITIES, but that file is to be trusted only after such an --unpack-only
# succeeded: an older launcher's unpack (1.0.0-beta has no such file) leaves a newer one behind.
# app/grimoire.sh names its own capabilities in its header.
#
# The app carries KOReader's Grimoire plugin (app/koreader/grimoire.koplugin). A start from the
# Library installs it into KOReader, when KOReader is there, and keeps it up to date; see
# koplugin below.

# Device paths. tests/launcher_check.sh moves them into a scratch tree; on the Kindle they are
# never set.
US=${GRIMOIRE_US:-/mnt/us}
TMP=${GRIMOIRE_TMP:-/tmp}
LOG=$US/grimoire/grimoire.log
APP=$US/grimoire/app
BUILD_ID=@BUILD_ID@
# Never creates the grimoire folder: the move from Quire below needs it missing.
log() { [ -d "$US/grimoire" ] && echo "$(date '+%F %T') Grimoire.sh: $*" >>"$LOG"; }

# KOReader keeps the pen and the touch screen to itself while it runs (an exclusive grab), so a
# Grimoire started under it gets neither, and its exit would bring the Kindle UI back underneath
# KOReader. Everything KOReader starts inherits KOREADER_DIR (its file browser's Execute shell
# script runs this file); a reader.lua still running is how koreader.sh itself tells that
# KOReader is up. Unpacking and the calibration reset touch neither the screen nor the input, so
# they are allowed. app/grimoire.sh has the same check. --hosted is the other way round: KOReader's
# Grimoire plugin runs it from KOReader's exit path, after KOReader let go of the screen and the
# input, so it runs only inside KOReader (app/grimoire.sh checks the rest: that koreader.sh
# started this KOReader and paused the Kindle UI).
case "$1" in
--unpack-only | --reset-calibration) ;;
--hosted)
    if [ -z "$KOREADER_DIR" ]; then
        MSG="not starting: --hosted is for KOReader's Grimoire plugin, inside KOReader."
        log "$MSG"
        echo "Grimoire: $MSG" >&2
        exit 3
    fi
    ;;
*)
    MSG=""
    if [ -n "$KOREADER_DIR" ]; then
        MSG="not starting inside KOReader, which holds the pen and the touch screen: quit KOReader, then tap Grimoire in the Library."
    elif KO_PID=$(pidof reader.lua 2>/dev/null); then
        MSG="not starting: KOReader is still running (reader.lua, pid $KO_PID) and holds the pen and the touch screen. Quit it, or restart the Kindle if it is not open, then tap Grimoire in the Library."
    fi
    if [ -n "$MSG" ]; then
        log "$MSG"
        echo "Grimoire: $MSG" >&2
        # Tapped in the Library while KOReader lingers: say it on the screen too, once the
        # Library has repainted (the scriptlet host refreshes the screen when this script ends).
        # Inside KOReader, KOReader shows the exit status itself.
        if [ -z "$KOREADER_DIR" ] && command -v eips >/dev/null 2>&1; then
            (usleep 2000000 2>/dev/null || sleep 2; eips 1 3 "Grimoire did not start: KOReader is running.") \
                </dev/null >/dev/null 2>&1 &
        fi
        exit 3
    fi
    ;;
esac

# Grimoire used to be called Quire. Its notebooks and settings move over once, before anything
# creates the new folder; the old launcher (which would start the old app) is retired.
OLD=$US/quire
if [ -d "$OLD" ] && [ ! -e "$US/grimoire" ]; then
    p=$(cat "$TMP/quire.launcher.pid" 2>/dev/null)
    if [ -n "$p" ] && kill -0 "$p" 2>/dev/null; then
        # The old app is still running: try again after leaving it. There is no grimoire folder
        # to log to yet, so a flag's caller learns it from the status.
        case "$1" in
        --unpack-only | --reset-calibration | --hosted)
            echo "Grimoire: the old Quire app is still running. Leave it, then try again." >&2
            exit 2
            ;;
        esac
        exit 0
    fi
    if mv "$OLD" "$US/grimoire"; then
        [ -f "$US/grimoire/quire.log" ] && mv -f "$US/grimoire/quire.log" "$LOG"
        log "moved the data from $OLD to $US/grimoire"
    fi
fi
for f in "$US/documents/Quire.sh" "$US/documents/Quire - reset pen calibration.sh"; do
    # The old launchers always named their folder as /mnt/us/quire.
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
case "$1" in
--unpack-only) exit 0 ;;
# Quick, and its status is the caller's answer: no need to detach.
--reset-calibration) exec sh "$APP/grimoire.sh" --reset-calibration ;;
# In the foreground: KOReader's exit path waits for Grimoire to end, then starts KOReader again.
--hosted) exec sh "$APP/grimoire.sh" "$@" ;;
esac

# KOReader's Grimoire plugin (Tools > Grimoire in KOReader), from app/koreader. With KOReader
# installed, a start from the Library copies it into KOReader's plugins folder once, then keeps
# that copy up to date: grimoire/.koplugin-installed holds the build it came from. A plugin folder
# the user deleted stays deleted (the marker then reads "removed"); one copied back by hand, from
# the zip's "KOReader plugin (optional)" folder, is kept up to date again. KOReader is not running
# here (the guard above), so it never loads a half-copied plugin, and each file goes in whole
# (a copy, then a rename).
koplugin() {
    src=$APP/koreader/grimoire.koplugin
    dst=$US/koreader/plugins/grimoire.koplugin
    mark=$US/grimoire/.koplugin-installed
    { [ -f "$US/koreader/reader.lua" ] && [ -d "$src" ]; } || return 0
    was=$(cat "$mark" 2>/dev/null)
    if [ -e "$mark" ] && [ ! -d "$dst" ]; then
        if [ "$was" != removed ]; then
            echo removed >"$mark"
            log "KOReader's Grimoire plugin was removed from $dst: not installing it again"
        fi
        return 0
    fi
    [ "$was" = "$BUILD_ID" ] && return 0
    if ! mkdir -p "$dst" 2>/dev/null; then
        log "could not create $dst for KOReader's Grimoire plugin"
        return 1
    fi
    for f in "$src"/*; do
        [ -f "$f" ] || continue
        n=${f##*/}
        if ! { cp -f "$f" "$dst/.$n.tmp" && mv -f "$dst/.$n.tmp" "$dst/$n"; } 2>/dev/null; then
            rm -f "$dst/.$n.tmp"
            log "could not copy $n of KOReader's Grimoire plugin into $dst"
            return 1
        fi
    done
    echo "$BUILD_ID" >"$mark"
    if [ -z "$was" ]; then
        log "installed KOReader's Grimoire plugin into $dst (Tools > Grimoire in KOReader)"
    else
        log "updated KOReader's Grimoire plugin in $dst"
    fi
}
koplugin

# Detach, so Grimoire keeps running whatever the scriptlet host does with this process.
if command -v setsid >/dev/null 2>&1; then
    setsid sh "$APP/grimoire.sh" "$@" </dev/null >/dev/null 2>&1 &
else
    nohup sh "$APP/grimoire.sh" "$@" </dev/null >/dev/null 2>&1 &
fi
exit 0
