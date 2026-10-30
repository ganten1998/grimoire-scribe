#!/bin/sh
# The Kindle's own radio controls, matching KOReader's Kindle backend. No profiles, credentials,
# services, firewall or boot settings are changed. The caller bounds execution in a child process.
set -u
get() { lipc-get-prop "$@" 2>/dev/null; }
read_radio() {
    wireless=$(get -i com.lab126.cmd wirelessEnable) || return 1
    enabled=$(get -i com.lab126.wifid enable) || return 1
    case "$wireless:$enabled" in 0:0|0:1|1:0|1:1) ;; *) return 1;; esac
}
report() {
    read_radio || return 1
    cm=$(get -s com.lab126.wifid cmState) || cm=unknown
    connected=0
    case "$cm" in CONNECTED) connected=1;; unknown|'') connected=unknown;; esac
    printf 'wireless=%s\nenabled=%s\nconnected=%s\n' "$wireless" "$enabled" "$connected"
}
case "${1:-}" in
    status) report;;
    on|off)
        # Refuse an unknown or unavailable backend before making any change.
        read_radio || exit 1
        if [ "$1" = on ]; then
            [ "$wireless" = 1 ] || lipc-set-prop -i com.lab126.cmd wirelessEnable 1 >/dev/null 2>&1 || exit 1
            [ "$enabled" = 1 ] || lipc-set-prop -i com.lab126.wifid enable 1 >/dev/null 2>&1 || exit 1
        else
            [ "$enabled" = 0 ] || lipc-set-prop -i com.lab126.wifid enable 0 >/dev/null 2>&1 || exit 1
            [ "$wireless" = 0 ] || lipc-set-prop -i com.lab126.cmd wirelessEnable 0 >/dev/null 2>&1 || exit 1
        fi
        report || exit 1
        if [ "$1" = on ]; then [ "$wireless:$enabled" = 1:1 ]; else [ "$wireless:$enabled" = 0:0 ]; fi
        ;;
    *) exit 2;;
esac
