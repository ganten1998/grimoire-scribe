#!/bin/sh
# Grimoire launcher, started by the documents/Grimoire.sh scriptlet. Pauses the Kindle UI the way
# KOReader does on upstart firmware (5.x, incl. 5.16+ hard-float and 5.19), runs Grimoire, and
# ALWAYS restores everything on exit.
#
#   grimoire.sh                      pause the UI (pillow off, awesome + statusbar stopped)
#   grimoire.sh --stop-framework     stop lab126_gui entirely, restart it afterwards
#   grimoire.sh --reset-calibration  forget the pen/touch calibration
#
# A flag file makes --stop-framework the default: touch /mnt/us/grimoire/stop-framework

DIR=$(cd "$(dirname "$0")" && pwd)
ROOT=/mnt/us/grimoire
LOG=$ROOT/grimoire.log
mkdir -p "$ROOT"
# The share server's port (Settings may change it; the firewall hole must match).
PORT=$(sed -n 's/^http_port=\([0-9][0-9]*\)$/\1/p' "$ROOT/settings.ini" 2>/dev/null | tail -n 1)
[ -n "$PORT" ] || PORT=8765

if [ "$1" = "--reset-calibration" ]; then
    if [ -f "$ROOT/settings.ini" ]; then
        sed -i -e '/^pen\./d' -e '/^touch\./d' "$ROOT/settings.ini"
    fi
    exit 0
fi

log() { echo "$(date '+%F %T') grimoire.sh: $*" >>"$LOG"; }

# Only one Grimoire at a time. The lock is this launcher's pid, not `pidof grimoire`: a background PDF
# export can outlive Grimoire (and shares its name), and a launcher that is still restoring the
# Kindle UI after Grimoire exited must finish first, or its restore lands on top of the new Grimoire.
LOCK=/tmp/grimoire.launcher.pid
lock_holder() {
    p=$(cat "$LOCK" 2>/dev/null)
    [ -n "$p" ] && [ "$p" != "$$" ] && kill -0 "$p" 2>/dev/null && grep -q grimoire.sh "/proc/$p/cmdline" 2>/dev/null && echo "$p"
}
TRIES=0
while HOLDER=$(lock_holder) && [ -n "$HOLDER" ]; do
    TRIES=$((TRIES + 1))
    if [ "$TRIES" -gt 20 ]; then
        log "already running (launcher pid $HOLDER), not starting another"
        exit 0
    fi
    usleep 500000 2>/dev/null || sleep 1
done
echo "$$" >"$LOCK"
release() { [ "$(cat "$LOCK" 2>/dev/null)" = "$$" ] && rm -f "$LOCK"; }

# Keep the log from growing forever.
if [ -f "$LOG" ] && [ "$(wc -c <"$LOG")" -gt 1048576 ]; then
    mv -f "$LOG" "$LOG.1"
fi

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
    # Grimoire's power-service watcher, should a crash have left it behind.
    pkill -f "lipc-wait-event com.lab126.powerd goingToScreenSaver" 2>/dev/null
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
        if [ -f /var/tmp/grimoire-fb.dump ]; then
            cat /var/tmp/grimoire-fb.dump >/dev/fb0 2>/dev/null
            rm -f /var/tmp/grimoire-fb.dump
        fi
        lipc-set-prop com.lab126.pillow disableEnablePillow enable 2>/dev/null
        lipc-set-prop com.lab126.appmgrd start app://com.lab126.booklet.home 2>/dev/null
    fi
}
QPID=""
stop_grimoire() {
    [ -n "$QPID" ] || return
    kill "$QPID" 2>/dev/null
    sleep 2
    kill -9 "$QPID" 2>/dev/null
    wait "$QPID" 2>/dev/null
}
trap 'restore; release' EXIT
# Never hand the screen back to the Kindle while Grimoire still has it.
trap 'stop_grimoire; restore; exit 1' INT TERM HUP

log "starting Grimoire (stop framework: $STOP_FRAMEWORK) on $(cat /etc/prettyversion.txt 2>/dev/null)"

# Launchers can start us niced; go back to normal priority for pen latency.
[ "$(nice)" -gt 0 ] 2>/dev/null && renice -n "-$(nice)" $$ >/dev/null 2>&1

if [ "$STOP_FRAMEWORK" = yes ]; then
    # The framework job sends SIGTERM on stop; do not let it take us down with it.
    trap "" TERM
    stop lab126_gui 2>/dev/null
    usleep 1250000 2>/dev/null || sleep 2
    trap 'stop_grimoire; restore; exit 1' INT TERM HUP
