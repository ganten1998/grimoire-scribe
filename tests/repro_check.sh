#!/bin/sh
# Builds every release file twice from clean and checks that the bytes are the same: the zip people
# install (and its copy named for the version), the KOReader patches zip, the app's tar inside
# Grimoire.sh and its BUILD_ID, the KPM package, its manifests and its checksum. Each build has an
# empty build folder of its own; the second also starts from an empty zig cache, under umask 077,
# another time zone and the C locale, so nothing of the builder's setup can reach the files. Both
# must also match what make package left in BUILD, the zip the other checks test. tools/archive.py
# and kindle/no-comment.ld are what make the bytes the same; this check holds them to it.
# usage: tests/repro_check.sh [build]
set -u
cd "$(dirname "$0")/.." || exit 1
BUILD=${1:-build}
R=$BUILD/repro
VERSION=$(cat VERSION)
# Each build is a make of its own, not a part of the caller's (whose -j and variables are in these);
# ZIG comes from the caller (make package-check passes its own), so both use the same compiler.
unset MAKEFLAGS MFLAGS MAKELEVEL
rm -rf "$R" && mkdir -p "$R"
fail=0
check() {  # check NAME COMMAND...: ok when the command succeeds
    name=$1
    shift
    if "$@"; then echo "ok   $name"; else echo "FAIL $name"; fail=1; fi
}
build() {  # build NAME: make kpm-package into $R/NAME, its log in $R/NAME.log
    if make BUILD="$R/$1" ZIG="${ZIG:-zig}" kpm-package >"$R/$1.log" 2>&1; then
        echo "ok   clean build $1"
    else
        tail -n 20 "$R/$1.log"
        echo "FAIL clean build $1 (see $R/$1.log)"
        echo "REPRO CHECK FAILED"
        exit 1
    fi
}
build_id() {  # build_id ZIP: the BUILD_ID line of the zip's documents/Grimoire.sh
    unzip -p "$1" documents/Grimoire.sh 2>/dev/null | LC_ALL=C sed -n 's/^BUILD_ID=//p' | head -n 1
}

# Build b's zig cache is in $R (an absolute path whether BUILD is one or not), so the reset above
# empties it and the cleanup below removes it.
case $R in /*) cache=$R/zig-cache ;; *) cache=$(pwd)/$R/zig-cache ;; esac
(
    export LC_ALL=en_US.UTF-8 TZ=UTC
    build a
) || exit 1
check "build b starts from an empty zig cache" [ ! -e "$cache" ]
(
    umask 077
    export ZIG_GLOBAL_CACHE_DIR="$cache" ZIG_LOCAL_CACHE_DIR="$cache" LC_ALL=C TZ=Asia/Kathmandu
    build b
) || exit 1
check "build b's zig cache is in $R, which the cleanup removes" [ -d "$R/zig-cache" ]

kpkg=$(cd "$R/a/kpm" && ls grimoire_*_kindlehf.kpkg 2>/dev/null)
check "build a made one KPM package ($kpkg)" [ "$(printf '%s\n' "$kpkg" | grep -c .)" = 1 ]
for f in dist/Grimoire.zip "dist/Grimoire-$VERSION.zip" dist/KOReader-patches.zip dist/app.tar kindle/grimoire \
    "kpm/$kpkg" kpm/manifest.json kpm/manifest.v2.json kpm/SHA256SUMS.txt; do
    check "$f: the same bytes in both clean builds" cmp -s "$R/a/$f" "$R/b/$f"
done
for f in dist/Grimoire.zip "dist/Grimoire-$VERSION.zip" dist/KOReader-patches.zip; do
    check "$f: the same bytes as $BUILD/$f" cmp -s "$R/a/$f" "$BUILD/$f"
done
ID=$(build_id "$R/a/dist/Grimoire.zip")
check "BUILD_ID is the app tar's checksum ($ID)" [ "$ID" = "$(shasum "$R/a/dist/app.tar" | cut -c1-12)" ]
check "BUILD_ID is the same in both clean builds" [ "$(build_id "$R/b/dist/Grimoire.zip")" = "$ID" ]
check "BUILD_ID is the same as in $BUILD" [ "$(build_id "$BUILD/dist/Grimoire.zip")" = "$ID" ]

if [ $fail = 0 ]; then
    rm -rf "$R"
    echo "REPRO CHECK PASSED"
else
    echo "REPRO CHECK FAILED (see $R)"
    exit 1
fi
