#!/bin/sh
# Runs the headless app on the Share screen against an existing data root and checks every route,
# then once more in the Classic style for the page fonts. Then the screen itself: its QR code read
# back (Apple's Vision, on macOS), the server across turns and a wake, Invert, made-up addresses.
# usage: tests/share_check.sh <grimoire-root>
# GRIMOIRE_TEST_PORT (default 8765) moves the server, so checks in several worktrees can run at once.
set -eu
ROOT=$1
PORT=${GRIMOIRE_TEST_PORT:-8765}
B=build/share-check
rm -rf "$B" && mkdir -p "$B"
# The app reads its port from the data root's settings (the root is a scratch copy of a scenario).
if [ -f "$ROOT/settings.ini" ]; then
    grep -v '^http_port=' "$ROOT/settings.ini" > "$B/settings.ini" || true
    mv "$B/settings.ini" "$ROOT/settings.ini"
fi
echo "http_port=$PORT" >> "$ROOT/settings.ini"
# Renders slowed down a little (as on the Kindle) so concurrent requests really queue.
GRIMOIRE_HTTPD_DELAY_MS=150 GRIMOIRE_ROOT="$ROOT" GRIMOIRE_SCRIPT=tests/scenarios/c_share.qs \
    GRIMOIRE_SHOTS=build/shots/c_share build/host/grimoire-headless 2>"$B/app.log" &
APP=$!
URL=http://127.0.0.1:$PORT
up() { for i in 1 2 3 4 5 6 7 8 9 10; do curl -fsS -o /dev/null "$URL/" 2>/dev/null && break; sleep 0.5; done; }
up
fail=0
check() { if eval "$2"; then echo "ok   $1"; else echo "FAIL $1"; fail=1; fi; }
# Waits (up to 20 s) for the app's log $1 to show a line containing $2: the runner logs every shot
# it takes, so the checks can keep step with a scenario.
await() {
    i=0
    while [ $i -lt 200 ]; do
        grep -q "$2" "$1" 2>/dev/null && return 0
        sleep 0.1
        i=$((i + 1))
    done
    echo "FAIL waiting for '$2' in $1"
    fail=1
}
# A QR decoder for the Share screen's code: Apple's Vision (tools/qrdecode.swift), macOS only.
QRDEC=
if [ "$(uname)" = Darwin ] && command -v swiftc >/dev/null 2>&1; then
    [ build/host/qrdecode -nt tools/qrdecode.swift ] ||
        swiftc -O tools/qrdecode.swift -o build/host/qrdecode >"$B/qrdecode-build.log" 2>&1 || true
    check "QR decoder builds (swiftc)" "[ build/host/qrdecode -nt tools/qrdecode.swift ]"
    QRDEC=build/host/qrdecode
