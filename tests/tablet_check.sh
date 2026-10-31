#!/bin/sh
# tools/scribe-tablet.c, the Kindle Scribe's pen as a Mac tablet (docs/TABLET.md): builds it, replays
# the synthetic streams of tests/tablet/make_capture.py through --dry-run and compares the events it
# would post with tests/tablet/expect-*.txt (worked out by hand: each case's comment gives its
# arithmetic). One case again through a pipe in 13-byte writes, so events arrive split across
# reads. Then bad options, and the per-event cost (--bench). macOS only; not part of make check.
# usage: [BUILD=build] tests/tablet_check.sh
set -eu
BUILD=${BUILD:-build}
make --no-print-directory -s BUILD="$BUILD" tablet
T=$BUILD/host/scribe-tablet
B=$BUILD/tablet-check
rm -rf "$B" && mkdir -p "$B"
python3 -I tests/tablet/make_capture.py raw "$B/raw.bin"
python3 -I tests/tablet/make_capture.py scaled "$B/scaled.bin"
failed=
check() { # name, then the tool's options after --replay
    name=$1
    shift
    if ! "$T" --dry-run --replay "$@" > "$B/$name.txt" 2> "$B/$name.err"; then
        cat "$B/$name.err"
        failed="$failed $name"
    elif ! diff -u "tests/tablet/expect-$name.txt" "$B/$name.txt"; then
        failed="$failed $name"
    fi
}
# A panel of 156.24 x 208.32 mm makes a raw unit 0.01 mm. A 160 x 100 mm area centred on the panel
# onto 1601 x 1001 points (spans of 1600 and 1000): x = (raw x / 100 + 1.88) * 10,
# y = (raw y / 100 - 54.16) * 10.
MM="--panel-mm 156.24,208.32 --area 160,100,78.12,104.16 --screen 0,0,1601,1001"
# shellcheck disable=SC2086
check mm "$B/raw.bin" $MM
# Turned a quarter clockwise: held, the panel is 208.32 x 156.24 mm; a 200 x 125 mm area, centred:
# x = (208.32 - raw y / 100 - 4.16) * 8, y = (raw x / 100 - 15.62) * 8. Touch at pressure 1000, up
# below 750; only the eraser makes the right button.
check rotated-pressure "$B/raw.bin" --rotate 90 --panel-mm 156.24,208.32 --area 200,125 \
    --screen 0,0,1601,1001 --pressure 1000 --right eraser
# shellcheck disable=SC2086
check hover-only "$B/raw.bin" --hover-only $MM
# The scaled pen: 186 x 248 mm makes a unit 0.1 mm; x = (raw x / 10 - 13) * 10, y = (raw y / 10 - 74) * 10.
check scaled "$B/scaled.bin" --device stylus-custom --panel-mm 186,248 --area 160,100,93,124 --screen 0,0,1601,1001
# The defaults: the 2022/2024 panel (1860 x 2480 px at 300 dpi), the largest area of the screen's shape.
check default-area "$B/raw.bin" --screen 0,0,1600,1000
# Events split across reads give the same.
# shellcheck disable=SC2086
python3 -I -c 'import os, sys, time
d = open(sys.argv[1], "rb").read()
for i in range(0, len(d), 13):
    os.write(1, d[i:i + 13])
    time.sleep(0.0005)' "$B/raw.bin" | "$T" --dry-run --replay /dev/stdin $MM > "$B/split.txt" 2> "$B/split.err"
if ! diff -u tests/tablet/expect-mm.txt "$B/split.txt"; then failed="$failed split"; fi
# Live, with tests/tablet/ssh standing in for ssh: the remote command finds the device by name and
# streams the capture; at its end the button is let go and the tool connects again after 1 s.
F=$B/fake
mkdir -p "$F/input"
cp "$B/raw.bin" "$F/input/event3"
cp "$B/scaled.bin" "$F/input/event6"
cat > "$F/devices" <<'DEVICES'
I: Bus=0018 Vendor=056a Product=0000 Version=0000
N: Name="WacomDigitizer Pen"
H: Handlers=event2

