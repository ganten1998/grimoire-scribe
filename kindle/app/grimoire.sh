#!/bin/sh
# Capabilities: guard hosted annotate book
# Grimoire launcher, started by the documents/Grimoire.sh scriptlet. Pauses the Kindle UI the way
# KOReader does on upstart firmware (5.x, incl. 5.16+ hard-float and 5.19), runs Grimoire, and
# ALWAYS restores everything on exit.
#
#   grimoire.sh                      pause the UI (pillow off, awesome + statusbar stopped)
#   grimoire.sh --stop-framework     stop lab126_gui entirely, restart it afterwards
#   grimoire.sh --hosted koreader    run inside KOReader's session, for its Grimoire plugin (below)
#   grimoire.sh --hosted koreader --annotate
#                                    the same, and open the page the plugin sent to annotate
#   grimoire.sh --hosted koreader --book
#                                    the same, and open the whole book the plugin sent
#   grimoire.sh --reset-calibration  forget the pen/touch calibration
#
# A flag file makes --stop-framework the default: touch /mnt/us/grimoire/stop-framework
# A Device Passcode stays enabled. Sleep releases input and resumes the native PIN screen.
# Inside KOReader it refuses to start (exit 3), as documents/Grimoire.sh does: --hosted is the one
# start allowed there. The Capabilities line above says what this file supports (guard: that
# refusal; hosted: --hosted koreader; annotate: its --annotate; book: its --book), for a tool that
# must not run it to find out; it always describes this very file, while app/CAPABILITIES can
# outlive a downgrade (see documents/Grimoire.sh).
#
# --annotate (after --hosted koreader only): KOReader's plugin wrote a page picture and a request
# into grimoire/import/.annotate/ (Annotate this page in Grimoire), and Grimoire is to open it: it
# gets GRIMOIRE_ANNOTATE=1. Every other start gets it empty, so a request found then (a hand-over
# that never reached Grimoire) is deleted unopened. --book, the same for a whole book (Open this book
# in Grimoire: every page, in grimoire/import/.book/): GRIMOIRE_BOOK=1.
#
# --hosted koreader: KOReader's Grimoire plugin (Tools > Grimoire) runs this from KOReader's exit
# path, once KOReader has let go of the screen and the input, and waits for it; then koreader.sh
# starts KOReader again in the same session. koreader.sh keeps the Kindle UI paused all along and
# puts it back, with the passcode, when KOReader quits, so this mode never touches the screen
# dump, pillow, the home booklet, volumd, lab126_gui or the passcode. It keeps the lock, the
# priority, the share port's firewall hole, the watchdog and the log, saves and puts back the
# power service's preventScreenSaver, and stops the status jobs while Grimoire runs (the KOReader
# that KPM installs leaves the 5.17+ status bar running). It stops awesome again, which KOReader's
# exit woke, keeps KOReader's LC_ALL from Grimoire, skips the settle wait and runs Grimoire in the
# foreground. Exit status: 0 Grimoire ran; 1 another Grimoire is running; 3 refused: not inside a
# KOReader that koreader.sh started and that paused the Kindle UI, or not from its exit path (its
# reader.lua still holds an input device).

DIR=$(cd "$(dirname "$0")" && pwd)
# Device paths. tests/launcher_check.sh moves them into a scratch tree; on the Kindle they are
# never set.
US=${GRIMOIRE_US:-/mnt/us}
TMP=${GRIMOIRE_TMP:-/tmp}
SYS=${GRIMOIRE_SYSROOT:-}
ROOT=$US/grimoire
LOG=$ROOT/grimoire.log
FB=$SYS/dev/fb0
DUMP=$SYS/var/tmp/grimoire-fb.dump
BEAT=$TMP/grimoire.alive  # the binary's heartbeat (it writes /tmp/grimoire.alive)
mkdir -p "$ROOT"
log() { echo "$(date '+%F %T') grimoire.sh: $*" >>"$LOG"; }

