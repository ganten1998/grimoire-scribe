#!/bin/sh
# The Kindle's own radio controls and its saved networks, matching KOReader's Kindle backend. No
# profile is created, changed or deleted; no password is printed, stored or logged; no services,
# firewall or boot settings are changed. The caller bounds execution in a child process.
set -u
get() { lipc-get-prop "$@" 2>/dev/null; }
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
        cm=$(get -s com.lab126.wifid cmState) || cm=unknown
        case "$cm" in CONNECTED) connected=1;; unknown|'') connected=unknown;; esac
    fi
    printf 'wireless=%s\nenabled=%s\nconnected=%s\n' "$wireless" "$enabled" "$connected"
}
radio_on() {
    [ "$wireless" = 1 ] || lipc-set-prop -i com.lab126.cmd wirelessEnable 1 >/dev/null 2>&1 || return 1
    [ "$enabled" = 1 ] || lipc-set-prop -i com.lab126.wifid enable 1 >/dev/null 2>&1 || return 1
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
# a hex name as $2 it prints instead the one saved name with that hex, as it is, or nothing.
essids() {
    { printf '{}\n' | lipc-hash-prop com.lab126.wifid "$1" 2>/dev/null || echo '#lipc-failed'; } |
        LC_ALL=C awk -v want="${2:-}" '
function take(v,   i, b, h) {
    if (v == "" || length(v) > 32 || index(v, "\\")) return
    h = ""
    for (i = 1; i <= length(v); i++) {
        b = ord[substr(v, i, 1)]
        if (b < 32 || b == 127) return
        h = h sprintf("%02x", b)
    }
    if (want != "") { if (h == want && !picked) { picked = 1; print v } return }
    if (h in seen || n >= 64) return
    seen[h] = 1; out[++n] = h
}
BEGIN { for (i = 1; i < 256; i++) ord[sprintf("%c", i)] = i; n = 0; bad = 0; failed = 0; picked = 0 }
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
        if (match(s, /^-?[0-9]+(\.[0-9]+)?/)) { key = ""; s = substr(s, RLENGTH + 1); continue }
        if (match(s, /^\([A-Za-z0-9_.+-]*\)/)) { key = ""; s = substr(s, RLENGTH + 1); continue }
        if (key != "") { bad = 1; break }
        if (match(s, /^[][{}(),;=:]+/)) { s = substr(s, RLENGTH + 1); continue }
        if (match(s, /^[A-Za-z_][A-Za-z0-9_]*/)) { s = substr(s, RLENGTH + 1); continue }
        bad = 1
    }
}
END {
    if (want != "") exit 0
    if (failed || bad) { print "list=0"; exit 0 }
    for (i = 1; i <= n; i++) print "saved=" out[i]
    print "list=1"
}'
}
# The network connected to, in hex, or nothing (not connected, or not known).
current() {
    if [ "$connected" = 1 ]; then
        printf 'current=%s\n' "$(essids currentEssid | sed -n 's/^saved=//p' | head -n 1)"
    else
        printf 'current=\n'
    fi
}
case "${1:-}" in
    status) report;;
    current) report || exit 1; current;;
    networks) report || exit 1; current; essids profileData;;
    on|off)
        # Refuse an unknown or unavailable backend before making any change.
        read_radio || exit 1
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
        essid=$(essids profileData "$hex")
        [ -n "$essid" ] || exit 3
        radio_on || exit 1
        lipc-set-prop -s com.lab126.cmd ensureConnection "wifi:$essid" >/dev/null 2>&1 || exit 1
        report || exit 1
        ;;
    *) exit 2;;
esac