fi
# The gray of a screenshot's top-left pixel: white, or black when the colors are inverted (the first
# pixel of the first row needs no PNG unfiltering).
corner() {
    python3 -I -c "
import struct, sys, zlib
d = open(sys.argv[1], 'rb').read()
p, z = 8, b''
while p < len(d):
    n, t = struct.unpack('>I4s', d[p:p + 8])
    z += d[p + 8:p + 8 + n] if t == b'IDAT' else b''
    p += n + 12
print(zlib.decompressobj().decompress(z, 2)[1])" "$1" 2>/dev/null || echo none
}
curl -fsS "$URL/" -o "$B/index.html"
check "index html" "grep -q '<h1>Grimoire</h1>' $B/index.html"
curl -fsS "$URL/api/list.json" -o "$B/list.json"
check "list.json parses" "python3 -I -m json.tool $B/list.json >/dev/null"
ID=$(python3 -I -c "import json;print(json.load(open('$B/list.json'))['notebooks'][0]['id'])")
NAME=$(python3 -I -c "import json;print(json.load(open('$B/list.json'))['notebooks'][0]['name'])")
curl -fsS "$URL/nb/$ID" -o "$B/nb.html"
check "notebook html" "grep -q '/nb/$ID/1.png' $B/nb.html"
curl -fsS -D "$B/pdf.h" "$URL/nb/$ID.pdf" -o "$B/nb.pdf"
check "pdf magic" "head -c 5 $B/nb.pdf | grep -q '%PDF-'"
check "pdf attachment header" "grep -qi 'content-disposition: attachment' $B/pdf.h"
check "pdf renders (sips)" "sips -s format png $B/nb.pdf --out $B/nb-pdf.png >/dev/null 2>&1"
curl -fsS "$URL/nb/$ID/1.png" -o "$B/p1.png"
check "page png" "file $B/p1.png | grep -q 'PNG image data, 1860 x 2480'"
curl -fsS -D "$B/ink.h" "$URL/nb/$ID/1.png?layer=ink" -o "$B/ink.png"
curl -fsS "$URL/nb/$ID/1.png?layer=paper" -o "$B/paper.png"
INKBOX=$(tr -d '\r' < "$B/ink.h" | sed -n 's/^X-Ink-Box: //p')
check "ink layer headers" "[ -n '$INKBOX' ] && grep -q '^X-Page-Size: 1860x2480' $B/ink.h && grep -q '^X-Paper: ' $B/ink.h"
curl -fsS "$URL/nb/$ID/1.png?layer=paper&f=2" -o "$B/paper-f2.png"
check "shrunk layer (f=2)" "file $B/paper-f2.png | grep -q 'PNG image data, 930 x 1240'"
code=$(curl -s -o /dev/null -w '%{http_code}' "$URL/nb/$ID/1.png?f=9")
check "f out of range 400 ($code)" "[ $code = 400 ]"
NPAGES=$(python3 -I -c "import json;print(json.load(open('$B/list.json'))['notebooks'][0]['pages'])")
check "parallel requests, late senders, idle and quitting connections" "python3 -I tests/share_parallel.py $PORT $ID $NPAGES >$B/parallel.log 2>&1"
check "ink and paper layers" "file $B/ink.png $B/paper.png | grep -c 'PNG image data, 1860 x 2480' | grep -q 2"
check "layers are not the page" "! cmp -s $B/ink.png $B/p1.png && ! cmp -s $B/paper.png $B/p1.png"
code=$(curl -s -o /dev/null -w '%{http_code}' "$URL/nb/$ID/1.png?layer=nope")
check "unknown layer 400 ($code)" "[ $code = 400 ]"
code=$(curl -s -o /dev/null -w '%{http_code}' "$URL/nb/$ID/1.pngx")
check "page png suffix exact ($code)" "[ $code = 404 ]"
curl -fsS "$URL/nb/$ID/studio" -o "$B/studio.html"
check "studio html" "grep -q 'window.GRIMOIRE_NB' $B/studio.html && grep -q 'function imCommand' $B/studio.html"
check "notebook page links the studio" "grep -q '/nb/$ID/studio' $B/nb.html"
check "gothic page fonts" "grep -q '/font/jacquard24.ttf' $B/index.html && grep -q '/font/jersey10.ttf' $B/index.html"
curl -fsS -D "$B/font.h" "$URL/font/jersey10.ttf?v=1" -o "$B/jersey10.ttf"
check "font = the built-in copy" "cmp -s $B/jersey10.ttf assets/Jersey10-Regular.ttf"
check "font cached by the browser" "grep -qi 'cache-control: public, max-age' $B/font.h"
code=$(curl -s -o /dev/null -w '%{http_code}' "$URL/font/nope.ttf")
check "unknown font 404 ($code)" "[ $code = 404 ]"
curl -fsS "$URL/nb/$ID/thumb.png" -o "$B/thumb.png"
check "thumb png" "file $B/thumb.png | grep -q 'PNG image data'"
curl -fsS "$URL/nb/$ID.zip" -o "$B/nb.zip"
check "notebook zip valid" "unzip -tq $B/nb.zip >/dev/null"
check "notebook zip has pngs + raw" "unzip -l $B/nb.zip | grep -q page-001.png && unzip -l $B/nb.zip | grep -q raw/notebook.ini"
curl -fsS "$URL/all.zip" -o "$B/all.zip"
check "all.zip valid" "unzip -tq $B/all.zip >/dev/null"
FOLDER=$(python3 -I -c "import json;f=json.load(open('$B/list.json'))['folders'];print(f[0] if f else '')")
if [ -n "$FOLDER" ]; then
    curl -fsS "$URL/folder/$(python3 -I -c "import urllib.parse,sys;print(urllib.parse.quote(sys.argv[1]))" "$FOLDER").zip" -o "$B/folder.zip"
    check "folder zip valid" "unzip -tq $B/folder.zip >/dev/null"