if [ "$1" = "--reset-calibration" ]; then
    p=$(cat "$TMP/grimoire.launcher.pid" 2>/dev/null)
    case "$p" in
    '' | *[!0-9]*) ;;
    *) if kill -0 "$p" 2>/dev/null; then
        echo 'Grimoire: close Grimoire before resetting calibration.' >&2
        exit 2
    fi ;;
    esac
    # A copy and a rename rather than sed -i, which busybox and BSD sed read differently.
    S=$ROOT/settings.ini
    [ -f "$S" ] || exit 0
    if sed -e '/^pen\./d' -e '/^touch\./d' "$S" >"$S.reset" && mv -f "$S.reset" "$S"; then
        log "pen and touch calibration reset"
        exit 0
    fi
    rm -f "$S.reset"
    log "could not reset the calibration in $S"
    exit 1
fi

# A process's name and parent from /proc/<pid>/stat ("pid (name) state ppid ..."): sets P_NAME
# and P_PARENT, fails when there is no such process.
proc_parent() {
    P_LINE=""
    { read -r P_LINE <"$SYS/proc/$1/stat"; } 2>/dev/null
    [ -n "$P_LINE" ] || return 1
    P_NAME=${P_LINE#*"("}
    P_NAME=${P_NAME%")"*}
    P_PARENT=${P_LINE##*") "}
    P_PARENT=${P_PARENT#* }
    P_PARENT=${P_PARENT%% *}
}
# For --hosted: KOReader's reader.lua is among this launcher's parents (its os.execute starts us,
# through a sh -c), and koreader.sh is reader.lua's own parent. A /var/tmp/koreader.sh that a
# crashed session left behind proves neither. Sets KO_READER, or KO_WHY when it fails.
ko_parents() {
    p=$PPID
    n=0
    while [ "$n" -lt 8 ] && proc_parent "$p"; do
        if [ "$P_NAME" = reader.lua ] || tr '\000' ' ' <"$SYS/proc/$p/cmdline" 2>/dev/null | grep -q 'reader\.lua'; then
            KO_READER=$p
            ancestor=$P_PARENT
            depth=0
            while [ "$depth" -lt 8 ]; do
                tr '\000' ' ' <"$SYS/proc/$ancestor/cmdline" 2>/dev/null | grep -q 'koreader\.sh' && return 0
                proc_parent "$ancestor" || break
                [ "$P_PARENT" -gt 1 ] 2>/dev/null || break
                ancestor=$P_PARENT
                depth=$((depth + 1))
            done
            KO_WHY="KOReader's reader.lua (pid $p) was not started by koreader.sh"
            return 1
        fi
        [ "$P_PARENT" -gt 1 ] 2>/dev/null || break
        p=$P_PARENT
        n=$((n + 1))
    done
    KO_WHY="KOReader's reader.lua is not among this launcher's parents"
    return 1
}
# For --hosted: KOReader's exit has let go of pen/touch input. A retained key-only device (such
# as kindle-input-pt on newer setups) cannot hold either digitizer. Permit it only when sysfs
# positively reports no absolute axes; unknown/malformed capabilities remain a refusal. Check
# every open device, so a keyboard never hides a retained pen or touch screen later in the list.
# Sets KO_HELD to the first blocking device; an unreadable fd list keeps the previous behavior.
ko_let_go() {
    KO_HELD=""
    for KO_INPUT in $(ls -l "$SYS/proc/$KO_READER/fd" 2>/dev/null | awk '$NF ~ /^\/dev\/input\// { print $NF }'); do
        KO_ABS=$(cat "$SYS/sys/class/input/${KO_INPUT##*/}/device/capabilities/abs" 2>/dev/null)
        case "$KO_ABS" in
        '' | *[!0\ ]*) ;;  # Missing, nonzero or malformed: still potentially pen/touch.
        *0*) continue ;;
        esac
        KO_HELD=$KO_INPUT
        return 1
    done
    return 0
}

HOSTED=""
KO_OWN_UI=no
ANNOTATE=""
BOOK=""
if [ "$1" = "--hosted" ]; then
    # KOReader's own variables (koreader.sh exports them), read before this script reuses the
    # names STOP_FRAMEWORK and AWESOME_STOPPED for its own pause below.
    KO_STOP_FRAMEWORK=$STOP_FRAMEWORK
    KO_AWESOME_STOPPED=$AWESOME_STOPPED
    MSG=""
    if [ "$2" != koreader ]; then
        MSG="not starting: --hosted ${2:-without a name} is no host Grimoire knows (the one host is koreader)."
    elif [ -z "$KOREADER_DIR" ]; then
        MSG="not starting: --hosted koreader runs only inside KOReader, from its Grimoire plugin."
    elif ! ko_parents; then
        MSG="not starting hosted: $KO_WHY."
    elif ! ko_let_go; then
        MSG="not starting hosted: KOReader still has $KO_HELD open (this is not its exit path), so Grimoire would get no pen or touch."

    fi
    if [ -n "$MSG" ]; then
        log "$MSG"
        echo "Grimoire: $MSG" >&2
        exit 3
    fi
    HOSTED=koreader
    if [ "$KO_STOP_FRAMEWORK" != yes ] && [ "$PILLOW_HARD_DISABLED" != yes ] && [ "$KO_AWESOME_STOPPED" != yes ]; then
        KO_OWN_UI=yes
    fi
    [ "$3" = --annotate ] && ANNOTATE=1
    [ "$3" = --book ] && BOOK=1
    # First of all: KOReader's exit has just woken awesome on Special Offers Kindles (a KOReader
    # quitting for good wants the Kindle UI back), so stop it again before the lock wait and the
    # lipc calls below give it time to draw. koreader.sh continues it when KOReader quits; this
    # launcher never does. (Logged once the session's log lines start.)
    AWESOME_RESTOPPED=no
    if [ "$KO_AWESOME_STOPPED" = yes ] && killall -STOP awesome 2>/dev/null; then AWESOME_RESTOPPED=yes; fi
    # koreader.sh exports LC_ALL=en_US.UTF-8 for KOReader itself, whatever the Kindle's language.
    # Grimoire gets the locale a start from the Library gets: LANG, which koreader.sh leaves as it
    # found it, carries on, and KOReader's LC_ALL goes.
    unset LC_ALL
else
    # KOReader keeps the pen and the touch screen to itself while it runs: Grimoire would get
    # neither, and the restore below would wake the Kindle UI underneath KOReader. See
    # documents/Grimoire.sh.
    MSG=""
    if [ -n "$KOREADER_DIR" ]; then
        MSG="not starting inside KOReader, which holds the pen and the touch screen: quit KOReader, then tap Grimoire in the Library."
    elif KO_PID=$(pidof reader.lua 2>/dev/null); then
        MSG="not starting: KOReader is still running (reader.lua, pid $KO_PID) and holds the pen and the touch screen. Quit it, or restart the Kindle if it is not open, then tap Grimoire in the Library."
    fi
    if [ -n "$MSG" ]; then
        log "$MSG"
        echo "Grimoire: $MSG" >&2
        if [ -z "$KOREADER_DIR" ] && command -v eips >/dev/null 2>&1; then
            (usleep 2000000 2>/dev/null || sleep 2; eips 1 3 "Grimoire did not start: KOReader is running.") \
                </dev/null >/dev/null 2>&1 &
        fi
        exit 3
    fi
fi

# The share server's port (Settings may change it; the firewall hole must match).
PORT=$(sed -n 's/^http_port=\([0-9][0-9]*\)$/\1/p' "$ROOT/settings.ini" 2>/dev/null | tail -n 1)
[ -n "$PORT" ] || PORT=8765

# Only one Grimoire at a time. The lock is this launcher's pid, not `pidof grimoire`: a background PDF
# export can outlive Grimoire (and shares its name), and a launcher that is still restoring the
# Kindle UI after Grimoire exited must finish first, or its restore lands on top of the new Grimoire.
LOCK=$TMP/grimoire.launcher.pid
lock_holder() {
    p=$(cat "$LOCK" 2>/dev/null)
    [ -n "$p" ] && [ "$p" != "$$" ] && kill -0 "$p" 2>/dev/null && grep -q grimoire.sh "$SYS/proc/$p/cmdline" 2>/dev/null && echo "$p"
}
TRIES=0
while HOLDER=$(lock_holder) && [ -n "$HOLDER" ]; do
    TRIES=$((TRIES + 1))
    if [ "$TRIES" -gt 20 ]; then
        log "already running (launcher pid $HOLDER), not starting another"
        # KOReader's plugin tells its user that Grimoire did not start.
        [ -n "$HOSTED" ] && exit 1
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
if [ -f "$ROOT/stop-framework" ]; then
    if [ -n "$HOSTED" ]; then
        log "the stop-framework flag does not apply inside KOReader, which paused the Kindle UI itself"
    else
        STOP_FRAMEWORK=yes
    fi
fi

AWESOME_STOPPED=no
PILLOW_DISABLED=no
VOLUMD_STOPPED=no
IPTABLES_ADDED=no
STOPPED_JOBS=""
RESTORED=no
PREVENT_BEFORE=0  # preventScreenSaver to put back: off, or hosted, what KOReader had
# Native PIN flag (never modified here) and legacy beta.1 recovery markers.
PASSCODE=$SYS/var/local/system/userpasswdenabled
PASSCODE_MARK=$ROOT/.passcode-paused
PASSCODE_LEFT=$ROOT/.passcode-left-off  # how many starts have told the user it was left off
PASSCODE_NOTE=""

# The Kindle is locked: its screensaver (or the passcode prompt over it) is up, or it is about
# to suspend. Sets STATE to what the power service says.
kindle_locked() {
    STATE=$(lipc-get-prop -s com.lab126.powerd state 2>/dev/null)
    case "$STATE" in
    screenSaver | readyToSuspend | suspended) return 0 ;;
    esac
    return 1
}


