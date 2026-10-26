#!/bin/sh
# Pairing a Bluetooth controller, end to end in the simulator: a stand-in for the helper's local API
# (tests/fake_bthelper.py) on a free port, and tests/bt/*.qs run against it (make
# scenarios never sees them: they need the stand-in). Also kindle/app/bluetooth.sh's host tests.
# usage: [BUILD=build] tests/bt_check.sh
set -eu
BUILD=${BUILD:-build}
PORT=${GRIMOIRE_BT_TEST_PORT:-18777}
HEADLESS=$BUILD/host/grimoire-headless
B=$BUILD/bt-check
rm -rf "$B" && mkdir -p "$B"
python3 -I tests/fake_bthelper.py "$PORT" > "$B/helper.log" 2>&1 &
FAKE=$!
trap 'kill "$FAKE" 2>/dev/null' EXIT
fail=0
for s in tests/bt/*.qs; do
    n=$(basename "$s" .qs)
    r=$B/$n
    mkdir -p "$r"
    sed -n 's/^# settings: *//p' "$s" > "$r/settings.ini"
    envs=$(sed -n 's/^# env: *//p' "$s" | tr '\n' ' ')
    # (Unquoted on purpose: "# env:" holds NAME=VALUE words.)
    # shellcheck disable=SC2086
    if env GRIMOIRE_LANG=en $envs GRIMOIRE_BT_API=127.0.0.1:$PORT GRIMOIRE_ROOT=$r GRIMOIRE_SCRIPT=$s \
        GRIMOIRE_SHOTS=$BUILD/shots/$n "$HEADLESS" > "$B/$n.log" 2>&1; then
        echo "ok   $n"
    else
        echo "FAIL $n (log: $B/$n.log)"
        grep -E 'expect failed|^fit|cut short' "$B/$n.log" | head -5
        fail=1
    fi
done
if python3 -I tests/test_bluetooth_sh.py > "$B/script.log" 2>&1; then echo "ok   kindle/app/bluetooth.sh"
else cat "$B/script.log"; fail=1; fi
if [ $fail -ne 0 ]; then
    echo "BT CHECK FAILED"
    exit 1
fi
echo "BT CHECK PASSED"
