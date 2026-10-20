#!/bin/sh
# Checks the zip people install (make package): its four files; documents/Grimoire.sh, the
# self-unpacking scriptlet (SH_Integration's keys, BUILD_ID, the icon, the Capabilities line, the
# archive); the app inside it (the binary, the launcher, CAPABILITIES, and a license for every
# embedded font and every third-party part of the binary); and that the launchers' test hooks
# fall back to the device's own paths and timings.
# usage: tests/package_check.sh [build/dist/Grimoire.zip]
set -u
cd "$(dirname "$0")/.." || exit 1
ZIP=${1:-build/dist/Grimoire.zip}
B=$(dirname "$ZIP")/../package-check
VERSION=$(cat VERSION)
export LC_ALL=C  # the scriptlet ends in a binary archive
rm -rf "$B" && mkdir -p "$B/zip" "$B/app"
fail=0
check() {  # check NAME COMMAND...: ok when the command succeeds
    name=$1
    shift
    if "$@"; then echo "ok   $name"; else echo "FAIL $name"; fail=1; fi
}
has() { grep -qF -- "$2" "$1" 2>/dev/null; }  # has FILE TEXT
lacks() { ! grep -qF -- "$2" "$1" 2>/dev/null; }
is() { [ "$1" = "$2" ]; }

if ! unzip -Z1 "$ZIP" >"$B/zip.list" 2>/dev/null || ! unzip -q "$ZIP" -d "$B/zip"; then
    echo "FAIL cannot read $ZIP (make package)"
    echo "PACKAGE CHECK FAILED"
    exit 1
fi
RESET="documents/Grimoire - reset pen calibration.sh"
for f in documents/Grimoire.sh "$RESET" README.txt LICENSE.txt; do
    check "zip has $f" grep -qxF -- "$f" "$B/zip.list"
done
grep -vxF -e documents/ -e documents/Grimoire.sh -e "$RESET" -e README.txt -e LICENSE.txt "$B/zip.list" >"$B/extra.list"
check "zip has nothing else" [ ! -s "$B/extra.list" ]
check "LICENSE.txt is the GPL" cmp -s "$B/zip/LICENSE.txt" LICENSE
check "the reset scriptlet is kindle's" cmp -s "$B/zip/$RESET" "kindle/Grimoire - reset pen calibration.sh"
check "README.txt names version $VERSION" has "$B/zip/README.txt" "Grimoire $VERSION"
check "README.txt has no placeholder left" lacks "$B/zip/README.txt" "@VERSION@"

# The scriptlet: a shell script, a marker line, then the app as a tar archive.
S=$B/zip/documents/Grimoire.sh
LINE=$(awk '/^__GRIMOIRE_APP_ARCHIVE__$/ { print NR + 1; exit }' "$S")
check "Grimoire.sh has the archive marker" [ -n "$LINE" ]
[ -n "$LINE" ] || LINE=1
head -n $((LINE - 2)) "$S" >"$B/script.sh"
tail -n +"$LINE" "$S" >"$B/app.tar"
check "the script part parses (sh -n)" sh -n "$B/script.sh"
check "the script part has no CR" lacks "$B/script.sh" "$(printf '\r')"
check "no placeholder left" sh -c '! grep -qE "@ICON@|@BUILD_ID@" "$1"' sh "$B/script.sh"
# SH_Integration reads only the first six lines for the Library entry's keys. They keep the lines
# they had in 1.0.0-beta (2 to 5), so a reader with a shorter window still finds them all, and
# the Capabilities line comes after them.
head -n 6 "$B/script.sh" >"$B/head6"
line() { sed -n "$1p" "$B/head6"; }  # line N: line N of the scriptlet
check "line 2: # Name: Grimoire" is "$(line 2)" '# Name: Grimoire'
check "line 3: # Author:" sh -c 'case "$1" in "# Author: "?*) exit 0 ;; esac; exit 1' sh "$(line 3)"
check "line 4: # Icon: a PNG data URI" sh -c 'case "$1" in "# Icon: data:image/png;base64,iVBORw0KGgo"*) exit 0 ;; esac; exit 1' sh "$(line 4)"
check "line 5: # DontUseFBInk" is "$(line 5)" '# DontUseFBInk'
check "line 6: # Capabilities:" sh -c 'case "$1" in "# Capabilities: "?*) exit 0 ;; esac; exit 1' sh "$(line 6)"
sed -n 's/^# Icon: data:image\/png;base64,//p' "$B/head6" >"$B/icon.b64"
{ base64 -d <"$B/icon.b64" 2>/dev/null || base64 -D <"$B/icon.b64"; } >"$B/icon.png"
check "the icon is assets/icon.png" cmp -s "$B/icon.png" assets/icon.png
CAPS=$(head -n 10 "$B/script.sh" | sed -n 's/^# Capabilities: //p')
check "Capabilities line in the header ($CAPS)" [ -n "$CAPS" ]
for c in guard unpack-only; do
    check "Capabilities include $c" sh -c 'case " $1 " in *" $2 "*) exit 0 ;; esac; exit 1' sh "$CAPS" "$c"
done
ID=$(sed -n 's/^BUILD_ID=//p' "$B/script.sh")
check "BUILD_ID is 12 hex digits ($ID)" sh -c 'printf "%s\n" "$1" | grep -qxE "[0-9a-f]{12}"' sh "$ID"
check "BUILD_ID is the archive's checksum" is "$(shasum "$B/app.tar" | cut -c1-12)" "$ID"

