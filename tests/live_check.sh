#!/bin/sh
# Live view end to end: the headless app opens Share, turns Live view on and holds its pairing card;
# tests/live_stream.py then pairs as a browser would and reads the stream. Last, the app stops: no
# viewer may be left running, and it must not wait for one at exit.
# usage: [BUILD=build] tests/live_check.sh
# GRIMOIRE_TEST_PORT (default 8765) moves the server, so checks in several worktrees can run at once.
set -eu
PORT=${GRIMOIRE_TEST_PORT:-8765}
BUILD=${BUILD:-build}
unset GRIMOIRE_HOST GRIMOIRE_PASSCODE GRIMOIRE_HTTPD_DELAY_MS GRIMOIRE_SIM_PRESSURE GRIMOIRE_SIM_STATS \
    GRIMOIRE_SIM_SIZE GRIMOIRE_BG_DELAY_MS GRIMOIRE_CHECK_FIT GRIMOIRE_L10N_DIR GRIMOIRE_SLEEP_MINUTE_MS
GRIMOIRE_LANG=en
GRIMOIRE_SIM_ADDRS="en0=127.0.0.1"
export GRIMOIRE_LANG GRIMOIRE_SIM_ADDRS
B=$BUILD/live-check
HEADLESS=$BUILD/host/grimoire-headless
rm -rf "$B" && mkdir -p "$B/root"
cp -R tests/fixtures/index/. "$B/root/"
grep -v -e '^http_port=' -e '^invert=' "$B/root/settings.ini" > "$B/settings.ini" 2>/dev/null || true
# (Grimoire's colors inverted: the stream says so, for the browser to follow; its frame never is.)
printf 'http_port=%s\ninvert=1\n' "$PORT" >> "$B/settings.ini"
mv "$B/settings.ini" "$B/root/settings.ini"
# The pairing card held open: everything after it is the browser's.
sed -n "/^# live_check holds here/q;p" tests/scenarios/live_1_card.qs > "$B/card.qs"
# The pen left hovering over the card: the stream tells it after its first batch (live_stream.py).
printf 'pen hover 900 1500\nhold\n' >> "$B/card.qs"
APP=
trap 'if [ -n "$APP" ]; then kill "$APP" 2>/dev/null; fi' EXIT
GRIMOIRE_ROOT="$B/root" GRIMOIRE_SCRIPT="$B/card.qs" GRIMOIRE_SHOTS="$B/shots" "$HEADLESS" 2>"$B/app.log" &
APP=$!
for i in 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20; do
    curl -fsS -o /dev/null "http://127.0.0.1:$PORT/live" 2>/dev/null && break
    sleep 0.5
done
fail=0
python3 -I tests/live_stream.py "$PORT" "$B/app.log" "$B" || fail=1
# The browser's frame reader on the same stream (node 18+; skipped without it).
if command -v node >/dev/null 2>&1; then
    if node tests/live_check.js "$B/stream.bin" "$B/stream.txt" "$B/pen.txt" >"$B/node.log" 2>&1; then
        echo "ok   web/live.js reads the stream, cut anywhere; its pen, recording and film math"
    else
        cat "$B/node.log"
        fail=1
    fi
else
    echo "skip web/live.js's frame reader (no node)"
fi
# Stopping the app stops its viewers, at once.
t0=$(date +%s)
kill "$APP" 2>/dev/null || true
wait "$APP" 2>/dev/null || true
APP=
t1=$(date +%s)
if [ $((t1 - t0)) -le 5 ]; then echo "ok   the app stopped at once ($((t1 - t0)) s)"; else echo "FAIL the app took $((t1 - t0)) s to stop"; fail=1; fi
# The viewers end with the app (they watch for their parent going): none is left after 3 s.
sleep 3
left=0
for pid in $(sed -n 's/.*live: (simulator) viewer pid \([0-9]*\).*/\1/p' "$B/app.log"); do
    if kill -0 "$pid" 2>/dev/null; then left=$((left + 1)); kill "$pid" 2>/dev/null || true; fi
done
pids=$(grep -c 'live: (simulator) viewer pid' "$B/app.log" || true)
if [ "$pids" -ge 2 ] && [ "$left" = 0 ]; then echo "ok   no viewer outlived the app ($pids viewers)"
else echo "FAIL $left of $pids viewer(s) outlived the app"; fail=1; fi
if [ $fail -ne 0 ]; then echo "LIVE CHECK FAILED (app log: $B/app.log)"; exit 1; fi
echo "LIVE CHECK PASSED"
