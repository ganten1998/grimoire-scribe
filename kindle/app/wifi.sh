#!/bin/sh
# The Kindle's own radio controls, its saved networks, a scan of the networks near it, the Wi-Fi
# sheet's add/forget changes, and a disconnect that leaves the radio on. A passphrase, the PSK
# Grimoire derives from it, or an enterprise username and password reaches this helper only on stdin;
# none is put in an argument, a variable exported to a child, a file or a log, and every secret is
# wiped once used. Nothing here prints a passphrase, a PSK, a username or a password. No services,
# firewall or boot settings are changed. The caller bounds execution in a child process.
set -u
get() { lipc-get-prop "$@" 2>/dev/null; }
# Set only for an EAP/WPA3 profile made through wpa_cli. It holds no name or secret; it lets
# status/current read the supplicant when wifid says it is not connected, without changing the
# behavior of wifid-managed WPA/WPA2 networks.
case "$0" in
    */*) supplicant_dir=${0%/*} ;;
    *) supplicant_dir=. ;;
esac
supplicant_marker=$supplicant_dir/.supplicant-wifi
# Set when an EAP/WPA3 profile was added through the supplicant. It keeps that profile under Saved
# after the active-link marker is cleared; it holds no name or secret.
supplicant_profiles=$supplicant_dir/.supplicant-profiles
# Sanitized events from the supplicant, for the EAP/WPA3 drop that happens after DHCP. It holds only
# event names and numeric fields, never a name, BSSID or credential.
probe_log=$supplicant_dir/.supplicant-events.log
read_radio() {
    wireless=$(get -i com.lab126.cmd wirelessEnable) || return 1
    enabled=$(get -i com.lab126.wifid enable) || return 1
    case "$wireless:$enabled" in 0:0|0:1|1:0|1:1) ;; *) return 1;; esac
}
report() {
    read_radio || return 1
    connected=0
    # Disconnect is asynchronous: wifid may briefly retain CONNECTED after its enable flag is 0.
    # Disabled radio flags take precedence over that stale connection report.
    if [ "$wireless:$enabled" = 1:1 ]; then
        if [ -f "$supplicant_marker" ]; then
            # The supplicant owns this profile. wifid's cmState can still name an old network, so it
            # must not be allowed to call that not-connected (or connected) while this one is active.
            # A completed 4-way is the link the sheet watches; DHCP runs separately in the background.
            if wpa_ready; then connected=1; else connected=0; fi
        else
            cm=$(get -s com.lab126.wifid cmState) || cm=unknown
            case "$cm" in CONNECTED) connected=1;; unknown|'') connected=unknown;; esac
        fi
    fi
    printf 'wireless=%s\nenabled=%s\nconnected=%s\n' "$wireless" "$enabled" "$connected"
    # The app uses this to keep automatic scans away from a supplicant-owned EAP/WPA3 link.
    if [ -f "$supplicant_marker" ]; then
        printf 'supplicant=1\n'
    fi
}
# Read wpa_cli's interactive event stream on stdin and append only a whitelisted event name and numeric
# fields. The raw line is never printed: it may hold an SSID or BSSID. This is deliberately separate from
# the helper's report, whose parser would reject an unknown line.
probe_read() {
    while IFS= read -r line; do
        line=$(printf '%s' "$line" | tr -d '\r')
        # The event is the first token after wpa_cli's <n> prefix. Do not search the whole line: an SSID
        # is untrusted text and could contain an event-shaped word.
        set -- $line
        event=
        while [ "$#" -gt 0 ]; do
            case "$1" in
                "<"*">CTRL-EVENT-"*) event=CTRL-EVENT-${1##*CTRL-EVENT-}; shift; break ;;
                "CTRL-EVENT-"*) event=$1; shift; break ;;
            esac
            shift
        done
        case "$event" in
            CTRL-EVENT-*) ;;
            *) continue ;;
        esac
        case "$event" in
            *[!A-Z-]*) continue ;;
        esac
        fields=
        for word in "$@"; do
            word=${word#\[}
            word=${word%\]}
            case "$word" in
                reason=*|locally_generated=*|id=*|status_code=*|auth_failures=*|duration=*)
                    key=${word%%=*}
                    value=${word#*=}
                    case "$value" in ''|*[!0-9]*) continue ;; esac
                    fields="$fields $key=$value"
                    ;;
            esac
        done
        printf '%s wifi: supplicant %s%s\n' "$(date '+%H:%M:%S')" "$event" "$fields" >> "$probe_log"
    done
}

# The supplicant's background scan is not the only scan that can deauth this profile: wifid itself may
# scan. Watch for the drop after the 4-way, in a detached process so closing the page does not end the
# watch. Its output is filtered by probe_read above, so only event names and numbers reach the file.
wpa_socket_dir() {
    for probe_dir in /var/run/wpa_supplicant /run/wpa_supplicant /var/run/wifi /run/wifi; do
        [ -e "$probe_dir/wlan0" ] && { printf '%s\n' "$probe_dir"; return 0; }
    done
    return 1
}
# A status snapshot names only the supplicant's state and AKM; ssid, bssid and address are dropped.
probe_state() {
    ps_path=$1
    ps_sock=$2
    if command -v timeout >/dev/null 2>&1; then
        ps_out=$(timeout 2 "$ps_path" -p "$ps_sock" -i wlan0 status 2>/dev/null) || ps_out=
    else
        ps_out=$("$ps_path" -p "$ps_sock" -i wlan0 status 2>/dev/null) || ps_out=
    fi
    [ -n "$ps_out" ] || return 0
    ps_line=
    for ps_key in wpa_state key_mgmt suppPortStatus eapState; do
        ps_value=$(printf '%s\n' "$ps_out" | sed -n "s/^$ps_key=//p" | head -n 1)
        case "$ps_value" in ''|*[!A-Za-z0-9_.:/-]*) continue ;; esac
        ps_line="$ps_line $ps_key=$ps_value"
    done
    ps_id=$(printf '%s\n' "$ps_out" | sed -n 's/^id=//p' | head -n 1)
    case "$ps_id" in ''|*[!0-9]*) ;; *) ps_line="$ps_line id=$ps_id" ;; esac
    if [ -n "$ps_line" ]; then
        printf '%s wifi: supplicant state%s\n' "$(date '+%H:%M:%S')" "$ps_line" >> "$probe_log"
    fi
    unset ps_out ps_line ps_key ps_value ps_id
}
probe_worker() {
    pw_path=$1
    pw_sock=$2
    printf '%s wifi: supplicant probe started\n' "$(date '+%H:%M:%S')" >> "$probe_log"
    ( sleep 5; probe_state "$pw_path" "$pw_sock"; sleep 10; probe_state "$pw_path" "$pw_sock";
      sleep 15; probe_state "$pw_path" "$pw_sock" ) &
    # The quit makes wpa_cli leave even where timeout is unavailable; timeout is the hard backstop.
    if command -v timeout >/dev/null 2>&1; then
        { sleep 90; printf 'quit\n'; } | timeout 95 "$pw_path" -p "$pw_sock" -i wlan0 2>/dev/null |
            sh "$0" probe-read
    else
        { sleep 30; printf 'quit\n'; } | "$pw_path" -p "$pw_sock" -i wlan0 2>/dev/null |
            sh "$0" probe-read
    fi
    printf '%s wifi: supplicant probe ended\n' "$(date '+%H:%M:%S')" >> "$probe_log"
    unset pw_path pw_sock
}
start_probe() {
    # Never start a watcher in a test or on a build host.
    [ -d /mnt/us ] || return 0
    wpa_path=$(command -v wpa_cli 2>/dev/null) || wpa_path=
    if [ -z "$wpa_path" ]; then
        for wpa_candidate in /usr/bin/wpa_cli /usr/sbin/wpa_cli; do
            [ -x "$wpa_candidate" ] && { wpa_path=$wpa_candidate; break; }
        done
    fi
    [ -n "$wpa_path" ] || return 0
    wpa_dir=$(wpa_socket_dir) || return 0
    # A new session keeps the watcher alive when the page or the app closes.
    if command -v setsid >/dev/null 2>&1; then
        setsid sh "$0" probe-worker "$wpa_path" "$wpa_dir" </dev/null >/dev/null 2>&1 &
    else
        sh "$0" probe-worker "$wpa_path" "$wpa_dir" </dev/null >/dev/null 2>&1 &
    fi
    unset wpa_path wpa_dir
}
# A read just after a profile change can catch wifid between states: try it again for a moment.
report_retry() {
    i=0
    while ! report; do
        [ "$i" -lt 20 ] || return 1
        sleep 0.1
        i=$((i + 1))
    done
}
# A connection-manager write can wait on wifid, so bound it where the firmware has a timeout: the caller
# kills the whole helper after a fixed time, and a wedged daemon must not cost it the report.
set_prop() {
    if command -v timeout >/dev/null 2>&1; then
        timeout 2 lipc-set-prop "$@" >/dev/null 2>&1
    else
        lipc-set-prop "$@" >/dev/null 2>&1
    fi
}
# wifid's connection manager rejoins a saved network as soon as it can. Its mode is a write-only string:
# "manual" keeps it off the air until the sheet asks for a network, "auto" gives it back. Best effort:
# firmware that does not know the property keeps its own behavior.
cm_mode() {
    set_prop -s com.lab126.wifid cmConnMode "$1"
}
radio_on() {
    [ "$wireless" = 1 ] || lipc-set-prop -i com.lab126.cmd wirelessEnable 1 >/dev/null 2>&1 || return 1
    [ "$enabled" = 1 ] || lipc-set-prop -i com.lab126.wifid enable 1 >/dev/null 2>&1 || return 1
    # A turn-on, a join or an add means "connect": give the manager its automatic mode back after a
    # disconnect made it manual.
    cm_mode auto
}
# One wpa_cli command against the supplicant, trying the control sockets wifid is known to use (its own
# default is not always one of them): "$@" is wpa_cli's own arguments, and its answer is printed. A bound
# keeps a missing socket from holding the helper open. The answers kept are terse (OK, one id line); a name
# never leaves this function, and nothing here names the network.
wpa_cmd() {
    wpa_path=$(command -v wpa_cli 2>/dev/null) || wpa_path=
    if [ -z "$wpa_path" ]; then
        for wpa_candidate in /usr/bin/wpa_cli /usr/sbin/wpa_cli; do
            [ -x "$wpa_candidate" ] && { wpa_path=$wpa_candidate; break; }
        done
    fi
    [ -n "$wpa_path" ] || return 1
    for wpa_dir in /var/run/wpa_supplicant /run/wpa_supplicant /var/run/wifi /run/wifi; do
        [ -e "$wpa_dir/wlan0" ] || continue
        if command -v timeout >/dev/null 2>&1; then
            wpa_out=$(timeout 2 "$wpa_path" -p "$wpa_dir" -i wlan0 "$@" 2>/dev/null) && { printf '%s\n' "$wpa_out"; return 0; }
        else
            wpa_out=$("$wpa_path" -p "$wpa_dir" -i wlan0 "$@" 2>/dev/null) && { printf '%s\n' "$wpa_out"; return 0; }
        fi
    done
    if command -v timeout >/dev/null 2>&1; then
        wpa_out=$(timeout 2 "$wpa_path" -i wlan0 "$@" 2>/dev/null) && { printf '%s\n' "$wpa_out"; return 0; }
    else
        wpa_out=$("$wpa_path" -i wlan0 "$@" 2>/dev/null) && { printf '%s\n' "$wpa_out"; return 0; }
    fi
    return 1
}
# One wpa_cli session against the supplicant, with the commands on its stdin (never in its arguments,
# so a password is not in any argv). It tries the control sockets wifid is known to use, and prints
# nothing: it succeeds when the session ran and at least $1 commands answered OK (wpa_cli puts its
# prompt before each answer) with no FAIL. The output, which may echo a command, stays in this
# function and is never printed.
wpa_input() {
    want=${1:-1}
    wpa_path=$(command -v wpa_cli 2>/dev/null) || wpa_path=
    if [ -z "$wpa_path" ]; then
        for wpa_candidate in /usr/bin/wpa_cli /usr/sbin/wpa_cli; do
            [ -x "$wpa_candidate" ] && { wpa_path=$wpa_candidate; break; }
        done
    fi
    [ -n "$wpa_path" ] || return 1
    inp=$(cat) || return 1
    # End the interactive session explicitly. Without this, wpa_cli can sit waiting for another
    # command (or re-trying a dead socket) until timeout kills it, losing the answers to the
    # commands that already ran. quit itself needs no answer.
    inp="$inp
quit"
    for wpa_dir in /var/run/wpa_supplicant /run/wpa_supplicant /var/run/wifi /run/wifi ''; do
        if [ -n "$wpa_dir" ]; then
            [ -e "$wpa_dir/wlan0" ] || continue
            set -- -p "$wpa_dir" -i wlan0
        else
            set -- -i wlan0
        fi
        if command -v timeout >/dev/null 2>&1; then
            wpa_out=$(printf '%s\n' "$inp" | timeout 3 "$wpa_path" "$@" 2>/dev/null)
        else
            wpa_out=$(printf '%s\n' "$inp" | "$wpa_path" "$@" 2>/dev/null)
        fi
        # Normalize wpa_cli's interactive answer before counting: the prompt can be "> OK",
        # ">OK" or followed by a CR, and trailing spaces must not hide an OK or FAIL.
        wpa_lines=$(printf '%s\n' "$wpa_out" | tr -d '\r' | sed 's/^[[:space:]]*> *//; s/[[:space:]]*$//')
        oks=$(printf '%s\n' "$wpa_lines" | grep -c '^OK$')
        fails=$(printf '%s\n' "$wpa_lines" | grep -c '^FAIL')
        if [ "$fails" -eq 0 ] && [ "$oks" -ge "$want" ]; then
            unset inp wpa_out wpa_lines oks fails
            return 0
        fi
    done
    unset inp wpa_out wpa_lines oks fails
    return 1
}
wpa_ready() {
    wpa_state=$(wpa_cmd status 2>/dev/null) || return 1
    case "$wpa_state" in
        *"wpa_state=COMPLETED"*)
            # wpa_cli status names an EAP AKM "WPA2/IEEE 802.1X" or "WPA3/IEEE 802.1X", not
            # the network block's "WPA-EAP"; accept both forms, and SAE for WPA3-Personal.
            # (The WPA-EAP pattern also matches WPA-EAP-SHA256.)
            case "$wpa_state" in
                *"key_mgmt=SAE"*|*"key_mgmt=FT-SAE"*|*"key_mgmt=WPA-EAP"*|*"IEEE 802.1X"*)
                    unset wpa_state
                    return 0
                    ;;
            esac
            ;;
    esac
    unset wpa_state
    return 1
}
# A bounded DHCP client for the supplicant-created EAP/WPA3 profile: wifid starts DHCP only for
# profiles it creates. Output is discarded; no network name or secret is involved.
dhcp_client() {
    for dhcp_candidate in /sbin/udhcpc /usr/sbin/udhcpc /usr/bin/udhcpc /bin/udhcpc \
                          /sbin/dhcpcd /usr/sbin/dhcpcd /usr/bin/dhcpcd \
                          /sbin/dhclient /usr/sbin/dhclient /usr/bin/dhclient; do
        [ -x "$dhcp_candidate" ] || continue
        case "$dhcp_candidate" in
            *udhcpc) "$dhcp_candidate" -i wlan0 -b -q -n -t 5 -T 2 >/dev/null 2>&1 ;;
            *dhcpcd) "$dhcp_candidate" -q -t 15 wlan0 >/dev/null 2>&1 ;;
            *dhclient) "$dhcp_candidate" -1 -timeout 15 wlan0 >/dev/null 2>&1 ;;
        esac
        return 0
    done
    if command -v busybox >/dev/null 2>&1; then
        busybox udhcpc -i wlan0 -b -q -n -t 5 -T 2 >/dev/null 2>&1
    fi
}
# The helper must not wait for the 4-way: the app watches Current after the profile is added.
# Run this in the background, long enough for the EAP exchange, then obtain an address.
dhcp_later() {
    # Never run a DHCP client in a test or on a build host; the Kindle mount says it is the device.
    [ -d /mnt/us ] || return 0
    dhcp_i=0
    while [ "$dhcp_i" -lt 60 ]; do
        if wpa_ready; then
            # The 4-way is done; watch for the drop that follows DHCP.
            start_probe
            dhcp_client
            return
        fi
        sleep 0.5
        dhcp_i=$((dhcp_i + 1))
    done
}
# cmState is asynchronous: wifid may take a moment to leave CONNECTED after either request.
wait_disconnected() {
    i=0
    while [ "$i" -lt 6 ]; do
        cm=$(get -s com.lab126.wifid cmState) || return 1
        [ "$cm" != CONNECTED ] && return 0
        sleep 0.25
        i=$((i + 1))
    done
    return 1
}
# The network names in one of wifid's hasharray properties, read as KOReader reads them (an empty
# hasharray in: profileData, the saved networks, and currentEssid, the one connected, only answer).
# profileData's answer holds each saved network's password (psk): it goes from lipc-hash-prop straight
# into the reader below, which keeps the essid values alone and nothing else, so no password reaches a
# variable, a file, this helper's output or a log. The reader knows one format: key = "value" (or
# "key": "value", key: "value"), values quoted with backslash escapes, numbers bare or in parentheses (the
# Scribe's firmware 5.20.1 answers "Hash Index: N" and then tab-indented key = "value" and key = (number)
# lines, a psk line among them, read on the owner's Scribe on 2026-11-11 with every value masked), braces, brackets,
# commas and words without a value around them. Anything else, an unquoted value above all (a
# password among them), makes it print no name at all ("list=0"): it fails closed. A name with a
# backslash (an escape this cannot undo), a control character, or more than 32 bytes is left out.
# Each name is printed in hex ("saved=<hex>", then "list=1"), so no byte of it can break a line; given
# a hex name as $2 it prints instead the one saved name with that hex, as it is, or nothing, and with
# "netid" as $3 it prints that same profile's integer id instead ("netid=<n>"), or nothing.
essids() {
    { printf '{}\n' | lipc-hash-prop com.lab126.wifid "$1" 2>/dev/null || echo '#lipc-failed'; } |
        LC_ALL=C awk -v want="${2:-}" -v need_netid="${3:-}" '
function commit() {
    if (want != "" && cur_h == want && got_n == "") got_n = cur_n
    cur_h = ""; cur_n = ""; pre_n = ""
}
function setnet(v) {
    if (v !~ /^-?[0-9]+$/) return
    if (cur_h != "" && cur_n != "") commit()
    if (cur_h == "") pre_n = v; else cur_n = v
}
function take(v,   i, b, h, p) {
    if (v == "" || length(v) > 32 || index(v, "\\")) return
    h = ""
    for (i = 1; i <= length(v); i++) {
        b = ord[substr(v, i, 1)]
        if (b < 32 || b == 127) return
        h = h sprintf("%02x", b)
    }
    if (want != "") {
        p = pre_n
        commit()
        cur_h = h; cur_n = p
        if (h == want && !picked) { picked = 1; print v }
        return
    }
    if (h in seen || n >= 64) return
    seen[h] = 1; out[++n] = h
}
BEGIN { for (i = 1; i < 256; i++) ord[sprintf("%c", i)] = i; n = 0; bad = 0; failed = 0; picked = 0; cur_h = ""; cur_n = ""; pre_n = ""; got_n = "" }
$0 == "#lipc-failed" { failed = 1; next }
{
    s = $0; key = ""
    while (s != "" && !bad) {
        if (substr(s, 1, 1) == "\"") {
            if (!match(s, /^"([^"\\]|\\.)*"/)) { bad = 1; break }
            v = substr(s, 2, RLENGTH - 2); s = substr(s, RLENGTH + 1)
            t = s; sub(/^[ \t]*/, "", t); c = substr(t, 1, 1)
            if (key == "" && (c == "=" || c == ":")) { key = v; s = substr(t, 2); continue }
            if (key == "essid") take(v)
            key = ""; continue
        }
        if (match(s, /^[A-Za-z_][A-Za-z0-9_]*[ \t]*[=:]/)) {
            if (key != "") { bad = 1; break }
            key = substr(s, 1, RLENGTH); sub(/[ \t]*[=:]$/, "", key); s = substr(s, RLENGTH + 1); continue
        }
        if (match(s, /^[ \t]+/)) { s = substr(s, RLENGTH + 1); continue }
        if (match(s, /^-?[0-9]+(\.[0-9]+)?/)) {
            v = substr(s, 1, RLENGTH); s = substr(s, RLENGTH + 1)
            if (want != "" && key == "netid") setnet(v)
            key = ""; continue
        }
        if (match(s, /^\([A-Za-z0-9_.+-]*\)/)) {
            v = substr(s, 2, RLENGTH - 2); s = substr(s, RLENGTH + 1)
            if (want != "" && key == "netid") setnet(v)
            key = ""; continue
        }
        if (key != "") { bad = 1; break }
        if (match(s, /^[][{}(),;=:]+/)) { s = substr(s, RLENGTH + 1); continue }
        if (match(s, /^[A-Za-z_][A-Za-z0-9_]*/)) { s = substr(s, RLENGTH + 1); continue }
        bad = 1
    }
}
END {
    if (want != "") {
        commit()
        if (need_netid != "" && got_n != "") print "netid=" got_n
        exit 0
    }
    if (failed || bad) { print "list=0"; exit 0 }
    for (i = 1; i <= n; i++) print "saved=" out[i]
    print "list=1"
}'
}
# The supplicant's saved networks are not in wifid's profileData. Encode their names as hex too.
supplicant_id() {
    wpa_cmd list_networks 2>/dev/null | LC_ALL=C awk -F '\t' -v want="$1" '
        $1 ~ /^[0-9]+$/ && $2 == want { print $1; exit }'
}
supplicant_saved() {
    # A profile added before the saved-profiles marker existed is still under the active marker.
    if [ ! -f "$supplicant_profiles" ] && [ -f "$supplicant_marker" ]; then
        : > "$supplicant_profiles" 2>/dev/null || true
    fi
    [ -f "$supplicant_profiles" ] || return 0
    wpa_cmd list_networks 2>/dev/null | LC_ALL=C awk -F '\t' '
        BEGIN { for (i = 1; i < 256; i++) ord[sprintf("%c", i)] = i }
        $1 ~ /^[0-9]+$/ && NF >= 2 && length($2) > 0 && length($2) <= 32 {
            h = ""; bad = 0
            for (i = 1; i <= length($2); i++) {
                b = ord[substr($2, i, 1)]
                if (b < 32 || b == 127) { bad = 1; break }
                h = h sprintf("%02x", b)
            }
            if (!bad) print "saved=" h
        }'
}
hex_decode() {
    printf '%s' "$1" | LC_ALL=C awk '
        BEGIN { for (i = 1; i < 256; i++) o[sprintf("%02x", i)] = sprintf("%c", i) }
        { for (i = 1; i <= length($0); i += 2) printf "%s", o[substr($0, i, 2)] }'
}
saved_all() {
    saved=$(essids profileData)
    case "$saved" in
        *"list=1"*)
            printf '%s\n' "$saved" | sed '$d'
            supplicant_saved
            printf 'list=1\n'
            ;;
        *) printf '%s\n' "$saved" ;;
    esac
    unset saved
}
# The network connected to, in hex, or nothing (not connected, or not known).
current() {
    if [ "$connected" = 1 ]; then
        current_hex=
        if [ -f "$supplicant_marker" ]; then
            # A supplicant-created profile is not in wifid's currentEssid. Read the name from the
            # supplicant first and print it only as hex, never as text.
            supplicant_status=$(wpa_cmd status 2>/dev/null) || supplicant_status=
            supplicant_name=$(printf '%s\n' "$supplicant_status" | sed -n 's/^ssid=//p' | head -n 1)
            if [ -n "$supplicant_name" ]; then
                current_hex=$(printf '%s' "$supplicant_name" | od -An -tx1 | tr -d ' \n')
            fi
            unset supplicant_status supplicant_name
        fi
        if [ -z "$current_hex" ]; then
            current_hex=$(essids currentEssid | sed -n 's/^saved=//p' | head -n 1)
        fi
        printf 'current=%s\n' "$current_hex"
        unset current_hex
    else
        printf 'current=\n'
    fi
}
# One line per network near the Kindle: near=<hex-essid>,<signal>,<signal_max>,<security>,<known>. The
# scanList hasharray is read as KOReader reads it; anything this reader does not know fails the whole
# scan (no list) rather than guessing a security type. A name with a backslash, a control character or
# more than 32 bytes is left out; the strongest BSSID of a name stands for it.
# security is open, wep, wpa, wpa2, wpa3, owe, enterprise, enterprise192, or unknown.
nearby() {
    { printf '{}\n' | lipc-hash-prop com.lab126.wifid scanList 2>/dev/null || echo '#lipc-failed'; } |
        LC_ALL=C awk '
function trim(v) { sub(/^[ \t]+/, "", v); sub(/[ \t]+$/, "", v); return v }
function setv(k, v) {
    if (k == "essid") {
        if (have && essid != "") flush()
        essid = v; have = 1
        signal = 0; signal_max = 5; km = ""; known = ""
    } else if (k == "signal") signal = v + 0
    else if (k == "signal_max") signal_max = v + 0
    else if (k == "key_mgmt") km = v
    else if (k == "known") known = v
}
function flush(   i, b, h, u, sec, kn) {
    if (!have || essid == "") { essid = ""; have = 0; return }
    if (length(essid) > 32 || index(essid, "\\")) { bad = 1; essid = ""; have = 0; return }
    h = ""
    for (i = 1; i <= length(essid); i++) {
        b = ord[substr(essid, i, 1)]
        if (b < 32 || b == 127) { bad = 1; essid = ""; have = 0; return }
        h = h sprintf("%02x", b)
    }
    u = toupper(km)
    # WPA3-Enterprise 192-bit (WPA-EAP-SUITE-B-192) needs EAP-TLS and GCMP-256, not PEAP/MSCHAPv2:
    # it is named apart so the sheet can say it is not supported rather than try it.
    # An AP that also offers SHA-256 EAP supports the non-192-bit profile even if it advertises
    # SUITE-B-192 alongside it; only a 192-bit-only AP stays gated.
    if (u ~ /SUITE|192/ && u !~ /WPA-EAP-SHA256/) sec = "enterprise192"
    else if (u ~ /ENTERPRISE|8021X|IEEE8021X|EAP/) sec = "enterprise"
    else if (u ~ /OWE|ENHANCED.OPEN/) sec = "owe"
    else if (u ~ /WPA3|SAE/) sec = "wpa3"
    else if (u ~ /WPA2/) sec = "wpa2"
    else if (u ~ /WPA/) sec = "wpa"
    else if (u ~ /WEP/) sec = "wep"
    else if (u == "" || u ~ /OPEN|NONE/) sec = "open"
    else sec = "unknown"
    kn = (known == "yes" || known == "1" || known == "true") ? 1 : 0
    if (n < 64) out[++n] = "near=" h "," signal "," signal_max "," sec "," kn
    essid = ""; have = 0; signal = 0; signal_max = 5; km = ""; known = ""
}
BEGIN {
    for (i = 1; i < 256; i++) ord[sprintf("%c", i)] = i
    n = 0; bad = 0; have = 0; key = ""; essid = ""; signal = 0; signal_max = 5; km = ""; known = ""
}
$0 == "#lipc-failed" { bad = 1; next }
{
    s = $0
    while (s != "" && !bad) {
        if (match(s, /^[ \t\r]+/)) { s = substr(s, RLENGTH + 1); continue }
        c = substr(s, 1, 1)
        if (c == "}") { flush(); s = substr(s, 2); continue }
        if (c == "{" || c == "[" || c == "]" || c == "(" || c == ")" || c == "," || c == ";") { s = substr(s, 2); continue }
        if (c == "\"") {
            if (!match(s, /^"([^"\\]|\\.)*"/)) { bad = 1; break }
            v = substr(s, 2, RLENGTH - 2); s = substr(s, RLENGTH + 1)
            t = s; sub(/^[ \t]*/, "", t)
            if (key == "" && (substr(t, 1, 1) == "=" || substr(t, 1, 1) == ":")) { key = v; s = substr(t, 2); continue }
            if (key != "") { setv(key, v); key = "" }
            continue
        }
        if (match(s, /^[A-Za-z_][A-Za-z0-9_.:+\/-]*/)) {
            v = substr(s, 1, RLENGTH); s = substr(s, RLENGTH + 1)
            t = s; sub(/^[ \t]*/, "", t)
            if (key == "" && (substr(t, 1, 1) == "=" || substr(t, 1, 1) == ":")) { key = v; s = substr(t, 2); continue }
            if (key != "") { setv(key, v); key = "" }
            continue
        }
        if (match(s, /^-?[0-9]+(\.[0-9]+)?/)) {
            v = substr(s, 1, RLENGTH); s = substr(s, RLENGTH + 1)
            if (key != "") { setv(key, v); key = "" }
            continue
        }
        if (c == "=" || c == ":") { s = substr(s, 2); continue }
        bad = 1
    }
}
END {
    if (bad) { print "scan=bad"; exit 1 }
    flush()
    for (i = 1; i <= n; i++) print out[i]
    print "near_end=1"
}'
}
case "${1:-}" in
    probe-read) probe_read;;
    status) report;;
    current) report || exit 1; current;;
    networks) report || exit 1; current; saved_all;;
    scan)
        # A tap's scan, or the sheet's quiet 20 s refresh. The radio is never turned on by a scan (the sheet
        # offers that), and the caller bounds this helper, so a wedged scanState fails rather than waiting.
        # The caller keeps the previous list on a quiet refresh; this helper has no quiet mode.
        report || exit 1
        if [ "$wireless:$enabled" != 1:1 ]; then printf 'scan=off\n'; exit 0; fi
        lipc-set-prop -s com.lab126.wifid scan "" >/dev/null 2>&1 || exit 1
        i=0; state=""
        while [ "$i" -lt 80 ]; do
            state=$(get -s com.lab126.wifid scanState) || exit 1
            [ "$state" = idle ] && break
            sleep 0.25
            i=$((i + 1))
        done
        [ "$state" = idle ] || exit 1
        printf 'scan=ok\n'
        nearby
        ;;
    add)
        # add <essid> <open|wpa|wpa2|wpa3|eap>: create a profile and ask the radio to connect, then
        # read the networks back. Open takes no secret. WPA/WPA2 takes the PSK on stdin (Grimoire
        # derives it; a direct caller may send a passphrase, which wpa_passphrase turns into the PSK
        # where the firmware has it). WPA3-Personal takes its passphrase on stdin and creates a SAE
        # network through wpa_cli: SAE needs the passphrase itself, not the PBKDF2 PSK. EAP takes the
        # username on stdin's first line and the password on its second; it creates a PEAP/MSCHAPv2
        # network through wpa_cli that is capable of PMF, so it joins both WPA2-Enterprise and
        # WPA3-Enterprise (non-192-bit). wpa_cli's commands carry every secret on its stdin, never in
        # an argument. Every secret is wiped and none is printed or logged. The profile hash for
        # WPA/WPA2 (holding only the PSK, never the passphrase) goes to createProfile on stdin, so no
        # secret is in any argument. A failure leaves the profiles as they were. Each failure has its
        # own exit, so the app's log can name the step (nothing here ever names the network): 1 the
        # radio could not be read or turned on, 18 no valid PSK (a malformed one, or wpa_passphrase
        # missing/unexpected output), 19 wifid refused the profile, 20 no read came back after the
        # change, 26 no wpa_cli or control socket, 27 the supplicant refused the network, 28 the
        # supplicant does not list SAE (WPA3-Personal) as a capability.
        essid=${2-}; method=${3-}
        case "$method" in open|wpa|wpa2|wpa3|eap) ;; *) exit 2;; esac
        case "$essid" in ''|*[[:cntrl:]]*) exit 2;; esac
        [ "${#essid}" -le 32 ] || exit 2
        read_radio || exit 1
        radio_on || exit 1
        # (quoted for wpa_cli's parser; no shell ever evaluates the values)
        q() { printf '%s' "$1" | sed 's/\\/\\\\/g; s/"/\\"/g'; }
        if [ "$method" = eap ]; then
            IFS= read -r identity || exit 2
            IFS= read -r password || { unset identity; exit 2; }
            case "$identity" in ''|*[[:cntrl:]]*) unset identity password; exit 2;; esac
            case "$password" in ''|*[[:cntrl:]]*) unset identity password; exit 2;; esac
            [ "${#identity}" -le 128 ] && [ "${#password}" -le 128 ] || { unset identity password; exit 2; }
            # wifid's automatic manager would rejoin its own saved network and can disassociate this
            # supplicant-owned profile after EAP succeeds. Keep the manager manual while it is active.
            cm_mode manual
            nid=$(wpa_cmd add_network 2>/dev/null | sed -n 's/^\([0-9][0-9]*\)$/\1/p' | tail -n 1)
            case "$nid" in
                ''|*[!0-9]*) cm_mode auto; unset identity password; exit 26;;
            esac
            # WPA3-Enterprise (non-192-bit) is the same EAP exchange with SHA-256 AKM and protected
            # management frames. Both AKMs are offered so the same profile still joins a WPA2-Enterprise
            # network; PMF stays optional (1), which an AP that requires it accepts. A supplicant too
            # old to know WPA-EAP-SHA256 keeps WPA-EAP alone. 192-bit (WPA-EAP-SUITE-B-192) is not used.
            # Best effort: wifid may run the supplicant's background scan (bgscan), whose learned
            # channel list can send this link off-channel and make the client deauth. This profile
            # does not roam, so turn that scan off before selecting it.
            printf 'set_network %s bgscan ""\n' "$nid" | wpa_input 1 || true
            caps=$(wpa_cmd get_capability key_mgmt 2>/dev/null) || caps=
            if printf '%s\n' "$caps" | grep -qw WPA-EAP-SHA256; then
                kmgmt='"WPA-EAP WPA-EAP-SHA256"'
            else
                kmgmt=WPA-EAP
            fi
            qid=$(q "$identity")
            qpass=$(q "$password")
            qessid=$(q "$essid")
            # Exactly one argument per placeholder: an extra $nid here makes printf repeat
            # the whole block with empty fields, and the repeated "set_network $nid ssid \"\""
            # clears the SSID just after select_network (the 4-way then never gets a 2/4).
            cmds=$(printf 'set_network %s ssid "%s"
set_network %s key_mgmt %s
set_network %s eap PEAP
set_network %s identity "%s"
set_network %s password "%s"
set_network %s phase2 "auth=MSCHAPV2"
set_network %s ieee80211w 1
set_network %s scan_ssid 1
enable_network %s
select_network %s
' "$nid" "$qessid" "$nid" "$kmgmt" "$nid" "$nid" "$qid" "$nid" "$qpass" "$nid" "$nid" "$nid" "$nid" "$nid")
            if ! printf '%s\n' "$cmds" | wpa_input 10; then
                printf 'remove_network %s\n' "$nid" | wpa_input 1 || true
                cm_mode auto
                unset identity password caps qid qpass qessid cmds kmgmt
                exit 27
            fi
            # Best effort: a supplicant that cannot save still has the live network.
            printf 'save_config\n' | wpa_input 1 || true
            # Mark the profile before the report: status and current must read the supplicant, not a
            # stale wifid connection, while the 4-way and DHCP run.
            : > "$supplicant_marker" 2>/dev/null || true
            : > "$supplicant_profiles" 2>/dev/null || true
            # wifid does not run DHCP for a profile it did not create. Let the supplicant finish the
            # 4-way, then obtain an address in the background; the app watches Current meanwhile.
            ( dhcp_later ) </dev/null >/dev/null 2>&1 &
            unset identity password caps qid qpass qessid cmds kmgmt
            report_retry || { rm -f "$supplicant_marker" 2>/dev/null || true; cm_mode auto; exit 20; }
            current
            saved_all
            exit 0
        fi
        if [ "$method" = wpa3 ]; then
            IFS= read -r password || exit 2
            case "$password" in ''|*[![:print:]]*) unset password; exit 2;; esac
            [ "${#password}" -ge 8 ] && [ "${#password}" -le 63 ] || { unset password; exit 2; }
            # A supplicant without SAE cannot join this network at all: name that step rather than
            # creating a profile that never works. An empty answer means the capability query itself
            # is unavailable (an older wpa_cli): try the network rather than refuse it.
            caps=$(wpa_cmd get_capability key_mgmt 2>/dev/null) || caps=
            if [ -n "$caps" ] && ! printf '%s\n' "$caps" | grep -qw SAE; then
                unset password caps
                exit 28
            fi
            # As for EAP: wifid's automatic manager must not fight this supplicant-owned profile.
            cm_mode manual
            # Best effort: offer both SAE password-to-element methods, for an AP that only takes H2E.
            printf 'set sae_pwe 2\n' | wpa_input 1 || true
            nid=$(wpa_cmd add_network 2>/dev/null | sed -n 's/^\([0-9][0-9]*\)$/\1/p' | tail -n 1)
            case "$nid" in
                ''|*[!0-9]*) cm_mode auto; unset password caps; exit 26;;
            esac
            qpass=$(q "$password")
            qessid=$(q "$essid")
            # Best effort: as for EAP, keep the supplicant's background scan from leaving this link.
            printf 'set_network %s bgscan ""\n' "$nid" | wpa_input 1 || true
            cmds=$(printf 'set_network %s ssid "%s"
set_network %s key_mgmt SAE
set_network %s psk "%s"
set_network %s ieee80211w 2
set_network %s scan_ssid 1
enable_network %s
select_network %s
' "$nid" "$qessid" "$nid" "$nid" "$qpass" "$nid" "$nid" "$nid" "$nid")
            if ! printf '%s\n' "$cmds" | wpa_input 7; then
                printf 'remove_network %s\n' "$nid" | wpa_input 1 || true
                cm_mode auto
                unset password caps qpass qessid cmds
                exit 27
            fi
            # Best effort: a supplicant that cannot save still has the live network.
            printf 'save_config\n' | wpa_input 1 || true
            # Mark the profile before the report, as for EAP.
            : > "$supplicant_marker" 2>/dev/null || true
            : > "$supplicant_profiles" 2>/dev/null || true
            # As for EAP: the supplicant owns this profile, so obtain its address here.
            ( dhcp_later ) </dev/null >/dev/null 2>&1 &
            unset password caps qpass qessid cmds
            report_retry || { rm -f "$supplicant_marker" 2>/dev/null || true; cm_mode auto; exit 20; }
            current
            saved_all
            exit 0
        fi
        # A wifid-managed profile replaces any supplicant-owned EAP/WPA3 profile.
        rm -f "$supplicant_marker" 2>/dev/null || true
        if [ "$method" = open ]; then
            psk=""
        else
            IFS= read -r secret || exit 2
            psk=
            # Grimoire sends the PSK it derived (64 hex digits). A direct call may send a passphrase
            # (8-63 printable bytes) instead; wpa_passphrase turns that into the PSK where it exists.
            case "$secret" in
                *[!0-9a-f]*) ;;
                *) if [ "${#secret}" -eq 64 ]; then psk=$secret; fi ;;
            esac
            if [ -z "$psk" ]; then
                pass=$secret
                case "$pass" in *[![:print:]]*) unset pass secret; exit 2;; esac
                [ "${#pass}" -ge 8 ] && [ "${#pass}" -le 63 ] || { unset pass secret; exit 2; }
                psk=$(printf '%s\n' "$pass" | wpa_passphrase "$essid" 2>/dev/null | sed -n 's/^[[:space:]]*psk=//p' | head -n 1)
                unset pass
                case "$psk" in *[!0-9a-f]*|'') unset secret; exit 18;; esac
                [ "${#psk}" -eq 64 ] || { unset secret; exit 18; }
            fi
            unset secret
        fi
        esc=$(printf '%s' "$essid" | sed 's/\\/\\\\/g; s/"/\\"/g')
        if [ "$method" = open ]; then
            hash=$(printf '{essid="%s", smethod="%s", secured="no"}\n' "$esc" "$method")
        else
            hash=$(printf '{essid="%s", smethod="%s", secured="yes", psk="%s"}\n' "$esc" "$method" "$psk")
        fi
        printf '%s\n' "$hash" | lipc-hash-prop com.lab126.wifid createProfile >/dev/null 2>&1 || exit 19
        unset hash esc psk
        # (the profile is saved now; a connection refused here does not undo it)
        lipc-set-prop -s com.lab126.cmd ensureConnection "wifi:$essid" >/dev/null 2>&1 || true
        report_retry || exit 20
        current
        saved_all
        ;;
    delete)
        # delete <essid>: forget one saved profile, then read the networks back. wifid takes the name as a
        # string on some firmware and the profile's integer netid on others (the Scribe's wifid), and the
        # supplicant keeps a network of its own: all three are tried in turn, and the profile counts as
        # gone only when a fresh read no longer lists it. Each failure has its own exit, so the app's log
        # can name the step (nothing here ever names the network): 21 the radio could not be read, 23 no
        # read came back after a change, 24 no netid could be read for the profile, 25 wifid kept the
        # profile after the netid write.
        essid=${2-}
        case "$essid" in ''|*[[:cntrl:]]*) exit 2;; esac
        [ "${#essid}" -le 32 ] || exit 2
        read_radio || exit 21
        # the name in hex, the form the saved list prints
        name=$(printf '%s\n' "$essid" | LC_ALL=C awk 'BEGIN { for (i = 1; i < 256; i++) o[sprintf("%c", i)] = i }
            { for (i = 1; i <= length($0); i++) printf "%02x", o[substr($0, i, 1)] }')
        # 0 once the profile is no longer saved, 1 while it is, 2 when the list cannot be read
        still=2
        check() {
            i=0
            while :; do
                list=$(essids profileData)
                case $list in *list=1*) break;; esac
                [ "$i" -lt 10 ] || { still=2; return; }
                sleep 0.1
                i=$((i + 1))
            done
            if printf '%s\n' "$list" | grep -qx "saved=$name"; then still=1; else still=0; fi
        }
        lipc-set-prop -s com.lab126.wifid deleteProfile "$essid" >/dev/null 2>&1 || true
        check
        nid=
        if [ "$still" = 1 ]; then
            # firmware that takes an integer: the Scribe's wifid wants the profile's netid. profileData
            # has it; netidValue is the other property that maps a saved name to that same id.
            nid=$(essids profileData "$name" netid | sed -n 's/^netid=//p' | head -n 1)
            [ -n "$nid" ] || nid=$(essids netidValue "$name" netid | sed -n 's/^netid=//p' | head -n 1)
            case "$nid" in ''|*[!0-9]*) nid=;; esac
        fi
        if [ "$still" = 1 ] && [ -n "$nid" ]; then
            lipc-set-prop -i com.lab126.wifid deleteProfile "$nid" >/dev/null 2>&1 || true
            check
        fi
        if [ "$still" = 1 ]; then
            # wpa_cli prints a saved network's name plainly when it is printable, which is the one matched
            id=$(wpa_cmd list_networks 2>/dev/null | LC_ALL=C awk -F'\t' -v want="$essid" '$2 == want { print $1; exit }')
            if [ -n "$id" ] && [ "$(wpa_cmd remove_network "$id" 2>/dev/null)" = OK ]; then
                wpa_cmd save_config >/dev/null 2>&1 || true
                check
            fi
        fi
        if [ -f "$supplicant_profiles" ]; then
            sid=$(supplicant_id "$essid")
            if [ -n "$sid" ]; then
                wpa_cmd remove_network "$sid" >/dev/null 2>&1 || true
                wpa_cmd save_config >/dev/null 2>&1 || true
                check
            fi
            if [ -z "$(supplicant_id "$essid")" ]; then
                if ! wpa_cmd list_networks 2>/dev/null | LC_ALL=C awk -F '\t' '$1 ~ /^[0-9]+$/ { found = 1 } END { exit found ? 0 : 1 }'; then
                    rm -f "$supplicant_profiles" "$supplicant_marker" 2>/dev/null || true
                fi
            fi
        fi
        case $still in
            0) ;;
            1) [ -n "$nid" ] && exit 25 || exit 24;;
            *) exit 23;;
        esac
        report_retry || exit 23
        current
        saved_all
        ;;
    disconnect)
        # Drop the network connected to, keeping Wi-Fi on. wifid's connection manager rejoins a profile as
        # soon as the link drops, so it goes to its manual mode first: the drop stays down until the sheet
        # asks for a network (join, add and on give the manager its automatic mode back). Then cmDisconnect
        # is asked for the current profile (some firmware ignores it without that name), then without a
        # name, then through wpa_supplicant. The supplicant itself can reassociate within a moment of the
        # drop, and so can a bare reconnect from wifid, so once the link is down its network is disabled at
        # the supplicant with wpa_cli before the second look; the report must say connected=0 before this
        # counts as done. A later join, add or turn-on asks wifid to connect, which manages its own
        # networks. A disconnect that does not take re-enables that network and asks for auto again, rather
        # than leaving a radio on but unable to reconnect. Each failure has its own exit, so the app's log
        # can name the step (nothing here ever names the network): 10 the radio could not be read, 11 the
        # connection state could not, 12 no route dropped the link, 13 it came straight back past the
        # disconnect pin, 17 it came back past the disable too, 15 it came back with no wpa_cli to pin it
        # down at all, 14 the last read could not be made.
        read_radio || exit 10
        [ "$wireless:$enabled" = 1:1 ] || exit 0
        rm -f "$supplicant_marker" 2>/dev/null || true
        cm=$(get -s com.lab126.wifid cmState) || exit 11
        if [ "$cm" = CONNECTED ]; then
            cm_mode manual
            down=0
            # the supplicant's own id for this network, so the drop can be pinned below
            id=$(wpa_cmd status 2>/dev/null | sed -n 's/^id=//p' | head -n 1) || id=
            case "$id" in ''|*[!0-9]*) id=;; esac
            profile=$(get -s com.lab126.wifid cmProfile) || profile=
            if [ -n "$profile" ] && set_prop -s com.lab126.wifid cmDisconnect "$profile"; then
                wait_disconnected && down=1
            fi
            if [ "$down" = 0 ]; then
                set_prop -s com.lab126.wifid cmDisconnect "" || :
                wait_disconnected && down=1
            fi
            if [ "$down" = 0 ] && wpa_cmd disconnect >/dev/null 2>&1; then
                wait_disconnected && down=1
            fi
            if [ "$down" = 0 ]; then
                cm_mode auto
                exit 12
            fi
            # Pin the drop at the supplicant: a plain disconnect still lets it (or a bare reconnect from
            # wifid) bring the link straight back, so the current network is disabled too. Best effort:
            # without wpa_cli, or without its id for this network, the drop still has to prove itself
            # below.
            pinned=0; disabled=0
            if wpa_cmd disconnect >/dev/null 2>&1; then
                pinned=1
                if [ -n "$id" ] && wpa_cmd disable_network "$id" >/dev/null 2>&1; then
                    disabled=1
                fi
            fi
            sleep 1
            cm=$(get -s com.lab126.wifid cmState) || { cm_mode auto; exit 11; }
            if [ "$cm" = CONNECTED ]; then
                # Leave no trace behind: the network comes back on, and so does the manager.
                [ -z "$id" ] || wpa_cmd enable_network "$id" >/dev/null 2>&1 || :
                cm_mode auto
                if [ "$disabled" = 1 ]; then exit 17; elif [ "$pinned" = 1 ]; then exit 13; else exit 15; fi
            fi
        fi
        report_retry || exit 14
        [ "$connected" = 0 ] || exit 14
        current
        ;;
    on|off)
        # Refuse an unknown or unavailable backend before making any change.
        read_radio || exit 1
        # A radio change returns the connection manager to automatic mode; no supplicant profile is
        # left in charge of the link.
        rm -f "$supplicant_marker" 2>/dev/null || true
        if [ "$1" = on ]; then
            radio_on || exit 1
        else
            [ "$enabled" = 0 ] || lipc-set-prop -i com.lab126.wifid enable 0 >/dev/null 2>&1 || exit 1
            [ "$wireless" = 0 ] || lipc-set-prop -i com.lab126.cmd wirelessEnable 0 >/dev/null 2>&1 || exit 1
        fi
        report || exit 1
        if [ "$1" = on ]; then [ "$wireless:$enabled" = 1:1 ]; else [ "$wireless:$enabled" = 0:0 ]; fi
        ;;
    join)
        # Asks wifid to connect to one of its saved networks, named in hex, as KOReader does when a
        # network is picked (com.lab126.cmd ensureConnection "wifi:<essid>"). A name that is no saved
        # network is refused before any change: nothing here creates a profile. The radio is turned on
        # first if it is off. Whether it connects is for the caller to watch (current).
        hex=${2:-}
        case "$hex" in ''|*[!0-9a-f]*) exit 2;; esac
        [ $((${#hex} % 2)) = 0 ] && [ "${#hex}" -le 64 ] || exit 2
        read_radio || exit 1
        # A supplicant-owned EAP/WPA3 profile wins even when wifid has a profile with the same name.
        supp_id=
        if [ -f "$supplicant_profiles" ]; then
            essid=$(hex_decode "$hex")
            [ -n "$essid" ] && supp_id=$(supplicant_id "$essid")
        fi
        if [ -z "$supp_id" ]; then
            essid=$(essids profileData "$hex")
            if [ -n "$essid" ]; then
                radio_on || exit 1
                rm -f "$supplicant_marker" 2>/dev/null || true
                lipc-set-prop -s com.lab126.cmd ensureConnection "wifi:$essid" >/dev/null 2>&1 || exit 1
                report_retry || exit 1
                exit 0
            fi
            exit 3
        fi
        sid=$supp_id
        unset supp_id
        radio_on || exit 1
        cm_mode manual
        if ! printf 'enable_network %s\nselect_network %s\n' "$sid" "$sid" | wpa_input 2; then
            unset essid sid
            cm_mode auto
            exit 27
        fi
        printf 'save_config\n' | wpa_input 1 || true
        : > "$supplicant_marker" 2>/dev/null || true
        ( dhcp_later ) </dev/null >/dev/null 2>&1 &
        unset essid sid
        report_retry || { rm -f "$supplicant_marker" 2>/dev/null || true; cm_mode auto; exit 20; }
        current
        saved_all
        exit 0
        ;;
    *) exit 2;;
esac