fi
code=$(curl -s -o /dev/null -w '%{http_code}' "$URL/nb/../../etc/passwd")
check "path traversal rejected ($code)" "[ $code = 400 ] || [ $code = 404 ]"
code=$(curl -s -o /dev/null -w '%{http_code}' "$URL/nb/nope.pdf")
check "missing notebook 404" "[ $code = 404 ]"
code=$(curl -s -o /dev/null -w '%{http_code}' -X POST "$URL/")
check "POST rejected ($code)" "[ $code = 405 ]"
code=$(curl -s -o /dev/null -w '%{http_code}' -H 'Host: evil.example' "$URL/api/list.json")
check "foreign Host refused ($code)" "[ $code = 400 ]"
GRIMOIRE_OUT="$B/pulled" tools/grimoire-pull --http "127.0.0.1:$PORT" --png >"$B/pull.log" 2>&1
check "grimoire-pull --http" "ls $B/pulled/*.pdf >/dev/null 2>&1 || ls $B/pulled/*/*.pdf >/dev/null 2>&1"
# The QR code, read back from the screenshot: it holds exactly the address in big type (the one
# the app logs), and that address answers like the Kindle's would for a phone. A machine with no
# Wi-Fi or Ethernet address gets no code (as a Kindle over USBNetwork only): nothing to read.
if [ -z "$QRDEC" ]; then
    echo "skip QR code read back (needs macOS and swiftc)"
elif ! grep -q 'share: QR code for ' "$B/app.log"; then
    echo "skip QR code read back (no Wi-Fi or Ethernet address here, so the screen shows none)"
else
    await "$B/app.log" 01-share
    QR=$($QRDEC build/shots/c_share/01-share.png 2>/dev/null | head -n 1)
    check "QR code reads back as the address shown ($QR)" \
        "echo '$QR' | grep -Eq '^http://[0-9.]+:[0-9]+\$' && grep -qF 'share: QR code for $QR ' $B/app.log"
    curl -fsS "$QR/" -o "$B/index-qr.html" 2>/dev/null || true
    check "the QR code's address answers" "grep -q '<h1>Grimoire</h1>' $B/index-qr.html"
fi
kill $APP 2>/dev/null || true
wait $APP 2>/dev/null || true
# The Classic style: the pages switch to Terminess.
C=$B/classic-root
rm -rf "$C" && cp -R "$ROOT" "$C"
grep -v '^theme=' "$ROOT/settings.ini" > "$C/settings.ini" && echo theme=classic >> "$C/settings.ini"
GRIMOIRE_ROOT="$C" GRIMOIRE_SCRIPT=tests/scenarios/c_share.qs GRIMOIRE_SHOTS=build/shots/c_share_classic \
    build/host/grimoire-headless 2>"$B/app-classic.log" &
APP=$!
up
curl -fsS "$URL/" -o "$B/index-classic.html"
check "classic page fonts" "grep -q '/font/terminess.ttf' $B/index-classic.html && ! grep -q jacquard24 $B/index-classic.html"
curl -fsS "$URL/font/terminess-bold.ttf" -o "$B/terminess-bold.ttf"
check "classic bold font" "cmp -s $B/terminess-bold.ttf assets/TerminessNerdFontMono-Bold.ttf"
await "$B/app-classic.log" 01-share  # the Classic Share screen's shot, for review
kill $APP 2>/dev/null || true
wait $APP 2>/dev/null || true
# Turned sideways and back, asleep and awake, on the Share screen (scenario q_share_rotate): one
# server for the whole visit, so a download still waiting for its render while the Kindle turns
# arrives too. Done stops the server; the next visit starts it. Renders are slowed down so that of
# three page downloads two render while the third waits in the queue.
T=$B/rotate-root
rm -rf "$T" build/shots/q_share_rotate && cp -R "$ROOT" "$T"
GRIMOIRE_HTTPD_DELAY_MS=2000 GRIMOIRE_ROOT="$T" GRIMOIRE_SCRIPT=tests/scenarios/q_share_rotate.qs \
    GRIMOIRE_SHOTS=build/shots/q_share_rotate build/host/grimoire-headless 2>"$B/app-rotate.log" &
