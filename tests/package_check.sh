#!/bin/sh
# Checks the zip people install (make package): its four files and the optional KOReader plugin
# folder; documents/Grimoire.sh, the self-unpacking scriptlet (SH_Integration's keys, BUILD_ID,
# the icon, the Capabilities line, the archive); the app inside it (the binary, the launcher,
# CAPABILITIES, KOReader's Grimoire plugin, and a license for every embedded font and every
# third-party part of the binary: the vendored code, musl and zig's compiler-rt, the parts of
# FBInk's tree with notices of their own, the Rust standard library and the libc crate); and that
# the launchers' test hooks fall back to the device's own paths and timings. The plugin the app
# carries also goes through tests/koplugin_check.sh.
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
# KOReader's Grimoire plugin, for a hand install into koreader/plugins: in a folder of its own,
# never a top-level koreader folder (copied over MTP, that could replace someone's KOReader).
KOZ="KOReader plugin (optional)/grimoire.koplugin"
KOPLUGIN=kindle/koreader/grimoire.koplugin
printf '%s\n' "KOReader plugin (optional)/" "$KOZ/" >"$B/allowed.list"
for f in "$KOPLUGIN"/*; do
    n=${f##*/}
    printf '%s\n' "$KOZ/$n" >>"$B/allowed.list"
    check "zip has $KOZ/$n, kindle's" cmp -s "$f" "$B/zip/$KOZ/$n"
done
PATCHZ="KOReader patches (optional)"
printf '%s\n' "$PATCHZ/" "$PATCHZ/fonts/" "$PATCHZ/fonts/Grimoire/" "$PATCHZ/README.txt" >>"$B/allowed.list"
for f in kindle/koreader/patches/*.lua; do
    n=${f##*/}
    printf '%s\n' "$PATCHZ/$n" >>"$B/allowed.list"
    check "optional patch $n matches the source" cmp -s "$f" "$B/zip/$PATCHZ/$n"
    if command -v luajit >/dev/null 2>&1; then
        check "optional patch $n parses" sh -c 'luajit -bl "$1" >/dev/null' sh "$B/zip/$PATCHZ/$n"
    fi
done
for f in assets/TerminessNerdFontMono-*.ttf assets/Terminess-OFL-1.1.txt; do
    n=${f##*/}
    printf '%s\n' "$PATCHZ/fonts/Grimoire/$n" >>"$B/allowed.list"
    check "manual patches carry $n" cmp -s "$f" "$B/zip/$PATCHZ/fonts/Grimoire/$n"
done
check "zip has no top-level koreader folder" sh -c '! grep -qiE "^koreader(/|$)" "$1"' sh "$B/zip.list"
grep -vxF -e documents/ -e documents/Grimoire.sh -e "$RESET" -e README.txt -e LICENSE.txt -f "$B/allowed.list" \
    "$B/zip.list" >"$B/extra.list"
check "zip has nothing else" [ ! -s "$B/extra.list" ]
check "LICENSE.txt is the GPL" cmp -s "$B/zip/LICENSE.txt" LICENSE
check "the reset scriptlet is kindle's" cmp -s "$B/zip/$RESET" "kindle/Grimoire - reset pen calibration.sh"
check "README.txt names version $VERSION" has "$B/zip/README.txt" "Grimoire $VERSION"
check "README.txt has no placeholder left" lacks "$B/zip/README.txt" "@VERSION@"
check "README.txt describes native PIN support" has "$B/zip/README.txt" "PASSCODE: "

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
for c in guard unpack-only hosted; do
    check "Capabilities include $c" sh -c 'case " $1 " in *" $2 "*) exit 0 ;; esac; exit 1' sh "$CAPS" "$c"
done
# A tool reads that line from the first 10 lines (Grimoire.sh says how), never by searching the
# whole file: the archive holds app/grimoire.sh, whose own Capabilities line is about that file.
# Still, the header's line comes first, so a tool that takes the first match gets it right.
first=$(grep -an '^# Capabilities:' "$S" | head -n 1 | cut -d: -f1)
check "the header's Capabilities line is the first in the file (line $first)" is "$first" 6
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
check "payload's bluetooth.sh is kindle's" cmp -s "$B/app/bluetooth.sh" kindle/app/bluetooth.sh
check "payload's update.sh is kindle's" cmp -s "$B/app/update.sh" kindle/app/update.sh
check "payload's CAPABILITIES match the header" is "$(cat "$B/app/CAPABILITIES" 2>/dev/null)" "$CAPS"
# app/grimoire.sh names what it supports itself: the guard, --hosted, --annotate (a page KOReader
# sends) and --book (a whole book), and no unpacking (that is Grimoire.sh's).
APP_CAPS=$(head -n 10 "$B/app/grimoire.sh" 2>/dev/null | sed -n 's/^# Capabilities: //p')
check "payload's grimoire.sh has its own Capabilities line ($APP_CAPS)" [ -n "$APP_CAPS" ]
for c in guard hosted annotate book; do
    check "payload's grimoire.sh Capabilities include $c" sh -c 'case " $1 " in *" $2 "*) exit 0 ;; esac; exit 1' sh "$APP_CAPS" "$c"
done
check "payload's grimoire.sh Capabilities leave out unpack-only" sh -c 'case " $1 " in *" unpack-only "*) exit 1 ;; esac' sh "$APP_CAPS"
# KOReader's Grimoire plugin, which Grimoire.sh installs into KOReader from app/koreader.
for f in "$KOPLUGIN"/*; do
    check "payload has koreader/grimoire.koplugin/${f##*/}, kindle's" cmp -s "$f" "$B/app/koreader/grimoire.koplugin/${f##*/}"
