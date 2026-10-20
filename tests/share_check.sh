#!/bin/sh
# Runs the headless app on the Share screen against an existing data root and checks every route,
# then once more in the Classic style for the page fonts.
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
kill $APP 2>/dev/null || true
wait $APP 2>/dev/null || true
# Offline: the studio's picture from the layers fetched above against ImageMagick's.
if command -v node >/dev/null 2>&1 && command -v magick >/dev/null 2>&1; then
    check "studio picture = ImageMagick's" "node tests/studio_check.js $B/ink.png $B/paper.png $B/studio '$INKBOX' >$B/studio.log 2>&1"
else
    echo "skip studio vs ImageMagick (needs node and magick)"
fi
[ $fail = 0 ] && echo "SHARE CHECK PASSED" || { echo "SHARE CHECK FAILED"; exit 1; }