restore() {
    [ "$RESTORED" = yes ] && return
    RESTORED=yes
    if [ -n "$HOSTED" ]; then
        log "handing the screen back to KOReader"
    else
        log "restoring the Kindle UI"
    fi
    if [ "$IPTABLES_ADDED" = yes ]; then
        iptables -D INPUT -i wlan0 -p tcp --dport "$PORT" -j ACCEPT 2>/dev/null
    fi
    if [ "$VOLUMD_STOPPED" = yes ]; then
        killall -CONT volumd 2>/dev/null
    fi
    lipc-set-prop com.lab126.powerd preventScreenSaver "$PREVENT_BEFORE" 2>/dev/null
    # Grimoire's power-service watcher, should a crash have left it behind.
    pkill -f "lipc-wait-event .*com.lab126.powerd goingToScreenSaver,outOfScreenSaver" 2>/dev/null
    # The Bluetooth helper, if Grimoire started it (Settings > Controller): stopped, and the Kindle's
    # own Bluetooth jobs started again as they were. One that someone else started is left running.
    if [ -f "$DIR/bluetooth.sh" ]; then
        GRIMOIRE_ROOT="$ROOT" sh "$DIR/bluetooth.sh" stop-owned >/dev/null 2>>"$LOG"
    fi
    if [ -n "$HOSTED" ] && [ "$KO_OWN_UI" = no ]; then
        # KOReader comes back over the screen, and puts the Kindle UI back when it quits: only the
        # status jobs stopped here are this launcher's to start.
        for job in $STOPPED_JOBS; do
            start "$job" 2>/dev/null
        done
        if [ "$PASSCODE_NOTE" = enabled ]; then
            [ "$PILLOW_HARD_DISABLED" = yes ] && lipc-set-prop com.lab126.pillow disableEnablePillow disable 2>/dev/null
            [ "$KO_AWESOME_STOPPED" = yes ] && killall -STOP awesome 2>/dev/null
        fi
        kindle_locked
        log "power state at exit: ${STATE:-unknown}"
        return
    fi
    if [ "$STOP_FRAMEWORK" = yes ]; then
        kindle_locked
        log "power state at exit: ${STATE:-unknown}"

        cd / && start lab126_gui 2>/dev/null
        return
    fi
    for job in $STOPPED_JOBS; do
        start "$job" 2>/dev/null
    done
    if [ "$AWESOME_STOPPED" = yes ]; then
        killall -CONT awesome 2>/dev/null
    fi
    # Locked: the screen saved at the start must not go back over the Kindle's screensaver or
    # its passcode prompt (the Kindle draws its own screen once it is unlocked).
    if kindle_locked || { [ "$PASSCODE_NOTE" = enabled ] && [ "$STATE" != active ]; }; then
        log "power state at exit: $STATE, so the saved screen is not put back"
        rm -f "$DUMP"
    else
        log "power state at exit: ${STATE:-unknown}"
    fi
    if [ "$PILLOW_DISABLED" = yes ]; then
        # Put back what was on screen before us, then let the home booklet repaint.
        if [ -f "$DUMP" ]; then
            cat "$DUMP" >"$FB" 2>/dev/null
            rm -f "$DUMP"
        fi
        lipc-set-prop com.lab126.pillow disableEnablePillow enable 2>/dev/null
        [ -z "$HOSTED" ] && lipc-set-prop com.lab126.appmgrd start app://com.lab126.booklet.home 2>/dev/null
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

if [ -n "$HOSTED" ]; then
    log "starting Grimoire hosted by KOReader (reader.lua pid $KO_READER; KOReader stopped the framework: ${KO_STOP_FRAMEWORK:-no}, awesome: ${KO_AWESOME_STOPPED:-no}) on $(cat "$SYS/etc/prettyversion.txt" 2>/dev/null)"
    # KOReader drops its stand-in for the framework (which the power service wants before it
    # takes the power button) at its exit.
    [ "$KO_STOP_FRAMEWORK" = yes ] && log "KOReader stopped the Kindle framework: the power button may not put the Kindle to sleep in Grimoire"
    [ -n "$ANNOTATE" ] && log "KOReader sent a page to annotate (grimoire/import/.annotate)"
    [ -n "$BOOK" ] && log "KOReader sent a whole book (grimoire/import/.book)"
else
    log "starting Grimoire (stop framework: $STOP_FRAMEWORK) on $(cat "$SYS/etc/prettyversion.txt" 2>/dev/null)"
fi
# What tells setups apart in a bug report. grimoire.log gets attached to public issues, so nothing
# here may identify the device or its network (no serial number, address or network name).
LANG_LINE=$(grep '^LANG=' "$SYS/var/local/system/locale" 2>/dev/null | head -n 1)
log "locale: ${LANG_LINE:-unknown}"
if [ -e "$SYS/var/local/system/userpasswdenabled" ]; then
    log "device passcode: on"
else
    log "device passcode: off"
fi
STATE=$(lipc-get-prop -s com.lab126.powerd state 2>/dev/null)
log "power state: ${STATE:-unknown}"
BLANKET=$(lipc-get-prop -s com.lab126.blanket load 2>/dev/null)
case "$BLANKET" in
*ad_screensaver*) log "blanket modules: $BLANKET (Special Offers)" ;;
*) log "blanket modules: ${BLANKET:-unknown}" ;;
esac
if [ -f "$US/koreader/reader.lua" ]; then
    KO_REV=$(head -n 1 "$US/koreader/git-rev" 2>/dev/null)
    log "KOReader: installed, version ${KO_REV:-unknown}"