done
if tests/koplugin_check.sh "$B/app/koreader/grimoire.koplugin" "$B/koplugin-check" >"$B/koplugin.log" 2>&1; then
    echo "ok   the plugin the app carries passes tests/koplugin_check.sh"
    grep -E '^(skip|koplugin test:)' "$B/koplugin.log"
else
    cat "$B/koplugin.log"
    echo "FAIL the plugin the app carries passes tests/koplugin_check.sh"
    fail=1
fi
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
    # Its CREDITS too (FBInk's names the code it bundles into the binary: cutef8, dfa, font8x8).
    for f in "$d"/CREDITS*; do
        [ -f "$f" ] || continue
        check "payload has licenses/$n-CREDITS.txt" cmp -s "$f" "$B/app/licenses/$n-CREDITS.txt"
    done
done
for h in third_party/*.h; do
    [ -f "$h" ] || continue
    check "a notice in licenses/ covers ${h##*/}" sh -c 'grep -qF "$1" "$2"/*.txt' sh "${h##*/}" "$B/app/licenses"
done
check "the stb notice has the MIT license" has "$B/app/licenses/stb-LICENSE.txt" "Permission is hereby granted"
check "payload has licenses/musl-COPYRIGHT.txt" has "$B/app/licenses/musl-COPYRIGHT.txt" "musl as a whole is licensed"
# zig cc also links zig's own compiler-rt into the binary (64-bit division, float conversions,
# memcpy and more), which zig's MIT license covers.
L=$B/app/licenses
check "payload has licenses/zig-compiler-rt-LICENSE.txt, zig's MIT license" sh -c \
    'grep -qF "Zig contributors" "$1" && grep -qF "Permission is hereby granted" "$1"' sh "$L/zig-compiler-rt-LICENSE.txt"
# The binary is Rust's: rustc links the Rust standard library into it, whose notices (its own MIT
# and Apache-2.0 terms, and those of each crate it is built from) the toolchain ships, and the libc
# crate, dual-licensed the same way.
check "payload has licenses/rust-std-LICENSE.txt, the Rust standard library's notices" sh -c \
    'grep -qF "The Rust Standard Library is dual-licensed under Apache 2.0 and MIT terms" "$1" &&
     grep -qF "Permission is hereby granted" "$1" && grep -qF "Apache License" "$1"' sh "$L/rust-std-LICENSE.txt"
check "payload has licenses/rust-libc-LICENSE.txt, the libc crate's MIT and Apache-2.0 licenses" sh -c \
    'grep -qF "From the libc crate" "$1" && grep -qF "The Rust Project Developers" "$1" &&
     grep -qF "Permission is hereby granted" "$1" && grep -qF "Apache License" "$1"' sh "$L/rust-libc-LICENSE.txt"
# FBInk compiles two parts of its tree with notices of their own into the binary: cutef8
# (cutef8/utf8.c) and Bjoern Hoehrmann's UTF-8 decoder (cutef8/dfa.c). Each ships the notice at the
# head of its source and its entry in FBInk's CREDITS, the only text the tree has for them. A new
# FBInk source in the build needs its notice too.
check "the binary's FBInk sources are fbink.c, cutef8/utf8.c and cutef8/dfa.c" is \
    "$(sed -n 's/^FBINK_SRC *:= *//p' Makefile)" '$(FBINK_DIR)/fbink.c $(FBINK_DIR)/cutef8/utf8.c $(FBINK_DIR)/cutef8/dfa.c'
holds_lines() {  # holds_lines FILE: FILE has each line of stdin, whole
    [ -s "$1" ] || return 1
    while IFS= read -r l; do grep -qxF -- "$l" "$1" || return 1; done
}
F=third_party/FBInk
{ sed -n '1,/\*\//p' "$F/cutef8/utf8.c"; awk -v RS= '/\* cutef8\/utf8\./' "$F/CREDITS"; } >"$B/cutef8.notice"
{ grep '^// Copyright' "$F/cutef8/dfa.c"; awk -v RS= '/\* cutef8\/dfa\./' "$F/CREDITS"; } >"$B/dfa.notice"
check "cutef8's notice found in the tree" grep -qF "placed in the public domain" "$B/cutef8.notice"
check "dfa's notice found in the tree" grep -qF "Copyright (c) 2008-2010 Bjoern Hoehrmann" "$B/dfa.notice"
check "payload has licenses/cutef8-LICENSE.txt: utf8.c's head and its CREDITS entry" \
    holds_lines "$L/cutef8-LICENSE.txt" <"$B/cutef8.notice"
check "payload has licenses/dfa-LICENSE.txt: dfa.c's copyright and its CREDITS entry" \
    holds_lines "$L/dfa-LICENSE.txt" <"$B/dfa.notice"

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
pins "the reset scriptlet" "$B/zip/$RESET" 'S=${GRIMOIRE_US:-/mnt/us}/grimoire/settings.ini' \
    'p=$(cat "${GRIMOIRE_TMP:-/tmp}/grimoire.launcher.pid" 2>/dev/null)'
# The watchdog looks for the heartbeat where the binary writes it (TMP is /tmp on the device).
check "app/grimoire.sh watches \$TMP/grimoire.alive" grep -qE '^BEAT=\$TMP/grimoire\.alive( |$)' "$B/app/grimoire.sh"
check "the binary beats in /tmp/grimoire.alive" grep -qF /tmp/grimoire.alive "$B/app/grimoire"
# restore()'s backstop for a power-service watcher a crash left behind (pkill -f) must match the
# watcher's command line, both kinds of it (src/input_evdev.c, WATCH_EVENTS): the two drifted once.
PAT=$(sed -n 's/^ *pkill -f "\(.*\)" 2>.*/\1/p' "$B/app/grimoire.sh")
EVENTS=$(sed -n 's/^#define WATCH_EVENTS "\(.*\)"$/\1/p' src/input_evdev.c)
check "restore() has a pkill pattern ($PAT)" [ -n "$PAT" ]
check "src/input_evdev.c names the watched events ($EVENTS)" [ -n "$EVENTS" ]
for w in "lipc-wait-event -s 0 $EVENTS" "lipc-wait-event -m -s 0 $EVENTS"; do
    check "the binary runs '$w'" sh -c 'tr "\000" "\n" <"$1" | grep -qF -- "$2"' sh "$B/app/grimoire" "$w"
    check "restore()'s pkill pattern matches '$w'" sh -c 'printf "%s\n" "$2" | grep -qE -- "$1"' sh "$PAT" "$w"
done

if [ $fail = 0 ]; then
    rm -rf "$B"
    echo "PACKAGE CHECK PASSED"
else
    echo "PACKAGE CHECK FAILED (see $B)"
    exit 1
fi