APP=$!
await "$B/app-rotate.log" 01-share-portrait
DL=
for k in 1 2 3; do
    curl -fsS "$URL/nb/$ID/1.png" -o "$B/turn-$k.png" 2>/dev/null &
    DL="$DL $!"
done
await "$B/app-rotate.log" 02-share-landscape
curl -fsS "$URL/api/list.json" -o "$B/turn-list.json" 2>/dev/null || true
check "answers in landscape" "python3 -I -m json.tool $B/turn-list.json >/dev/null 2>&1"
wait $DL 2>/dev/null || true
# (The app logs the turn. Three requests before it means one of them was waiting in the queue when
# the Kindle turned.)
early=$(awk '/share: laid out again/ { n++ } n == 0 && /httpd: GET .*\/1\.png/ { k++ } END { print k + 0 }' "$B/app-rotate.log")
[ "$early" = 3 ] && how="one was waiting in the queue during the turn" || how="this run sent them after the turn"
check "three downloads across the turn arrive ($how)" \
    "[ \$(file $B/turn-1.png $B/turn-2.png $B/turn-3.png | grep -c 'PNG image data, 1860 x 2480') = 3 ]"
await "$B/app-rotate.log" 04-share-after-wake
curl -fsS "$URL/" -o "$B/turn-index.html" 2>/dev/null || true
check "answers after waking" "grep -q '<h1>Grimoire</h1>' $B/turn-index.html"
await "$B/app-rotate.log" 05-library
code=$(curl -s -o /dev/null -w '%{http_code}' "$URL/" 2>/dev/null || true)
check "Done stops the server ($code)" "[ '$code' = 000 ]"
await "$B/app-rotate.log" 06-share-landscape-again
curl -fsS "$URL/" -o "$B/turn-index2.html" 2>/dev/null || true
check "Share opened again answers" "grep -q '<h1>Grimoire</h1>' $B/turn-index2.html"
await "$B/app-rotate.log" exiting
kill $APP 2>/dev/null || true
wait $APP 2>/dev/null || true
check "one server start per visit, none for a turn or a wake" "[ \$(grep -c 'httpd: listening' $B/app-rotate.log) = 2 ]"
if [ -z "$QRDEC" ]; then
    echo "skip landscape QR code read back (needs macOS and swiftc)"
elif ! grep -q 'share: QR code for ' "$B/app-rotate.log"; then
    echo "skip landscape QR code read back (no Wi-Fi or Ethernet address here)"
else
    for s in 02-share-landscape 06-share-landscape-again; do
        got=$($QRDEC build/shots/q_share_rotate/$s.png 2>/dev/null | head -n 1)
        check "landscape QR code reads back ($s: $got)" \
            "[ -n '$got' ] && grep -qF 'share: QR code for $got ' $B/app-rotate.log"
    done
fi
# Invert on (white on black): the code is drawn as its negative, so the panel still shows it dark on
# light, the way every phone camera reads it.
V=$B/invert-root
rm -rf "$V" build/shots/q_share_invert && cp -R "$ROOT" "$V"
grep -v '^invert=' "$ROOT/settings.ini" > "$V/settings.ini" || true
echo invert=1 >> "$V/settings.ini"
GRIMOIRE_ROOT="$V" GRIMOIRE_SCRIPT=tests/scenarios/q_share_rotate.qs GRIMOIRE_SHOTS=build/shots/q_share_invert \
    build/host/grimoire-headless 2>"$B/app-invert.log" &
APP=$!
await "$B/app-invert.log" 02-share-landscape
kill $APP 2>/dev/null || true
wait $APP 2>/dev/null || true
check "Invert: the screen is white on black" "[ \"\$(corner build/shots/q_share_invert/01-share-portrait.png)\" = 0 ]"
if [ -z "$QRDEC" ]; then
    echo "skip Invert QR code read back (needs macOS and swiftc)"
elif ! grep -q 'share: QR code for ' "$B/app-invert.log"; then
    echo "skip Invert QR code read back (no Wi-Fi or Ethernet address here)"