I: Bus=0018 Vendor=056a Product=0000 Version=0000
N: Name="WacomDigitizer"
P: Phys=
H: Handlers=mouse0 event3
B: EV=b

I: Bus=0000 Vendor=0000 Product=0000 Version=0000
N: Name="stylus-custom"
H: Handlers=event6
DEVICES
export TABLET_FAKE="$F"
FAKE_PATH="$(pwd)/tests/tablet:$PATH"
# shellcheck disable=SC2086
PATH=$FAKE_PATH "$T" --host kindle.test --bind 10.0.0.2 --dry-run $MM -- -F /dev/null > "$B/live.txt" 2> "$B/live.err" &
LIVE=$!
i=0
while [ "$(wc -l < "$B/live.txt")" -lt 66 ] && [ $i -lt 100 ]; do sleep 0.1; i=$((i + 1)); done
kill -TERM $LIVE
wait $LIVE || true
cat tests/tablet/expect-mm.txt tests/tablet/expect-mm.txt > "$B/live-expect.txt"
if ! diff -u "$B/live-expect.txt" "$B/live.txt"; then cat "$B/live.err"; failed="$failed live"; fi
grep -q "event3 (WacomDigitizer)" "$B/live.err" || { cat "$B/live.err"; failed="$failed live-device"; }
grep -q "again in 1 s" "$B/live.err" || { cat "$B/live.err"; failed="$failed live-reconnect"; }
# ssh's arguments: the caller's first (ssh keeps the first value of an option), then the tool's.
tr '\n' ' ' < "$F/args" > "$B/args.txt"
case $(cat "$B/args.txt") in
"-F /dev/null -T -x -a -o BatchMode=yes -o ConnectTimeout=5 -o ServerAliveInterval=2 -o ServerAliveCountMax=3 -o Compression=no -o IPQoS=ef -p 2222 -l root -b 10.0.0.2 kindle.test n="*) ;;
*) echo "ssh's arguments: $(cat "$B/args.txt")"; failed="$failed ssh-args" ;;
esac
# A node by number, and a name the Kindle does not have (no retrying: status 1 at once).
PATH=$FAKE_PATH "$T" --host k --device event6 --range 1860,2480 --dry-run --panel-mm 186,248 --area 160,100,93,124 \
    --screen 0,0,1601,1001 > "$B/node.txt" 2> "$B/node.err" &
LIVE=$!
i=0
while [ "$(wc -l < "$B/node.txt")" -lt 5 ] && [ $i -lt 100 ]; do sleep 0.1; i=$((i + 1)); done
kill -TERM $LIVE
wait $LIVE || true
if ! diff -u tests/tablet/expect-scaled.txt "$B/node.txt"; then cat "$B/node.err"; failed="$failed node"; fi
if PATH=$FAKE_PATH "$T" --host k --device Nothing --range 10,10 --dry-run --screen 0,0,100,100 > /dev/null 2> "$B/none.err"; then
    status=0
else
    status=$?
fi
if [ "$status" != 1 ] || ! grep -q "no input device named Nothing" "$B/none.err"; then
    cat "$B/none.err"
    failed="$failed no-device"
fi
# Bad options are refused with status 2.
for args in "--host k --rotate 45" "--host k --area 0" "--host k --area 10,20,30" "--host k --pressure x" \
    "--host k --right middle" "--host k --device it's" "--host k --device other" "--host k stray" "--port 2222"; do
    # shellcheck disable=SC2086
    if "$T" $args > /dev/null 2>&1; then status=0; else status=$?; fi
    if [ "$status" != 2 ]; then
        echo "scribe-tablet $args: status $status, not 2"
        failed="$failed options"
    fi
done
"$T" --help > /dev/null || failed="$failed help"
"$T" --bench 20000000 || failed="$failed bench"
if [ -n "$failed" ]; then
    echo "TABLET CHECK FAILED:$failed"
    exit 1
fi
echo "tablet-check: every replay matches"