else
    log "KOReader: not installed"
fi
log "user id: $(id -u 2>/dev/null)"

# Keep the native PIN enabled. The binary yields input and the UI on sleep and only takes them
# back after outOfScreenSaver (which Kindle sends after a successful PIN). Legacy markers from
# beta.1 are still consumed, with the old recovery notice if that beta left the PIN disabled.
# KOReader itself may already have disabled the PIN; never recreate a flag without proof of a PIN.
if [ -n "$HOSTED" ]; then
    log "device passcode: left to KOReader, which pauses it for its session"
    [ -e "$PASSCODE" ] && PASSCODE_NOTE=enabled
else
    if [ -e "$PASSCODE_MARK" ] && [ ! -e "$PASSCODE" ]; then
        log "device passcode: left off by a session that did not end (a restart?); not turned back on here"
        echo 0 2>/dev/null >"$PASSCODE_LEFT"
        PASSCODE_NOTE=left-off
    fi
    rm -f "$PASSCODE_MARK" 2>/dev/null
    if [ -e "$PASSCODE_LEFT" ]; then
        if [ -e "$PASSCODE" ]; then
            rm -f "$PASSCODE_LEFT"  # back on: nothing more to tell
        else
            TOLD=$(cat "$PASSCODE_LEFT" 2>/dev/null)
            case "$TOLD" in '' | *[!0-9]*) TOLD=0 ;; esac
            TOLD=$((TOLD + 1))
            if [ "$TOLD" -ge 3 ]; then rm -f "$PASSCODE_LEFT"; else echo "$TOLD" 2>/dev/null >"$PASSCODE_LEFT"; fi
            PASSCODE_NOTE=left-off
            log "device passcode: still off since a session that did not end; telling the user ($TOLD of 3)"
        fi
    fi
    if [ -e "$PASSCODE" ]; then
        PASSCODE_NOTE=enabled
        log "device passcode: enabled throughout the session; native unlock on wake"
        if [ "$STOP_FRAMEWORK" = yes ]; then
            STOP_FRAMEWORK=no
            log "device passcode: keeping the framework running for its PIN keypad"
        fi
        if ! kindle_locked && [ "$STATE" = active ]; then
            :
        else
            log "not starting: unlock the Kindle first (power state: ${STATE:-unknown})"
            exit 3
        fi
    fi
