#!/bin/sh
# Quire launcher, started by the documents/Quire.sh scriptlet. Pauses the Kindle UI the way
# KOReader does on upstart firmware (5.x, incl. 5.16+ hard-float and 5.19), runs Quire, and
# ALWAYS restores everything on exit.
#
#   quire.sh                      pause the UI (pillow off, awesome + statusbar stopped)
#   quire.sh --stop-framework     stop lab126_gui entirely, restart it afterwards
#   quire.sh --reset-calibration  forget the pen/touch calibration
#
# A flag file makes --stop-framework the default: touch /mnt/us/quire/stop-framework

DIR=$(cd "$(dirname "$0")" && pwd)
ROOT=/mnt/us/quire
LOG=$ROOT/quire.log
PORT=8765
mkdir -p "$ROOT"

if [ "$1" = "--reset-calibration" ]; then
    if [ -f "$ROOT/settings.ini" ]; then
        sed -i -e '/^pen\./d' -e '/^touch\./d' "$ROOT/settings.ini"
    fi
    exit 0
fi

# Only one Quire at a time.
if pidof quire >/dev/null 2>&1; then
    exit 0
fi

# Keep the log from growing forever.
if [ -f "$LOG" ] && [ "$(wc -c <"$LOG")" -gt 1048576 ]; then
    mv -f "$LOG" "$LOG.1"
fi
log() { echo "$(date '+%F %T') quire.sh: $*" >>"$LOG"; }

STOP_FRAMEWORK=no
[ "$1" = "--stop-framework" ] && STOP_FRAMEWORK=yes
[ -f "$ROOT/stop-framework" ] && STOP_FRAMEWORK=yes

AWESOME_STOPPED=no
PILLOW_DISABLED=no
VOLUMD_STOPPED=no
IPTABLES_ADDED=no
STOPPED_JOBS=""
RESTORED=no

restore() {
    [ "$RESTORED" = yes ] && return
    RESTORED=yes
    log "restoring the Kindle UI"
    if [ "$IPTABLES_ADDED" = yes ]; then
        iptables -D INPUT -i wlan0 -p tcp --dport "$PORT" -j ACCEPT 2>/dev/null
    fi
    if [ "$VOLUMD_STOPPED" = yes ]; then
        killall -CONT volumd 2>/dev/null
    fi
    lipc-set-prop com.lab126.powerd preventScreenSaver 0 2>/dev/null
    if [ "$STOP_FRAMEWORK" = yes ]; then
        cd / && start lab126_gui 2>/dev/null
        return
    fi
    for job in $STOPPED_JOBS; do
        start "$job" 2>/dev/null
    done
    if [ "$AWESOME_STOPPED" = yes ]; then
        killall -CONT awesome 2>/dev/null
    fi
    if [ "$PILLOW_DISABLED" = yes ]; then
        # Put back what was on screen before us, then let the home booklet repaint.
        if [ -f /var/tmp/quire-fb.dump ]; then
            cat /var/tmp/quire-fb.dump >/dev/fb0 2>/dev/null
            rm -f /var/tmp/quire-fb.dump
        fi
        lipc-set-prop com.lab126.pillow disableEnablePillow enable 2>/dev/null
        lipc-set-prop com.lab126.appmgrd start app://com.lab126.booklet.home 2>/dev/null
    fi
}
trap restore EXIT
trap 'restore; exit 1' INT TERM HUP

log "starting Quire (stop framework: $STOP_FRAMEWORK) on $(cat /etc/prettyversion.txt 2>/dev/null)"

# Launchers can start us niced; go back to normal priority for pen latency.
[ "$(nice)" -gt 0 ] 2>/dev/null && renice -n "-$(nice)" $$ >/dev/null 2>&1

if [ "$STOP_FRAMEWORK" = yes ]; then
    # The framework job sends SIGTERM on stop; do not let it take us down with it.
    trap "" TERM
    stop lab126_gui 2>/dev/null
    usleep 1250000 2>/dev/null || sleep 2
    trap 'restore; exit 1' INT TERM HUP
else
    cat /dev/fb0 >/var/tmp/quire-fb.dump 2>/dev/null
    lipc-set-prop com.lab126.pillow disableEnablePillow disable 2>/dev/null && PILLOW_DISABLED=yes
    # Pillow off is not enough to stop the clock / window manager repainting on 5.7.2+.
    if killall -STOP awesome 2>/dev/null; then AWESOME_STOPPED=yes; fi
    # On 5.17+ the status bar is its own upstart job and draws over everything.
    if [ -f /etc/upstart/statusbar.conf ]; then
        stop statusbar 2>/dev/null && STOPPED_JOBS="$STOPPED_JOBS statusbar"
    fi
fi

# Stay awake while Quire runs (it saves and exits on the power button).
lipc-set-prop com.lab126.powerd preventScreenSaver 1 2>/dev/null
# Keep USB mass storage from yanking /mnt/us out from under us.
if [ -e /etc/upstart/volumd.conf ] && killall -STOP volumd 2>/dev/null; then VOLUMD_STOPPED=yes; fi
# Let the Wi-Fi share screen be reached (the stock firewall drops incoming connections).
if iptables -A INPUT -i wlan0 -p tcp --dport "$PORT" -j ACCEPT 2>/dev/null; then IPTABLES_ADDED=yes; fi

# Give the Kindle UI a moment to finish repainting after the scriptlet returned, so its last
# frame lands before ours (Quire also re-checks the panel and repaints if it was drawn over).
usleep 900000 2>/dev/null || sleep 1

cd "$DIR" || exit 1
rm -f /tmp/quire.alive
QUIRE_ROOT="$ROOT" "$DIR/quire" >>"$LOG" 2>&1 &
QPID=$!
log "Quire running as pid $QPID"
# Watchdog: Quire touches /tmp/quire.alive every 2 s once its first frame is up. No first
# frame within 25 s, or a heartbeat older than 15 s, means it is stuck: kill it so the trap
# restores the Kindle UI instead of leaving the device paused.
START=$(date +%s)
while kill -0 "$QPID" 2>/dev/null; do
    sleep 2
    NOW=$(date +%s)
    if [ -f /tmp/quire.alive ]; then
        AGE=$((NOW - $(date -r /tmp/quire.alive +%s 2>/dev/null || echo "$NOW")))
        if [ "$AGE" -gt 15 ]; then
            log "watchdog: no heartbeat for ${AGE}s, killing Quire"
            kill "$QPID" 2>/dev/null; sleep 2; kill -9 "$QPID" 2>/dev/null
        fi
    elif [ $((NOW - START)) -gt 25 ]; then
        log "watchdog: no first frame after 25s, killing Quire"
        kill "$QPID" 2>/dev/null; sleep 2; kill -9 "$QPID" 2>/dev/null
    fi
done
wait "$QPID" 2>/dev/null
log "Quire exited with status $?"
rm -f /tmp/quire.alive
exit 0
