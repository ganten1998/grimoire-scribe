#!/bin/sh
# Phase 0 Wi-Fi probe for a Kindle over SSH. Read-only: it never creates, changes or deletes a profile
# and never scans or changes the radio. It prints the shape of wifid's answers with every value masked,
# so an SSID or a password cannot reach a bug report.
#
#   sh tools/kindle-wifi-probe.sh > wifi-probe.txt
#
# Paste wifi-probe.txt into the results table in docs/WIFI-PHASE0.md, or keep it with the issue.
set -u
mask() {
    # Keep punctuation and the hasharray's shape; replace every value with v and every key with x.
    sed -e 's/"[^"]*"/"v"/g' -e 's/[^][{}()=:,;" ]/x/g' | tr -s x
}
prop() {
    lipc-get-prop "$1" "$2" 2>/dev/null || printf 'unavailable'
}
echo "probe=wifid-2.1"
echo "model=$(prop com.lab126.system boardid)"
echo "firmware=$(prop com.lab126.system version)"
echo "wireless_enable=$(prop -i com.lab126.cmd wirelessEnable)"
echo "wifid_enable=$(prop -i com.lab126.wifid enable)"
echo "profile_count=$(prop com.lab126.wifid profileCount)"
echo "scan_state=$(prop com.lab126.wifid scanState)"
echo "wpa_passphrase=$(command -v wpa_passphrase >/dev/null 2>&1 && echo yes || echo no)"
echo "wpa_cli=$(command -v wpa_cli >/dev/null 2>&1 && echo yes || echo no)"
if command -v lipc-probe >/dev/null 2>&1; then
    echo '--- properties (values removed) ---'
    lipc-probe -v com.lab126.wifid 2>&1 |
        grep -E 'createProfile|deleteProfile|profileData|currentEssid|scanList|cmState|cmProfile|scanState|profileCount|enable' |
        sed -e 's/[[:space:]]*\[.*$//'
else
    echo "lipc-probe=unavailable"
fi
if command -v lipc-hash-prop >/dev/null 2>&1; then
    echo '--- currentEssid shape ---'
    { printf '{}\n' | lipc-hash-prop com.lab126.wifid currentEssid 2>&1 || echo unavailable; } | mask
    echo '--- profileData shape (values masked) ---'
    { printf '{}\n' | lipc-hash-prop com.lab126.wifid profileData 2>&1 || echo unavailable; } | mask
    echo '--- scanList shape (values masked) ---'
    { printf '{}\n' | lipc-hash-prop com.lab126.wifid scanList 2>&1 || echo unavailable; } | mask
else
    echo "lipc-hash-prop=unavailable"
fi