# The app inside.
tar -tf "$B/app.tar" >"$B/app.list" 2>/dev/null && tar -xf "$B/app.tar" -C "$B/app" 2>/dev/null
check "the archive unpacks" [ -s "$B/app.list" ]
for f in grimoire grimoire.sh CAPABILITIES licenses/GPL-3.0.txt; do
    check "payload has $f" [ -s "$B/app/$f" ]
done
check "payload's grimoire.sh is kindle's" cmp -s "$B/app/grimoire.sh" kindle/app/grimoire.sh
check "payload's CAPABILITIES match the header" is "$(cat "$B/app/CAPABILITIES" 2>/dev/null)" "$CAPS"
check "payload has no Mac metadata" sh -c '! grep -qE "(^|/)(\._|\.DS_Store)" "$1"' sh "$B/app.list"
if command -v file >/dev/null 2>&1; then
    check "grimoire is a static ARM binary" sh -c 'file "$1" | grep -q "ARM.*statically linked"' sh "$B/app/grimoire"
fi
for f in assets/*-OFL-1.1.txt; do
    check "payload has licenses/${f##*/}" cmp -s "$f" "$B/app/licenses/${f##*/}"
done
# Each font the binary embeds (FONTS in the Makefile) needs its OFL text: a
# licenses/<family>-OFL-1.1.txt whose family begins the font's file name (Terminess-OFL-1.1.txt
# covers TerminessNerdFontMono-Regular.ttf).
FONTS=$(awk '/^FONTS[ \t]*:=/ { on = 1; sub(/^FONTS[ \t]*:=/, "") }
    on { l = $0; sub(/\\$/, "", l); print l; if ($0 !~ /\\$/) exit }' Makefile)
nfonts=0
for font in $FONTS; do
    nfonts=$((nfonts + 1))
    found=no
    for f in "$B"/app/licenses/*-OFL-1.1.txt; do
        family=${f##*/}
        family=${family%-OFL-1.1.txt}
        case "$font" in "$family"*) [ -s "$f" ] && found=yes ;; esac
    done
    check "payload has the OFL text for $font" is "$found" yes
done
check "the Makefile's FONTS names the embedded fonts ($nfonts)" [ "$nfonts" -gt 0 ]
# Every vendored component needs its license in the package: a third_party/<dir> brings a
# LICENSE or COPYING file, a single-file library at the top of third_party/ must be named in a
# notice. The binary is linked static, so musl's notice goes too.
for d in third_party/*/; do
    d=${d%/}
    n=${d##*/}
    found=no
    for f in "$d"/LICENSE* "$d"/COPYING*; do [ -f "$f" ] && found=yes; done
    check "third_party/$n has a license file" is "$found" yes
    check "payload has licenses/$n-LICENSE.txt" [ -s "$B/app/licenses/$n-LICENSE.txt" ]
done
for h in third_party/*.h; do
    [ -f "$h" ] || continue
    check "a notice in licenses/ covers ${h##*/}" sh -c 'grep -qF "$1" "$2"/*.txt' sh "${h##*/}" "$B/app/licenses"
done
check "the stb notice has the MIT license" has "$B/app/licenses/stb-LICENSE.txt" "Permission is hereby granted"
check "payload has licenses/musl-COPYRIGHT.txt" has "$B/app/licenses/musl-COPYRIGHT.txt" "musl as a whole is licensed"

# The launchers' test hooks (tests/launcher_check.sh sets them) must fall back to the device's own
# paths and timings: each shipped script reads exactly these GRIMOIRE_* variables, with exactly
# these defaults, and no others.
pins() {  # pins WHAT FILE LINE...: FILE has each LINE, and no other line reads a GRIMOIRE_* variable
    what=$1
    file=$2
    shift 2
    for l in "$@"; do
        check "$what: $l" grep -qxF -- "$l" "$file"
    done
    n=$(grep -c '\$[{]*GRIMOIRE_' "$file")
    check "$what reads no other GRIMOIRE_* variable ($n in all)" is "$n" $#
}
pins "Grimoire.sh" "$B/script.sh" 'US=${GRIMOIRE_US:-/mnt/us}' 'TMP=${GRIMOIRE_TMP:-/tmp}'
pins "app/grimoire.sh" "$B/app/grimoire.sh" 'US=${GRIMOIRE_US:-/mnt/us}' 'TMP=${GRIMOIRE_TMP:-/tmp}' \
    'SYS=${GRIMOIRE_SYSROOT:-}' 'WD_TICK=${GRIMOIRE_WD_TICK:-2}' 'WD_FIRST=${GRIMOIRE_WD_FIRST:-25}' \
    'WD_STALE=${GRIMOIRE_WD_STALE:-15}'
pins "the reset scriptlet" "$B/zip/$RESET" 'S=${GRIMOIRE_US:-/mnt/us}/grimoire/settings.ini'
# The watchdog looks for the heartbeat where the binary writes it (TMP is /tmp on the device).
check "app/grimoire.sh watches \$TMP/grimoire.alive" grep -qE '^BEAT=\$TMP/grimoire\.alive( |$)' "$B/app/grimoire.sh"
check "the binary beats in /tmp/grimoire.alive" grep -qF /tmp/grimoire.alive "$B/app/grimoire"

if [ $fail = 0 ]; then
    rm -rf "$B"
    echo "PACKAGE CHECK PASSED"
else
    echo "PACKAGE CHECK FAILED (see $B)"
    exit 1
fi
