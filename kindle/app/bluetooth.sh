#!/bin/sh
# bluetooth.sh: the Bluetooth helper (kindle-hid-passthrough, docs/SHORTCUTS.md) as Grimoire runs it.
#   start       start the helper for Grimoire, unless one runs already (then it is someone else's)
#   stop-owned  stop the helper this script started, and give the Kindle its own Bluetooth back
#   status      owned, external, stopped or missing
# The helper is the one unpacked in grimoire/bluetooth/ (only its program and its own data: never
# its installer, whose boot-time button mapper grabs controllers away from Grimoire), else an
# existing /mnt/us/kindle_hid_passthrough. Nothing here installs anything, writes outside /tmp and
# the helper's own folder, or turns on its start at boot. State lives in /tmp/grimoire-bt.
# GRIMOIRE_ROOT: the data folder (default /mnt/us/grimoire). BT_STATE, BT_API, BT_JOBS: tests only.
set -u
ROOT=${GRIMOIRE_ROOT:-/mnt/us/grimoire}
STATE=${BT_STATE:-/tmp/grimoire-bt}
API=${BT_API:-127.0.0.1:8321}
JOBS=${BT_JOBS:-btmanagerd acsbtfd}

say() { echo "bluetooth.sh: $*" >&2; }

# One run at a time (Grimoire's start and the launcher's stop can meet): a lock directory, given
# up after 20 s by a run that finds it stale.
lock() {
    i=0
    until mkdir "$STATE.lock" 2>/dev/null; do
        i=$((i + 1))
        if [ $i -ge 100 ]; then
            say "a stale lock: taking it"
            rm -rf "$STATE.lock"
        fi
        sleep 0.2
    done
    trap 'rmdir "$STATE.lock" 2>/dev/null' EXIT
}

helper_dir() {
    for d in "$ROOT/bluetooth" /mnt/us/kindle_hid_passthrough; do
        if [ -x "$d/kindle-hid-passthrough" ]; then
            echo "$d"
            return 0
        fi
    done
    return 1
}

api_up() {
    if command -v curl >/dev/null 2>&1; then
        curl -s -m 2 "http://$API/health" 2>/dev/null | grep -q '"ok": *true'
    else
        wget -q -T 2 -O - "http://$API/health" 2>/dev/null | grep -q '"ok": *true'
    fi
}

# The recorded pid, if it is still the helper this script started (its command line names its folder).
owned_pid() {
    [ -f "$STATE/pid" ] && [ -f "$STATE/dir" ] || return 1
    pid=$(cat "$STATE/pid")
    dir=$(cat "$STATE/dir")
    case $pid in ''|*[!0-9]*) return 1 ;; esac
    kill -0 "$pid" 2>/dev/null || return 1
    tr '\0' ' ' < "/proc/$pid/cmdline" 2>/dev/null | grep -qF "$dir" || [ -n "${BT_NO_PROC:-}" ] || return 1
    echo "$pid"
}

job_running() { status "$1" 2>/dev/null | grep -q 'start/running'; }

# The helper in Grimoire's folder keeps its paired devices and keys there too (its config.ini as
# shipped names /mnt/us/kindle_hid_passthrough), and plays no media remote (a phone could pair with
# the Kindle as a speaker). Its other settings stay as they are.
fix_config() {
    cfg=$1/config.ini
    if [ ! -f "$cfg" ]; then
        printf '[paths]\ncache_dir = %s/cache\ndevices_config = %s/devices.conf\n\n[media_remote]\nenabled = false\n' \
            "$1" "$1" > "$cfg"
        return
    fi
    awk -v d="$1" '
        /^[ \t]*\[/ { sec = $0; sub(/^[ \t]*/, "", sec); sub(/[ \t]*$/, "", sec) }
        sec == "[paths]" && /^[ \t]*cache_dir[ \t]*=/ { print "cache_dir = " d "/cache"; next }
        sec == "[paths]" && /^[ \t]*devices_config[ \t]*=/ { print "devices_config = " d "/devices.conf"; next }
        sec == "[media_remote]" && /^[ \t]*enabled[ \t]*=/ { print "enabled = false"; off = 1; next }
        { print }
        END { if (!off) printf "\n[media_remote]\nenabled = false\n" }' "$cfg" > "$cfg.tmp" && mv "$cfg.tmp" "$cfg"
}

bt_start() {
    mkdir -p "$STATE"
    if owned_pid >/dev/null; then
        echo owned
        return 0
    fi
    if api_up || pgrep -f 'kindle-hid-passthrough|kindle_hid_passthrough' >/dev/null 2>&1; then
        echo external
        return 0
    fi
    dir=$(helper_dir) || { echo missing; return 1; }
    # The Kindle's own Bluetooth jobs that run now: the helper stops them, stop-owned starts them again.
    : > "$STATE/jobs"
    for job in $JOBS; do
        if job_running "$job"; then echo "$job" >> "$STATE/jobs"; fi
    done
    if [ "$dir" = "$ROOT/bluetooth" ]; then fix_config "$dir"; fi
    echo "$dir" > "$STATE/dir"
    if command -v setsid >/dev/null 2>&1; then
        setsid "$dir/kindle-hid-passthrough" --daemon >> "$STATE/helper.log" 2>&1 < /dev/null &
    else
        "$dir/kindle-hid-passthrough" --daemon >> "$STATE/helper.log" 2>&1 < /dev/null &
    fi
    echo $! > "$STATE/pid"
    i=0
    while [ $i -lt 50 ]; do
        if api_up; then
            say "the helper answers (from $dir)"
            echo owned
            return 0
        fi
        if ! kill -0 "$(cat "$STATE/pid")" 2>/dev/null; then
            say "the helper stopped as it started (see $STATE/helper.log)"
            restore_jobs
            echo failed
            return 1
        fi
        i=$((i + 1))
        sleep 0.2
    done
    say "the helper started but does not answer yet"
    echo owned
}

restore_jobs() {
    [ -f "$STATE/jobs" ] || return 0
    while read -r job; do
        [ -n "$job" ] && ! job_running "$job" && start_job "$job"
    done < "$STATE/jobs"
    rm -f "$STATE/jobs"
}

start_job() { start "$1" >/dev/null 2>&1 || initctl start "$1" >/dev/null 2>&1 || true; }

bt_stop_owned() {
    if pid=$(owned_pid); then
        kill -TERM "$pid" 2>/dev/null
        # The helper detaches its devices on the way out (it gives that 10 s): wait 12, then insist.
        i=0
        while kill -0 "$pid" 2>/dev/null && [ $i -lt 60 ]; do
            i=$((i + 1))
            sleep 0.2
        done
        if kill -0 "$pid" 2>/dev/null; then
            say "the helper did not stop: killing it"
            kill -KILL "$pid" 2>/dev/null
        fi
    fi
    rm -f "$STATE/pid" "$STATE/dir"
    restore_jobs
    echo stopped
}

status_of() {
    if owned_pid >/dev/null; then echo owned
    elif api_up; then echo external
    elif helper_dir >/dev/null; then echo stopped
    else echo missing
    fi
}

case "${1:-}" in
start) lock; bt_start ;;
stop-owned) lock; bt_stop_owned ;;
status) status_of ;;
*) echo "usage: bluetooth.sh start|stop-owned|status" >&2; exit 2 ;;
esac