else
    cat /dev/fb0 >/var/tmp/grimoire-fb.dump 2>/dev/null
    lipc-set-prop com.lab126.pillow disableEnablePillow disable 2>/dev/null && PILLOW_DISABLED=yes
    # Pillow off is not enough to stop the clock / window manager repainting on 5.7.2+.
    if killall -STOP awesome 2>/dev/null; then AWESOME_STOPPED=yes; fi
    # On 5.17+ the status bar is its own upstart job and draws over everything.
    # On 5.17+ the status bar (clock, battery) is its own upstart job and draws over everything.
    # Its name varies by device/firmware, so stop every job that looks like one.
    for conf in /etc/upstart/*status*.conf /etc/upstart/*chrome*.conf; do
        [ -f "$conf" ] || continue
        job=$(basename "$conf" .conf)
        if stop "$job" >/dev/null 2>&1; then
            STOPPED_JOBS="$STOPPED_JOBS $job"
            log "stopped job $job"
        fi
    done
    # For diagnosis: what is still running that could draw (logged once per launch).
    log "pillow disabled: $PILLOW_DISABLED, awesome stopped: $AWESOME_STOPPED"
    log "upstart jobs running: $(initctl list 2>/dev/null | grep -i 'start/running' | cut -d' ' -f1 | tr '\n' ' ')"
fi

# Stay awake while Grimoire runs (it saves and exits on the power button).
lipc-set-prop com.lab126.powerd preventScreenSaver 1 2>/dev/null
# Keep USB mass storage from yanking /mnt/us out from under us.
if [ -e /etc/upstart/volumd.conf ] && killall -STOP volumd 2>/dev/null; then VOLUMD_STOPPED=yes; fi
# Let the Wi-Fi share screen be reached (the stock firewall drops incoming connections).
if iptables -A INPUT -i wlan0 -p tcp --dport "$PORT" -j ACCEPT 2>/dev/null; then IPTABLES_ADDED=yes; fi

# Give the Kindle UI a moment to finish repainting after the scriptlet returned, so its last
# frame lands before ours (Grimoire also re-checks the panel and repaints if it was drawn over).
usleep 900000 2>/dev/null || sleep 1

cd "$DIR" || exit 1
rm -f /tmp/grimoire.alive
GRIMOIRE_AWESOME_STOPPED="$AWESOME_STOPPED" GRIMOIRE_ROOT="$ROOT" "$DIR/grimoire" >>"$LOG" 2>&1 &
QPID=$!
log "Grimoire running as pid $QPID"
# Watchdog: Grimoire touches /tmp/grimoire.alive every 2 s once its first frame is up. No first
# frame within 25 s, or a heartbeat older than 15 s, means it is stuck: kill it so the trap
# restores the Kindle UI instead of leaving the device paused.
# While the Kindle is suspended Grimoire cannot touch the heartbeat either, so after a gap in this
# loop (the device slept) give Grimoire a fresh 20 s before judging it.
START=$(date +%s)
LAST=$START
GRACE=0
while kill -0 "$QPID" 2>/dev/null; do
    sleep 2
    NOW=$(date +%s)
    if [ $((NOW - LAST)) -gt 8 ]; then
        log "watchdog: the device slept for $((NOW - LAST))s, waiting for a fresh heartbeat"
        GRACE=$((NOW + 20))
        START=$NOW
    fi
    LAST=$NOW
    [ "$NOW" -lt "$GRACE" ] && continue
    if [ -f /tmp/grimoire.alive ]; then
        AGE=$((NOW - $(date -r /tmp/grimoire.alive +%s 2>/dev/null || echo "$NOW")))
        if [ "$AGE" -gt 15 ]; then
            log "watchdog: no heartbeat for ${AGE}s, killing Grimoire"
            kill "$QPID" 2>/dev/null; sleep 2; kill -9 "$QPID" 2>/dev/null
        fi
    elif [ $((NOW - START)) -gt 25 ]; then
        log "watchdog: no first frame after 25s, killing Grimoire"
        kill "$QPID" 2>/dev/null; sleep 2; kill -9 "$QPID" 2>/dev/null
    fi
done
wait "$QPID" 2>/dev/null
log "Grimoire exited with status $?"
rm -f /tmp/grimoire.alive
exit 0
