#!/bin/sh
# Runs the headless app on the Share screen against an existing data root and checks every route,
# then once more in the Classic style for the page fonts, once in Spanish, and once from a catalog
# of markup (every translation must reach the pages as text). Then the screen itself: its QR code
# read back (Apple's Vision, on macOS), the server across turns and a wake, Invert, made-up
# addresses. Last, the studio: its picture against ImageMagick's, and its Copy command button.
# usage: [BUILD=build] tests/share_check.sh <grimoire-root>
# GRIMOIRE_TEST_PORT (default 8765) moves the server, so checks in several worktrees can run at once.
# BUILD (default build) is the Makefile's build directory: the app, the decoder and the shots.
set -eu
ROOT=$1
PORT=${GRIMOIRE_TEST_PORT:-8765}
BUILD=${BUILD:-build}
# The app's test hooks come from each run below, never from the caller's environment; the runs
# check English text, so they are in English whatever the caller's LANG says, unless a run below
# names another language (GRIMOIRE_LANG ranks first).
unset GRIMOIRE_HOST GRIMOIRE_PASSCODE GRIMOIRE_SIM_ADDRS GRIMOIRE_HTTPD_DELAY_MS GRIMOIRE_SIM_PRESSURE \
    GRIMOIRE_SIM_STATS GRIMOIRE_SIM_SIZE GRIMOIRE_BG_DELAY_MS GRIMOIRE_CHECK_FIT GRIMOIRE_L10N_DIR \
    GRIMOIRE_SLEEP_MINUTE_MS
GRIMOIRE_LANG=en
export GRIMOIRE_LANG
B=$BUILD/share-check
HEADLESS=$BUILD/host/grimoire-headless
rm -rf "$B" && mkdir -p "$B"
# An app still running when the script ends (a check failed under set -e) is stopped with it.
APP=
trap 'if [ -n "$APP" ]; then kill "$APP" 2>/dev/null; fi' EXIT
stop_app() {
    kill "$APP" 2>/dev/null || true
    wait "$APP" 2>/dev/null || true
    APP=
}
# Keep the server alive until stop_app: cloud downloads may take longer than 25 seconds.
# The driver's EXIT trap also stops it on failure; no arbitrary lifetime races the requests.
awk '/^shot 02-share-after$/ { print "hold" } { print }' tests/scenarios/c_share.qs >"$B/c_share_long.qs"
# The app reads its port from the data root's settings (the root is a scratch copy of a scenario).
if [ -f "$ROOT/settings.ini" ]; then
    grep -v '^http_port=' "$ROOT/settings.ini" > "$B/settings.ini" || true
    mv "$B/settings.ini" "$ROOT/settings.ini"
fi
echo "http_port=$PORT" >> "$ROOT/settings.ini"
# Renders slowed down a little (as on the Kindle) so concurrent requests really queue.
GRIMOIRE_HTTPD_DELAY_MS=150 GRIMOIRE_ROOT="$ROOT" GRIMOIRE_SCRIPT="$B/c_share_long.qs" \
    GRIMOIRE_SHOTS=$BUILD/shots/c_share "$HEADLESS" 2>"$B/app.log" &
