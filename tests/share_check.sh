#!/bin/sh
# Runs the headless app on the Share screen against an existing data root and checks every route.
# usage: tests/share_check.sh <grimoire-root>
set -eu
ROOT=$1
B=build/share-check
rm -rf "$B" && mkdir -p "$B"
GRIMOIRE_ROOT="$ROOT" GRIMOIRE_SCRIPT=tests/scenarios/c_share.qs GRIMOIRE_SHOTS=build/shots/c_share \
    build/host/grimoire-headless 2>"$B/app.log" &
APP=$!
URL=http://127.0.0.1:8765
for i in 1 2 3 4 5 6 7 8 9 10; do curl -fsS -o /dev/null "$URL/" 2>/dev/null && break; sleep 0.5; done
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
GRIMOIRE_OUT="$B/pulled" tools/grimoire-pull --http 127.0.0.1 --png >"$B/pull.log" 2>&1
check "grimoire-pull --http" "ls $B/pulled/*.pdf >/dev/null 2>&1 || ls $B/pulled/*/*.pdf >/dev/null 2>&1"
kill $APP 2>/dev/null || true
wait $APP 2>/dev/null || true
[ $fail = 0 ] && echo "SHARE CHECK PASSED" || { echo "SHARE CHECK FAILED"; exit 1; }
