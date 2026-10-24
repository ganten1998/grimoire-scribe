#!/bin/sh
# Checks KOReader's Grimoire plugin: kindle/koreader/grimoire.koplugin, or the folder given
# (tests/package_check.sh passes the copy inside the zip's app, so make check runs this). Every
# Lua file parses (luajit, else luac); with luajit, tests/koplugin_test.lua runs the plugin
# against stand-ins for the KOReader modules it uses; with luacheck, that has its say too. With
# neither luajit nor luac it prints a skip line and passes.
# usage: tests/koplugin_check.sh [plugin folder [scratch folder]]
set -u
cd "$(dirname "$0")/.." || exit 1
DIR=${1:-kindle/koreader/grimoire.koplugin}
B=${2:-build/koplugin-check}
fail=0
check() {  # check NAME COMMAND...: ok when the command succeeds
    name=$1
    shift
    if "$@"; then echo "ok   $name"; else echo "FAIL $name"; fail=1; fi
}

for f in main.lua _meta.lua; do
    check "plugin has $f" [ -s "$DIR/$f" ]
done
if command -v luajit >/dev/null 2>&1; then
    for f in "$DIR"/*.lua tests/koplugin_test.lua; do
        check "luajit parses ${f##*/}" sh -c 'luajit -bl "$1" >/dev/null' sh "$f"
    done
    rm -rf "$B" && mkdir -p "$B"
    if luajit tests/koplugin_test.lua "$DIR" "$B"; then
        rm -rf "$B"
    else
        echo "FAIL tests/koplugin_test.lua (scratch Kindles in $B)"
        fail=1
    fi
elif command -v luac >/dev/null 2>&1; then
    for f in "$DIR"/*.lua tests/koplugin_test.lua; do
        check "luac parses ${f##*/}" luac -p "$f"
    done
    echo "skip the plugin's stub test (needs luajit)"
else
    echo "skip the KOReader plugin check (neither luajit nor luac is installed)"
fi
if command -v luacheck >/dev/null 2>&1; then
    # KOReader methods receive implicit self even when unused; translation literals stay whole.
    check "luacheck" luacheck --no-color -q --std luajit --no-self --max-line-length 200 \
        --globals G_reader_settings -- "$DIR"
else
    echo "skip luacheck (not installed)"
fi

if [ $fail = 0 ]; then
    echo "KOPLUGIN CHECK PASSED"
else
    echo "KOPLUGIN CHECK FAILED"
    exit 1
fi