APP=$!
URL=http://127.0.0.1:$PORT
# up LOG: waits for the server to answer, or fails the whole check (its app's log is LOG).
up() {
    for i in 1 2 3 4 5 6 7 8 9 10; do
        curl -fsS -o /dev/null "$URL/" 2>/dev/null && return 0
        sleep 0.5
    done
    echo "FAIL the server on port $PORT never answered (app log: $1)"
    echo "SHARE CHECK FAILED"
    exit 1
}
up "$B/app.log"
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
    [ "$BUILD/host/qrdecode" -nt tools/qrdecode.swift ] ||
        swiftc -O tools/qrdecode.swift -o "$BUILD/host/qrdecode" >"$B/qrdecode-build.log" 2>&1 || true
    check "QR decoder builds (swiftc)" "[ $BUILD/host/qrdecode -nt tools/qrdecode.swift ]"
    QRDEC=$BUILD/host/qrdecode
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
check "index html" "grep -q 'Grimoire</h1>' $B/index.html"
check "index title has the bat and the icons" "grep -q '<h1 class=brand><svg class=bat' $B/index.html && grep -q 'rel=icon type=image/svg+xml' $B/index.html && grep -q 'rel=apple-touch-icon' $B/index.html"
check "index html in English" "grep -q '<html lang=en>' $B/index.html && grep -q ' on your Kindle</p>' $B/index.html"
VERSION_SHOWN=$("$HEADLESS" --version)
check "index footer has the running app's version" "grep -qF 'Grimoire $VERSION_SHOWN</p>' $B/index.html"
curl -fsS "$URL/api/list.json" -o "$B/list.json"
check "list.json parses" "python3 -I -m json.tool $B/list.json >/dev/null"
check "API version matches the footer and running app" "python3 -I -c \"import json; assert json.load(open('$B/list.json'))['version'] == '$VERSION_SHOWN'\""
ID=$(python3 -I -c "import json;print(json.load(open('$B/list.json'))['notebooks'][0]['id'])")
NAME=$(python3 -I -c "import json;print(json.load(open('$B/list.json'))['notebooks'][0]['name'])")
curl -fsS "$URL/nb/$ID" -o "$B/nb.html"
check "notebook html" "grep -q '/nb/$ID/1.png' $B/nb.html"
check "reader defaults to a page field, one image" "grep -q 'id=page-number name=p type=number' $B/nb.html && [ \"\$(grep -o '<img ' $B/nb.html | wc -l | tr -d ' ')\" = 1 ]"
curl -fsS "$URL/nb/$ID?p=999999999" -o "$B/last.html"
LAST=$(python3 -I -c "import json;print(json.load(open('$B/list.json'))['notebooks'][0]['pages'])")
check "reader clamps page entry to the last page" "grep -q 'max=$LAST value=$LAST' $B/last.html && grep -q '/nb/$ID/$LAST.png?f=2' $B/last.html"
curl -fsS "$URL/nb/$ID?p=-9" -o "$B/first.html"
check "reader clamps negative page entry" "grep -q 'value=1' $B/first.html"

curl -fsS -D "$B/pdf.h" "$URL/nb/$ID.pdf" -o "$B/nb.pdf"
check "pdf magic" "head -c 5 $B/nb.pdf | grep -q '%PDF-'"
check "pdf attachment header" "grep -qi 'content-disposition: attachment' $B/pdf.h"
check "pdf renders (sips)" "sips -s format png $B/nb.pdf --out $B/nb-pdf.png >/dev/null 2>&1"
python3 -I tests/download_check.py "$URL" "$ID" "$B/nb.pdf"
node tests/download_ui_check.js
check "PDF progress UI on both pages" "grep -q GRIMOIRE_DOWNLOAD_WORDS $B/index.html && grep -q GRIMOIRE_DOWNLOAD_WORDS $B/nb.html"
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
    # grimoire.log goes to public bug reports: the request is logged without the folder's name.
    check "the log leaves the folder's name out" \
        "grep -qF 'httpd: GET /folder/<name>.zip' $B/app.log && ! grep -qF '/folder/$FOLDER' $B/app.log"
    # However the path is spelled: the same folder with 'folder' percent-encoded.
    curl -fsS "$URL/%66older/$(python3 -I -c "import urllib.parse,sys;print(urllib.parse.quote(sys.argv[1]))" "$FOLDER").zip" -o "$B/folder2.zip"
    check "an encoded folder path is served and logged without its name" \
        "[ \"\$(unzip -Z1 $B/folder.zip)\" = \"\$(unzip -Z1 $B/folder2.zip)\" ] && [ \$(grep -cF 'httpd: GET /folder/<name>.zip' $B/app.log) -ge 2 ] && ! grep -qF '%66older' $B/app.log"
fi
# A request in proxy form names an address in its request line: the log never repeats it.
curl -s -o /dev/null --request-target "http://203.0.113.9:$PORT/api/list.json" "$URL/" || true
check "a proxy-form request leaves its address out of the log" \
    "grep -qF 'httpd: GET (not a path)' $B/app.log && ! grep -qF '203.0.113.9' $B/app.log"
