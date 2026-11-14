#!/bin/sh
# Scenario-only radio and saved networks, private to the synthetic data root (kindle/app/wifi.sh's
# verbs and report). Nine saved networks, two pages of Settings > Wi-Fi: Home, Studio 5 GHz, Café Lumière,
# 猫の家, a name of 32 bytes, Library guest, Phone hotspot, Train Wi-Fi and Far away, which refuses a switch.
# Like wifid perhaps, it lists them only while the radio is on; on, it is connected to Home until a switch.
set -eu
file=$GRIMOIRE_ROOT/wifi-state
cur=$GRIMOIRE_ROOT/wifi-current
extra=$GRIMOIRE_ROOT/wifi-extra
state=$(cat "$file" 2>/dev/null || echo 0)
linked=$(cat "$GRIMOIRE_ROOT/wifi-connected" 2>/dev/null || echo "$state")
saved='486f6d65 53747564696f20352047487a 436166c3a9204c756d69c3a87265 e78cabe381aee5aeb6
5570737461697273206f666669636520657874656e6465722035472032303234 4c696272617279206775657374
50686f6e6520686f7473706f74 547261696e2057692d4669 4661722061776179'
case "$1" in
    on) state=1; linked=1; echo "$state" > "$file"; echo "$linked" > "$GRIMOIRE_ROOT/wifi-connected";;
    off) state=0; linked=0; echo "$state" > "$file"; echo "$linked" > "$GRIMOIRE_ROOT/wifi-connected";;
    status|current|networks) ;;
    disconnect) linked=0; echo "$linked" > "$GRIMOIRE_ROOT/wifi-connected";;
    scan)
        if [ "$state" = 1 ]; then
            printf 'scan=ok\n'
            printf 'near=486f6d65,4,5,wpa2,1\n'
            printf 'near=53747564696f20352047487a,4,5,wpa2,1\n'
            printf 'near=436166c3a9204c756d69c3a87265,3,5,wpa2,0\n'
            printf 'near=4e65696768626f72,2,5,wpa3,0\n'
            printf 'near=4f70656e,1,5,open,0\n'
            printf 'near_end=1\n'
        else
            printf 'scan=off\n'
        fi
        ;;
    add)
        # The real helper makes a profile; the scenario's knows the names it is given as hex states.
        hex=$(printf '%s' "${2-}" | od -An -tx1 | tr -d ' \n')
        [ -n "$hex" ] || exit 2
        [ "${3-}" = open ] || cat >/dev/null
        printf '%s\n' "$hex" >> "$extra"
        state=1; linked=1; echo "$state" > "$file"; echo "$linked" > "$GRIMOIRE_ROOT/wifi-connected"; echo "$hex" > "$cur";;
    delete)
        hex=$(printf '%s' "${2-}" | od -An -tx1 | tr -d ' \n')
        [ -n "$hex" ] || exit 2
        if [ -f "$extra" ]; then grep -vx "$hex" "$extra" > "$extra.tmp" || true; mv "$extra.tmp" "$extra"; fi
        [ "$(cat "$cur" 2>/dev/null)" != "$hex" ] || echo 486f6d65 > "$cur";;
    join)
        case " $(echo $saved) " in *" ${2:-x} "*) ;; *) exit 3;; esac
        [ "$2" != 4661722061776179 ] || exit 1
        state=1; linked=1; echo "$state" > "$file"; echo "$linked" > "$GRIMOIRE_ROOT/wifi-connected"; echo "$2" > "$cur";;
    *) exit 1;;
esac
connected=0
[ "$state" = 1 ] && [ "$linked" = 1 ] && connected=1
printf 'wireless=%s\nenabled=%s\nconnected=%s\n' "$state" "$state" "$connected"
case "$1" in
    current|networks|add|delete|disconnect)
        if [ "$connected" = 1 ]; then printf 'current=%s\n' "$(cat "$cur" 2>/dev/null || echo 486f6d65)"; else echo 'current='; fi;;
esac
if [ "$1" = networks ] || [ "$1" = add ] || [ "$1" = delete ]; then
    if [ "$state" = 1 ]; then for h in $saved $(cat "$extra" 2>/dev/null); do echo "saved=$h"; done; echo 'list=1'; else echo 'list=0'; fi
fi