else
    for s in 01-share-portrait 02-share-landscape; do
        got=$($QRDEC --polarity build/shots/q_share_invert/$s.png 2>/dev/null | head -n 1)
        check "Invert: the code reads dark on light ($s: $got)" \
            "[ '${got% *} dark-on-light' = '$got' ] && grep -qF 'share: QR code for ${got% *} ' $B/app-invert.log"
    done
fi
# Made-up address lists (the simulator's GRIMOIRE_SIM_ADDRS): one, two and three addresses,
# USBNetwork alone and none, in both styles and both orientations. The code holds the Wi-Fi address
# whatever order the system lists the interfaces in, the others get a line each, over USBNetwork
# alone there is no code, and with no address at all the screen asks for Wi-Fi. Shots for review
# in build/shots/share_layouts/<style>-<case>/.
L=build/shots/share_layouts
rm -rf "$L" && mkdir -p "$L"
printf 'wait 300\ntap 1345 95\nwait 400\nshot portrait\nrotate 1\nwait 400\nshot landscape\nquit\n' > "$B/layout.qs"
for theme in gothic classic; do
    for spec in "one|wlan0=192.168.100.200" \
                "two|wlan0=192.168.100.200,usb0=192.168.15.244" \
                "three|usb0=192.168.15.244,eth0=10.0.0.5,wlan0=192.168.100.200" \
                "usb-only|usb0=192.168.15.244" \
                "none|"; do
        name=${spec%%|*}; addrs=${spec#*|}
        D=$B/layout-$theme-$name
        rm -rf "$D" && mkdir -p "$D"
        printf 'theme=%s\nhttp_port=%s\n' "$theme" "$PORT" > "$D/settings.ini"
        GRIMOIRE_SIM_ADDRS=$addrs GRIMOIRE_ROOT="$D" GRIMOIRE_SCRIPT="$B/layout.qs" GRIMOIRE_SHOTS="$L/$theme-$name" \
            build/host/grimoire-headless 2>"$D.log" || true
        wifi=$(echo "$addrs" | tr ',' '\n' | sed -n 's/^wlan[0-9]*=//p' | head -n 1)
        if [ -n "$wifi" ]; then
            check "$theme $name: the code is the Wi-Fi address" "grep -qF 'share: QR code for http://$wifi:$PORT (wlan' $D.log"
        elif [ -n "$addrs" ]; then
            check "$theme $name: no code" "grep -q 'share: no Wi-Fi address' $D.log && ! grep -q 'share: QR code' $D.log"
        else
            check "$theme $name: no address, no code" "grep -q 'httpd: listening' $D.log && ! grep -q 'share: QR code' $D.log"
        fi
        [ -n "$QRDEC" ] || continue
        for o in portrait landscape; do
            got=$($QRDEC "$L/$theme-$name/$o.png" 2>/dev/null | head -n 1)
            check "$theme $name $o: reads back '$got'" "[ '$got' = '${wifi:+http://$wifi:$PORT}' ]"
        done
    done
done
[ -n "$QRDEC" ] || echo "skip made-up addresses' QR codes read back (needs macOS and swiftc)"
# share.c checks its own layout as it draws: a "share: layout:" line means a line was cut short to
# fit its column (the address in big type included, the very string in the code), or a part of the
# screen (the code, the text, the counter, a button) left the card or ran into another. There must
# be none in any run above, in either style or orientation. (Each log must exist: grep exits 2.)
no_layout_lines() { grep -H 'share: layout:' "$@"; [ $? = 1 ]; }
check "the Share screen lays out clear in every run above" \
    "no_layout_lines $B/app.log $B/app-classic.log $B/app-rotate.log $B/app-invert.log $B/layout-*.log"
# Offline: the studio's picture from the layers fetched above against ImageMagick's.
if command -v node >/dev/null 2>&1 && command -v magick >/dev/null 2>&1; then
    check "studio picture = ImageMagick's" "node tests/studio_check.js $B/ink.png $B/paper.png $B/studio '$INKBOX' >$B/studio.log 2>&1"
else
    echo "skip studio vs ImageMagick (needs node and magick)"
fi
[ $fail = 0 ] && echo "SHARE CHECK PASSED" || { echo "SHARE CHECK FAILED"; exit 1; }