# A folder name as long as a folder's can be (95 bytes; with accents the keyboard gets there at
# about 47 letters): the link to its zip answers too. One byte more cannot be a folder.
LONG=$(python3 -I -c "
import urllib.parse
s = 'Apuntes de armonía y contrapunto, segundo curso de música '
s += 'ñ' * ((95 - len(s.encode())) // 2)
s += 'x' * (95 - len(s.encode()))
print(urllib.parse.quote(s))")
code=$(curl -s -o "$B/long-folder.zip" -w '%{http_code}' "$URL/folder/$LONG.zip")
check "a 95-byte folder name: its zip answers ($code)" "[ $code = 200 ]"
code=$(curl -s -o /dev/null -w '%{http_code}' "$URL/folder/${LONG}y.zip")
check "a 96-byte folder name: 404 ($code)" "[ $code = 404 ]"
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
# the simulator logs; on the Kindle the log names no address), and that address answers like the
# Kindle's would for a phone. A machine with no Wi-Fi or Ethernet address gets no code (as a
# Kindle over USBNetwork only): nothing to read.
if [ -z "$QRDEC" ]; then
    echo "skip QR code read back (needs macOS and swiftc)"
elif ! grep -q 'share: QR code for ' "$B/app.log"; then
    echo "skip QR code read back (no Wi-Fi or Ethernet address here, so the screen shows none)"
else
    await "$B/app.log" 01-share
    QR=$($QRDEC $BUILD/shots/c_share/01-share.png 2>/dev/null | head -n 1)
    check "QR code reads back as the address shown ($QR)" \
        "echo '$QR' | grep -Eq '^http://[0-9.]+:[0-9]+\$' && grep -qF 'share: (simulator) the code holds $QR (' $B/app.log"
    curl -fsS "$QR/" -o "$B/index-qr.html" 2>/dev/null || true
    check "the QR code's address answers" "grep -q 'Grimoire</h1>' $B/index-qr.html"
fi
stop_app
# The Classic style: the pages switch to Terminess.
C=$B/classic-root
rm -rf "$C" && cp -R "$ROOT" "$C"
grep -v '^theme=' "$ROOT/settings.ini" > "$C/settings.ini" && echo theme=classic >> "$C/settings.ini"
GRIMOIRE_ROOT="$C" GRIMOIRE_SCRIPT="$B/c_share_long.qs" GRIMOIRE_SHOTS=$BUILD/shots/c_share_classic \
    "$HEADLESS" 2>"$B/app-classic.log" &
APP=$!
up "$B/app-classic.log"
curl -fsS "$URL/" -o "$B/index-classic.html"
check "classic page fonts" "grep -q '/font/terminess.ttf' $B/index-classic.html && ! grep -q jacquard24 $B/index-classic.html"
curl -fsS "$URL/font/terminess-bold.ttf" -o "$B/terminess-bold.ttf"
check "classic bold font" "cmp -s $B/terminess-bold.ttf assets/TerminessNerdFontMono-Bold.ttf"
await "$B/app-classic.log" 01-share  # the Classic Share screen's shot, for review
stop_app
# In Spanish (as on a Kindle set to Spanish): the pages say so in their lang attribute and speak it,
# Gothic's bold links keep their accents legible (letters past ASCII in spans without the shadow),
# and the studio gets its words as window.GRIMOIRE_L10N while its code stays the same.
E=$B/es-root
rm -rf "$E" && cp -R "$ROOT" "$E"
GRIMOIRE_LANG=es GRIMOIRE_ROOT="$E" GRIMOIRE_SCRIPT="$B/c_share_long.qs" GRIMOIRE_SHOTS=$BUILD/shots/c_share_es \
    "$HEADLESS" 2>"$B/app-es.log" &
APP=$!
up "$B/app-es.log"
curl -fsS "$URL/" -o "$B/index-es.html"
check "index html in Spanish" "grep -q '<html lang=es>' $B/index-es.html && grep -q ' en tu Kindle</p>' $B/index-es.html"
curl -fsS "$URL/nb/$ID" -o "$B/nb-es.html"
check "notebook html in Spanish, accents spared in bold" \
    "grep -q 'todos los cuadernos' $B/nb-es.html && grep -q 'p<span class=na>á</span>ginas en PNG' $B/nb-es.html"
curl -fsS "$URL/nb/$ID/studio" -o "$B/studio-es.html"
check "studio words in Spanish" "grep -q 'window.GRIMOIRE_L10N={' $B/studio-es.html && \
    grep -q '\"Background\":\"Fondo\"' $B/studio-es.html && grep -q 'function imCommand' $B/studio-es.html"
code=$(curl -s -o "$B/missing-es.txt" -w '%{http_code}' "$URL/nb/nope")
check "a missing notebook, in Spanish ($code)" "[ $code = 404 ] && grep -q 'no existe ese cuaderno' $B/missing-es.txt"
await "$B/app-es.log" 01-share  # the Spanish Share screen's shot, for review
stop_app
# A catalog is third-party input: none may add markup to a page. Every Spanish translation with
# tags, quotes and an entity before it (tests/l10n_markup.py, read through the simulator's
# GRIMOIRE_L10N_DIR in place of the built-in catalog): the index, a notebook's page and the studio
# show each one as text, in elements, in attributes (the pages' alt='...') and in the studio's
# script, and never as markup.
X=$B/markup-root
rm -rf "$X" "$B/markup-l10n" && cp -R "$ROOT" "$X" && mkdir -p "$B/markup-l10n"
python3 -I tests/l10n_markup.py make po/es.mo "$B/markup-l10n/es.mo"
# (Every word is long here, so the Library's top bar has its buttons as wide as they grow: the
# middle of Share is at 1220.)
printf 'wait 300\ntap 1220 95\nwait 400\nexpect screen is share\nwait 30000\nquit\n' > "$B/markup.qs"
GRIMOIRE_LANG=es GRIMOIRE_L10N_DIR="$B/markup-l10n" GRIMOIRE_ROOT="$X" GRIMOIRE_SCRIPT="$B/markup.qs" \
    GRIMOIRE_SHOTS="$B/markup-shots" "$HEADLESS" 2>"$B/app-markup.log" &
APP=$!
up "$B/app-markup.log"
check "the markup catalog is the one read" "grep -qF 'i18n: (simulator) the catalog for es comes from' $B/app-markup.log"
curl -fsS "$URL/" -o "$B/index-markup.html"
check "a catalog of markup: the index shows it as text" "python3 -I tests/l10n_markup.py check $B/index-markup.html"
curl -fsS "$URL/nb/$ID" -o "$B/nb-markup.html"
check "a catalog of markup: a notebook's page shows it as text" "python3 -I tests/l10n_markup.py check $B/nb-markup.html"
curl -fsS "$URL/nb/$ID/studio" -o "$B/studio-markup.html"
check "a catalog of markup: the studio shows it as text, in its script too" \
    "python3 -I tests/l10n_markup.py check $B/studio-markup.html --script"
stop_app
# Turned sideways and back, asleep and awake, on the Share screen (scenario q_share_rotate): one
# server for the whole visit, so a download still waiting for its render while the Kindle turns
# arrives too. Done stops the server; the next visit starts it. Renders are slowed down so that of
# three page downloads two render while the third waits in the queue.
T=$B/rotate-root
rm -rf "$T" "$BUILD/shots/q_share_rotate" && cp -R "$ROOT" "$T"
GRIMOIRE_HTTPD_DELAY_MS=2000 GRIMOIRE_ROOT="$T" GRIMOIRE_SCRIPT=tests/scenarios/q_share_rotate.qs \
    GRIMOIRE_SHOTS=$BUILD/shots/q_share_rotate "$HEADLESS" 2>"$B/app-rotate.log" &
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
await "$B/app-rotate.log" 05-share-after-wake
curl -fsS "$URL/" -o "$B/turn-index.html" 2>/dev/null || true
check "answers after waking" "grep -q 'Grimoire</h1>' $B/turn-index.html"
await "$B/app-rotate.log" 06-library
code=$(curl -s -o /dev/null -w '%{http_code}' "$URL/" 2>/dev/null || true)
check "Done stops the server ($code)" "[ '$code' = 000 ]"
await "$B/app-rotate.log" 07-share-landscape-again
curl -fsS "$URL/" -o "$B/turn-index2.html" 2>/dev/null || true
check "Share opened again answers" "grep -q 'Grimoire</h1>' $B/turn-index2.html"
await "$B/app-rotate.log" exiting
stop_app
check "one server start per visit, none for a turn or a wake" "[ \$(grep -c 'httpd: listening' $B/app-rotate.log) = 2 ]"
if [ -z "$QRDEC" ]; then
    echo "skip landscape QR code read back (needs macOS and swiftc)"
elif ! grep -q 'share: QR code for ' "$B/app-rotate.log"; then
    echo "skip landscape QR code read back (no Wi-Fi or Ethernet address here)"
else
    for s in 02-share-landscape 07-share-landscape-again; do
        got=$($QRDEC $BUILD/shots/q_share_rotate/$s.png 2>/dev/null | head -n 1)
        check "landscape QR code reads back ($s: $got)" \
            "[ -n '$got' ] && grep -qF 'share: (simulator) the code holds $got (' $B/app-rotate.log"
    done
fi
# Invert on (white on black): the code is drawn as its negative, so the panel still shows it dark on
# light, the way every phone camera reads it.
V=$B/invert-root
rm -rf "$V" "$BUILD/shots/q_share_invert" && cp -R "$ROOT" "$V"
grep -v '^invert=' "$ROOT/settings.ini" > "$V/settings.ini" || true
echo invert=1 >> "$V/settings.ini"
GRIMOIRE_ROOT="$V" GRIMOIRE_SCRIPT=tests/scenarios/q_share_rotate.qs GRIMOIRE_SHOTS=$BUILD/shots/q_share_invert \
    "$HEADLESS" 2>"$B/app-invert.log" &
APP=$!
await "$B/app-invert.log" 02-share-landscape
stop_app
check "Invert: the screen is white on black" "[ \"\$(corner $BUILD/shots/q_share_invert/01-share-portrait.png)\" = 0 ]"
if [ -z "$QRDEC" ]; then
    echo "skip Invert QR code read back (needs macOS and swiftc)"
elif ! grep -q 'share: QR code for ' "$B/app-invert.log"; then
    echo "skip Invert QR code read back (no Wi-Fi or Ethernet address here)"
else
    for s in 01-share-portrait 02-share-landscape; do
        got=$($QRDEC --polarity $BUILD/shots/q_share_invert/$s.png 2>/dev/null | head -n 1)
        check "Invert: the code reads dark on light ($s: $got)" \
            "[ '${got% *} dark-on-light' = '$got' ] && grep -qF 'share: (simulator) the code holds ${got% *} (' $B/app-invert.log"
    done
fi
# Made-up address lists (the simulator's GRIMOIRE_SIM_ADDRS): one, two and three addresses,
# USBNetwork alone and none, in both styles and both orientations. The code holds the Wi-Fi address
# whatever order the system lists the interfaces in, the others get a line each, over USBNetwork
# alone there is no code, and with no address at all the screen asks for Wi-Fi. Shots for review
# in build/shots/share_layouts/<style>-<case>/. On the Kindle the log names the interface and
# the port only: only the simulator's own "(simulator)" lines may hold an address.
L=$BUILD/shots/share_layouts
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
            "$HEADLESS" 2>"$D.log" || true
        wifi=$(echo "$addrs" | tr ',' '\n' | sed -n 's/^wlan[0-9]*=//p' | head -n 1)
        if [ -n "$wifi" ]; then
            check "$theme $name: the code is the Wi-Fi address" \
                "grep -qF 'share: QR code for the wlan0 address, port $PORT' $D.log && grep -qF 'share: (simulator) the code holds http://$wifi:$PORT (wlan' $D.log"
        elif [ -n "$addrs" ]; then
            check "$theme $name: no code" "grep -qF 'share: no Wi-Fi address, so no QR code; showing the usb0 address, port $PORT' $D.log && ! grep -q 'share: QR code' $D.log"
        else
            check "$theme $name: no address, no code" "grep -q 'httpd: listening' $D.log && ! grep -q 'share: QR code' $D.log"
        fi
        check "$theme $name: the log names no address (but in its simulator lines)" \
            "! grep -v '(simulator)' $D.log | grep -qE '192\\.168\\.|10\\.0\\.0\\.5'"
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
    "no_layout_lines $B/app.log $B/app-classic.log $B/app-es.log $B/app-rotate.log $B/app-invert.log $B/layout-*.log"
# Offline: the studio's picture from the layers fetched above against ImageMagick's, and its Copy
# command button on a page with no clipboard (plain http), where the browser copies and where it
# will not.
if command -v node >/dev/null 2>&1 && command -v magick >/dev/null 2>&1; then
    check "studio picture = ImageMagick's" "node tests/studio_check.js $B/ink.png $B/paper.png $B/studio '$INKBOX' >$B/studio.log 2>&1"
else
    echo "skip studio vs ImageMagick (needs node and magick)"
fi
if command -v node >/dev/null 2>&1; then
    check "studio Copy command, copied or selected" "node tests/studio_copy_check.js >$B/studio-copy.log 2>&1"
else
    echo "skip the studio's Copy command button (needs node)"
fi
[ $fail = 0 ] && echo "SHARE CHECK PASSED" || { echo "SHARE CHECK FAILED"; exit 1; }
