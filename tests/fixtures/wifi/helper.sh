#!/bin/sh
# Scenario-only radio and saved networks, private to the synthetic data root (kindle/app/wifi.sh's
# verbs and report). Nine saved networks, two pages of Settings > Wi-Fi: Home, Studio 5 GHz, Café Lumière,
# 猫の家, a name of 32 bytes, Library guest, Phone hotspot, Train Wi-Fi and Far away, which refuses a switch.
# Like wifid perhaps, it lists them only while the radio is on; on, it is connected to Home until a switch.
set -eu
file=$GRIMOIRE_ROOT/wifi-state
cur=$GRIMOIRE_ROOT/wifi-current
state=$(cat "$file" 2>/dev/null || echo 0)
saved='486f6d65 53747564696f20352047487a 436166c3a9204c756d69c3a87265 e78cabe381aee5aeb6
5570737461697273206f666669636520657874656e6465722035472032303234 4c696272617279206775657374
50686f6e6520686f7473706f74 547261696e2057692d4669 4661722061776179'
case "$1" in
    on) state=1; echo "$state" > "$file";;
    off) state=0; echo "$state" > "$file";;
    status|current|networks) ;;
    join)
        case " $(echo $saved) " in *" ${2:-x} "*) ;; *) exit 3;; esac
        [ "$2" != 4661722061776179 ] || exit 1
        state=1; echo "$state" > "$file"; echo "$2" > "$cur";;
    *) exit 1;;
esac
printf 'wireless=%s\nenabled=%s\nconnected=%s\n' "$state" "$state" "$state"
case "$1" in
    current|networks)
        if [ "$state" = 1 ]; then printf 'current=%s\n' "$(cat "$cur" 2>/dev/null || echo 486f6d65)"; else echo 'current='; fi;;
esac
if [ "$1" = networks ]; then
    if [ "$state" = 1 ]; then for h in $saved; do echo "saved=$h"; done; echo 'list=1'; else echo 'list=0'; fi
fi