fi

# Launchers can start us niced; go back to normal priority for pen latency.
[ "$(nice)" -gt 0 ] 2>/dev/null && renice -n "-$(nice)" $$ >/dev/null 2>&1

# On 5.17+ the status bar (clock, battery) is its own upstart job and draws over everything. Its
# name varies by device/firmware, so stop every job that looks like one; restore() starts again
# exactly those that were running.
stop_status_jobs() {
    for conf in "$SYS"/etc/upstart/*status*.conf "$SYS"/etc/upstart/*chrome*.conf; do
        [ -f "$conf" ] || continue
        job=$(basename "$conf" .conf)
        if stop "$job" >/dev/null 2>&1; then
            STOPPED_JOBS="$STOPPED_JOBS $job"
            log "stopped job $job"
        fi
    done
}

if [ -n "$HOSTED" ] && [ "$KO_OWN_UI" = no ]; then
    # KOReader paused the Kindle UI; awesome, which its exit woke, was stopped again first thing.
    [ "$AWESOME_RESTOPPED" = yes ] && log "awesome stopped again, as KOReader had it"
    stop_status_jobs
    log "upstart jobs running: $(initctl list 2>/dev/null | grep -i 'start/running' | cut -d' ' -f1 | tr '\n' ' ')"
elif [ "$STOP_FRAMEWORK" = yes ]; then
    # The framework job sends SIGTERM on stop; do not let it take us down with it.
    trap "" TERM
    stop lab126_gui 2>/dev/null
    usleep 1250000 2>/dev/null || sleep 2
    trap 'stop_grimoire; restore; exit 1' INT TERM HUP
else
    cat "$FB" >"$DUMP" 2>/dev/null
    lipc-set-prop com.lab126.pillow disableEnablePillow disable 2>/dev/null && PILLOW_DISABLED=yes
    # Pillow off is not enough to stop the clock / window manager repainting on 5.7.2+.
    if killall -STOP awesome 2>/dev/null; then AWESOME_STOPPED=yes; fi
    stop_status_jobs
    # For diagnosis: what is still running that could draw (logged once per launch).
    log "pillow disabled: $PILLOW_DISABLED, awesome stopped: $AWESOME_STOPPED"
    log "upstart jobs running: $(initctl list 2>/dev/null | grep -i 'start/running' | cut -d' ' -f1 | tr '\n' ' ')"
fi

# Keep the Kindle awake while Grimoire starts. Grimoire turns this off again unless Sleep after
# is set to never (the power service ignores the button while it is on); then a press of the
# power button sleeps on the page, and holding it leaves Grimoire. Hosted, KOReader's own value
# goes back at the end (its Keep alive, say), not off.
if [ -n "$HOSTED" ]; then
    PREVENT_BEFORE=$(lipc-get-prop -i com.lab126.powerd preventScreenSaver 2>/dev/null)
    case "$PREVENT_BEFORE" in 0 | 1) ;; *) PREVENT_BEFORE=0 ;; esac
fi
lipc-set-prop com.lab126.powerd preventScreenSaver 1 2>/dev/null
# Keep USB mass storage from yanking /mnt/us out from under us (hosted, KOReader has volumd
# stopped already).
if [ -z "$HOSTED" ] && [ -e "$SYS/etc/upstart/volumd.conf" ] && killall -STOP volumd 2>/dev/null; then
    VOLUMD_STOPPED=yes
fi
# Let the Wi-Fi share screen be reached (the stock firewall drops incoming connections).
if iptables -A INPUT -i wlan0 -p tcp --dport "$PORT" -j ACCEPT 2>/dev/null; then IPTABLES_ADDED=yes; fi

# Give the Kindle UI a moment to finish repainting after the scriptlet returned, so its last
# frame lands before ours (Grimoire also re-checks the panel and repaints if it was drawn over).
# Hosted, nothing repaints: KOReader has closed its screen.
if [ -z "$HOSTED" ]; then
    usleep 900000 2>/dev/null || sleep 1
fi

cd "$DIR" || exit 1
rm -f "$BEAT"
# Hosted, Grimoire learns its host (its Exit goes back to KOReader), and AWESOME_STOPPED stayed
# no above: it never wakes awesome, which KOReader's session keeps stopped. GRIMOIRE_ANNOTATE and
# GRIMOIRE_BOOK: a page or a book KOReader sent, set for that hosted start alone (empty otherwise,
# whatever this launcher inherited).
APP_AWESOME=$AWESOME_STOPPED
APP_PILLOW=$PILLOW_DISABLED
if [ -n "$HOSTED" ] && [ "$KO_OWN_UI" = no ] && [ "$PASSCODE_NOTE" = enabled ]; then
    APP_AWESOME=$KO_AWESOME_STOPPED
    APP_PILLOW=$PILLOW_HARD_DISABLED
fi
GRIMOIRE_AWESOME_STOPPED="$APP_AWESOME" GRIMOIRE_PILLOW_DISABLED="$APP_PILLOW" GRIMOIRE_PASSCODE="$PASSCODE_NOTE" GRIMOIRE_HOST="$HOSTED" \
    GRIMOIRE_ANNOTATE="$ANNOTATE" GRIMOIRE_BOOK="$BOOK" GRIMOIRE_ROOT="$ROOT" GRIMOIRE_BT_SCRIPT="$DIR/bluetooth.sh" \
    "$DIR/grimoire" >>"$LOG" 2>&1 &
QPID=$!
log "Grimoire running as pid $QPID"
# Watchdog: Grimoire touches its heartbeat file every 2 s once its first frame is up. No first
# frame within 25 s, or a heartbeat older than 15 s, means it is stuck: kill it so the trap
# restores the Kindle UI instead of leaving the device paused.
# While the Kindle is suspended Grimoire cannot touch the heartbeat either, so after a gap in this
# loop (the device slept) give Grimoire a fresh 20 s before judging it.
# The GRIMOIRE_WD_* variables only exist so tests/launcher_check.sh can run this in seconds.
WD_TICK=${GRIMOIRE_WD_TICK:-2}
WD_FIRST=${GRIMOIRE_WD_FIRST:-25}
WD_STALE=${GRIMOIRE_WD_STALE:-15}
START=$(date +%s)
LAST=$START
GRACE=0
while kill -0 "$QPID" 2>/dev/null; do
    sleep "$WD_TICK"
    NOW=$(date +%s)
    if [ $((NOW - LAST)) -gt 8 ]; then
        log "watchdog: the device slept for $((NOW - LAST))s, waiting for a fresh heartbeat"
        GRACE=$((NOW + 20))
        START=$NOW
    fi
    LAST=$NOW
    [ "$NOW" -lt "$GRACE" ] && continue
    if [ -f "$BEAT" ]; then
        AGE=$((NOW - $(date -r "$BEAT" +%s 2>/dev/null || echo "$NOW")))
        if [ "$AGE" -gt "$WD_STALE" ]; then
            log "watchdog: no heartbeat for ${AGE}s, killing Grimoire"
            kill "$QPID" 2>/dev/null; sleep 2; kill -9 "$QPID" 2>/dev/null
        fi
    elif [ $((NOW - START)) -gt "$WD_FIRST" ]; then
        log "watchdog: no first frame after ${WD_FIRST}s, killing Grimoire"
        kill "$QPID" 2>/dev/null; sleep 2; kill -9 "$QPID" 2>/dev/null
    fi
done
wait "$QPID" 2>/dev/null
RC=$?
log "Grimoire exited with status $RC"
rm -f "$BEAT"
exit "$RC"
