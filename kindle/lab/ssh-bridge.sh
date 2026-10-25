#!/bin/sh
# Explicit developer SSH bridge. Never installed by the Grimoire app or KPM hooks.
set -eu
umask 077
STATE=${KINDLELAB_SSH_STATE:-/var/local/kindle-lab-ssh}
PROC=${KINDLELAB_PROC:-/proc}
IPTABLES=${KINDLELAB_IPTABLES:-iptables}
PORT=2223
TAG=kindlelab-ssh
PID=$STATE/listener.pid
BIN=$STATE/dropbear-lab
OWNER=kindlelab-ssh-v1
fail() { echo "KindleLab SSH: $*" >&2; exit 1; }
managed() { [ "$(cat "$STATE/owner" 2>/dev/null)" = "$OWNER" ]; }
live_pid() {
    p=$(cat "$PID" 2>/dev/null || true)
    case "$p" in ''|*[!0-9]*|0|1) return 1;; esac
    kill -0 "$p" 2>/dev/null
}
owned_pid() { live_pid && [ "$(readlink "$PROC/$p/exe" 2>/dev/null)" = "$BIN" ]; }
fw_input() { "$IPTABLES" "$1" INPUT ${2:+$2} -p tcp --dport "$PORT" -m conntrack --ctstate NEW,ESTABLISHED -m comment --comment "$TAG" -j ACCEPT; }
fw_output() { "$IPTABLES" "$1" OUTPUT ${2:+$2} -p tcp --sport "$PORT" -m conntrack --ctstate ESTABLISHED -m comment --comment "$TAG" -j ACCEPT; }
fw_remove() {
    if fw_input -C '' 2>/dev/null; then fw_input -D ''; fi
    if fw_output -C '' 2>/dev/null; then fw_output -D ''; fi
}
stop_listener() {
    if live_pid; then
        owned_pid || fail 'PID belongs to another process; leaving it alone.'
        kill -TERM "$p"
        tries=0
        while kill -0 "$p" 2>/dev/null; do
            tries=$((tries + 1))
            [ "$tries" -lt 5 ] || fail 'Listener did not stop; no other process was killed.'
            sleep 1
        done
    fi
    rm -f "$PID"
}
install() {
    source=${2:-}
    [ -n "$source" ] && [ -f "$source/reader.lua" ] || fail 'Specify an installed KOReader directory.'
    [ -x "$source/dropbear" ] || fail 'KOReader has no Dropbear server.'
    [ -s "$source/settings/SSH/authorized_keys" ] || fail 'Configure a public SSH key in KOReader first.'
    [ -s "$source/settings/SSH/dropbear_ed25519_host_key" ] || fail 'Start KOReader SSH once to create its host identity.'
    if [ -e "$STATE" ] && ! managed; then fail 'State directory is not owned by this bridge.'; fi
    mkdir -p "$STATE"
    chmod 700 "$STATE"
    printf '%s\n' "$OWNER" > "$STATE/owner"
    live_pid && fail 'Stop this bridge before refreshing it.'
    help=$("$source/dropbear" -h 2>&1 || true)
    printf '%s\n' "$help" | grep -q -- '-D' || fail 'Dropbear does not support a separate authorized-keys directory.'
    cp "$source/dropbear" "$BIN.new"
    cp "$source/settings/SSH/authorized_keys" "$STATE/authorized_keys.new"
    cp "$source/settings/SSH/dropbear_ed25519_host_key" "$STATE/hostkey.new"
    chmod 700 "$BIN.new"
    chmod 600 "$STATE/authorized_keys.new" "$STATE/hostkey.new"
    mv -f "$BIN.new" "$BIN"
    mv -f "$STATE/authorized_keys.new" "$STATE/authorized_keys"
    mv -f "$STATE/hostkey.new" "$STATE/hostkey"
    echo 'KindleLab SSH prepared. Key authentication only; start it explicitly.'
}
start() {
    managed || fail 'Bridge is not installed.'
    if live_pid; then
        owned_pid || fail 'PID belongs to another process; leaving it alone.'
        echo "KindleLab SSH is already running on port $PORT (key authentication only)."
        return
    fi
    [ -x "$BIN" ] && [ -s "$STATE/authorized_keys" ] && [ -s "$STATE/hostkey" ] || fail 'Bridge installation is incomplete.'
    rm -f "$PID"
    # The distinct executable name and PID keep KOReader's own SSH cleanup separate.
    "$BIN" -E -m -s -j -k -T 3 -K 30 -I 600 -p "0.0.0.0:$PORT" -P "$PID" \
        -D "$STATE" -r "$STATE/hostkey" </dev/null >>"$STATE/server.log" 2>&1 || fail 'Could not start the listener; see server.log.'
    tries=0
    until owned_pid; do
        tries=$((tries + 1))
        [ "$tries" -lt 4 ] || fail 'Listener failed to start; see server.log.'
        sleep 1
    done
    if ! { { fw_input -C '' 2>/dev/null || fw_input -I 1; } && { fw_output -C '' 2>/dev/null || fw_output -I 1; }; }; then
        stop_listener
        fw_remove || true
        fail 'Could not add the bridge firewall rules; listener was stopped.'
    fi
    echo "KindleLab SSH started on port $PORT (key authentication only)."
    echo 'Available across app exits while networking is awake. Stops at reboot; no boot service added.'
}
stop() {
    managed || fail 'Bridge is not installed.'
    stop_listener
    fw_remove
    echo 'KindleLab SSH listener stopped. Existing authenticated sessions may finish.'
}
status() {
    if ! managed; then echo 'KindleLab SSH is not installed.'; return 1; fi
    if owned_pid; then echo "KindleLab SSH running on port $PORT (key authentication only)."; return; fi
    live_pid && fail 'PID belongs to another process; leaving it alone.'
    echo 'KindleLab SSH stopped.'
    return 1
}
# Serialize lifecycle commands, including simultaneous Library taps. Status is read-only.
case "${1:-status}" in
status) status; exit $? ;;
install) if [ ! -e "$STATE" ]; then mkdir -p "$STATE"; chmod 700 "$STATE"; printf '%s\n' "$OWNER" > "$STATE/owner"; fi ;;
start|stop|toggle) managed || fail 'Bridge is not installed.' ;;
*) fail 'Use install KOREADER_DIR, start, stop, status or toggle.' ;;
esac
managed || fail 'State directory is not owned by this bridge.'
mkdir "$STATE/lock" 2>/dev/null || fail 'Another bridge operation is in progress.'
trap 'rmdir "$STATE/lock"' EXIT
trap 'exit 1' HUP INT TERM
case "$1" in
install) install "$@" ;;
start) start ;;
stop) stop ;;
toggle) if owned_pid; then stop; else start; fi ;;
esac
