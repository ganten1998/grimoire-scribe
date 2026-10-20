#!/bin/sh
# Runs the Kindle launchers from the zip (make package) on this computer: documents/Grimoire.sh
# and the app/grimoire.sh it unpacks, in a scratch device tree, with the Kindle's tools replaced
# by stand-ins that record every call (tests/shims) and the binary by a shell script
# (tests/shims/fake-grimoire). Checks unpacking, --unpack-only, the KOReader guard,
# --reset-calibration, the order of the pause and the restore, the restore on a locked Kindle,
# the watchdog and its device timings, the move from Quire, the hosted run inside KOReader
# (--hosted koreader, under tests/shims/fake-koreader) and its refusals, and the install of
# KOReader's Grimoire plugin, once with each POSIX shell found (sh, dash, busybox sh); the shells
# run side by side.
# usage: tests/launcher_check.sh [build/dist/Grimoire.zip]
set -u
cd "$(dirname "$0")/.." || exit 1
REPO=$(pwd)
ZIP=${1:-build/dist/Grimoire.zip}
case "$ZIP" in /*) ;; *) ZIP=$REPO/$ZIP ;; esac
W=$(cd "$(dirname "$ZIP")/.." && pwd)/launcher-check
SHIMS=$REPO/tests/shims
export LC_ALL=C  # Grimoire.sh ends in a binary archive
unset KOREADER_DIR FAKE_MODE FAKE_EXIT_STATE SHIM_HOLD SHIM_NICE GRIMOIRE_WD_TICK GRIMOIRE_WD_FIRST GRIMOIRE_WD_STALE \
    STOP_FRAMEWORK AWESOME_STOPPED PILLOW_HARD_DISABLED FAKE_KO_PARENT FAKE_KO_HOLDS
rm -rf "$W" && mkdir -p "$W"
fail=0
check() {  # check NAME COMMAND...: ok when the command succeeds
    name=$1
    shift
    if "$@"; then echo "ok   $name"; else echo "FAIL $name"; fail=1; fi
}
has() { grep -qF -- "$2" "$1" 2>/dev/null; }  # has FILE TEXT
lacks() { ! grep -qF -- "$2" "$1" 2>/dev/null; }
is() { [ "$1" = "$2" ]; }
content() { cat "$1" 2>/dev/null; }
wait_until() {  # wait_until TENTHS COMMAND...: polls the command until it succeeds
    n=$1
    shift
    until "$@"; do
        [ "$n" -gt 0 ] || return 1
        n=$((n - 1))
        sleep 0.1
    done
}

# Every script parses, with each shell at hand.
for f in kindle/Grimoire.sh kindle/app/grimoire.sh "kindle/Grimoire - reset pen calibration.sh" \
    tests/launcher_check.sh tests/package_check.sh tests/koplugin_check.sh tests/shims/*; do
    check "sh -n $f" sh -n "$f"
    if command -v dash >/dev/null 2>&1; then check "dash -n $f" dash -n "$f"; fi
done
if command -v shellcheck >/dev/null 2>&1; then
    check "shellcheck" shellcheck -s sh -S warning kindle/Grimoire.sh kindle/app/grimoire.sh \
        "kindle/Grimoire - reset pen calibration.sh" tests/launcher_check.sh tests/package_check.sh \
        tests/koplugin_check.sh tests/shims/*
else
    echo "skip shellcheck (not installed)"
fi

if ! unzip -p "$ZIP" documents/Grimoire.sh >"$W/Grimoire.sh" 2>/dev/null ||
    ! unzip -p "$ZIP" "documents/Grimoire - reset pen calibration.sh" >"$W/reset.sh" 2>/dev/null; then
    echo "FAIL cannot read $ZIP (make package)"
    echo "LAUNCHER CHECK FAILED"
    exit 1
fi
BUILD_ID=$(awk -F= '/^BUILD_ID=/ { print $2; exit }' "$W/Grimoire.sh")

# A scratch Kindle: $T/us is /mnt/us, $T/tmp is /tmp, $T/sys holds /etc, /var and /dev/fb0 (a
# plain file), $T/state the stand-ins' answers, $T/calls.log every call they got.
enter() {  # enter NAME: points the variables, and the launchers and stand-ins run next, at NAME
    T=$W/$SH_NAME/$1
    US=$T/us
    APP=$US/grimoire/app
    DOC=$US/documents/Grimoire.sh
    RESET="$US/documents/Grimoire - reset pen calibration.sh"
    LOGF=$US/grimoire/grimoire.log
    GRIMOIRE_US=$US GRIMOIRE_TMP=$T/tmp GRIMOIRE_SYSROOT=$T/sys SHIM_LOG=$T/calls.log SHIM_STATE=$T/state
    export GRIMOIRE_US GRIMOIRE_TMP GRIMOIRE_SYSROOT SHIM_LOG SHIM_STATE
}
tree() {  # tree NAME: a fresh scratch Kindle NAME, entered
    enter "$1"
    rm -rf "$T"
    mkdir -p "$T/us/documents" "$T/tmp" "$T/sys/etc/upstart" "$T/sys/var/tmp" "$T/sys/var/local/system" \
        "$T/sys/dev" "$T/state/lipc" "$T/state/pidof"
    ln "$W/Grimoire.sh" "$DOC" 2>/dev/null || cp "$W/Grimoire.sh" "$DOC"  # never written to
    cp "$W/reset.sh" "$RESET"
    : >"$T/sys/etc/upstart/volumd.conf"
    : >"$T/sys/etc/upstart/statusbar.conf"
    echo "Kindle 5.19.6 (test)" >"$T/sys/etc/prettyversion.txt"
    printf 'LANG=es_ES.UTF-8\nLC_ALL=es_ES.UTF-8\n' >"$T/sys/var/local/system/locale"
    printf 'KINDLE' >"$T/sys/dev/fb0"
    echo active >"$T/state/lipc/com.lab126.powerd.state"
    echo "screensaver splash" >"$T/state/lipc/com.lab126.blanket.load"
    : >"$SHIM_LOG"
}
fake() { cp "$SHIMS/fake-grimoire" "$APP/grimoire" && chmod +x "$APP/grimoire"; }
at() { grep -nF -- "$1" "$SHIM_LOG" | head -n 1 | cut -d: -f1; }  # line of the first such call
in_order() {  # in_order CALL...: each call was made, in this order
    prev=0
    for c in "$@"; do
        n=$(at "$c")
        [ -n "$n" ] && [ "$n" -gt "$prev" ] || return 1
        prev=$n
    done
}
launcher_done() { has "$LOGF" "restoring the Kindle UI" && [ ! -e "$GRIMOIRE_TMP/grimoire.launcher.pid" ]; }
fake_gone() {  # the fake binary has stopped running
    p=$(cat "$SHIM_STATE/grimoire.pid" 2>/dev/null)
    [ -n "$p" ] && ! kill -0 "$p" 2>/dev/null
}
finish() {  # finish PID TENTHS: waits for a launcher run in the background, its status in rc.
    # Fails when it is not done in time, and then stops it (its trap stops the binary) and the fake.
    if wait_until "$2" launcher_done; then
        wait "$1"
        rc=$?
        return 0
    fi
    kill "$1" 2>/dev/null
    wait "$1"
    rc=$?
    fake_gone || kill -9 "$(cat "$SHIM_STATE/grimoire.pid")" 2>/dev/null
    return 1
}
lines_in() { wc -l <"$1" | tr -d ' '; }
log_order() {  # log_order TEXT...: each line is in grimoire.log, in this order
    prev=0
    for c in "$@"; do
        n=$(grep -nF -- "$c" "$LOGF" | head -n 1 | cut -d: -f1)
        [ -n "$n" ] && [ "$n" -gt "$prev" ] || return 1
        prev=$n
    done
}
PASSCODE_REL=sys/var/local/system/userpasswdenabled
settings_reset() {  # the calibration is gone, the other settings are kept
    S=$US/grimoire/settings.ini
    lacks "$S" "pen." && lacks "$S" "touch." && has "$S" "theme=gothic" && has "$S" "http_port=18763" &&
        [ ! -e "$S.reset" ]
}
calibrated() { printf 'theme=gothic\npen.valid=1\npen.ax=1.5\ntouch.valid=1\ntouch.by=-3\nhttp_port=18763\n' >"$US/grimoire/settings.ini"; }
PILLOW_OFF="lipc-set-prop com.lab126.pillow disableEnablePillow disable"
PILLOW_ON="lipc-set-prop com.lab126.pillow disableEnablePillow enable"
HOME_UP="lipc-set-prop com.lab126.appmgrd start app://com.lab126.booklet.home"
STARTED="grimoire root="  # what the fake binary records
after_start() { n=$(at "$STARTED"); [ -n "$n" ] && tail -n +"$((n + 1))" "$SHIM_LOG" | grep -qF -- "$1"; }  # after_start CALL
# Hosted inside KOReader: a scratch Kindle with the app unpacked and KOReader installed. ko_paused
# runs a command with what koreader.sh exports for a session that paused the Kindle UI (and a
# quick watchdog), through env: VAR=value words may come first, and nothing outlives the call
# (assignments before a shell function do, in some shells). The command is mostly fake-koreader,
# a reader.lua that koreader.sh started, running its own command the way os.execute does.
hosted_tree() {  # hosted_tree NAME
    tree "$1"
    sh "$DOC" --unpack-only
    fake
    mkdir -p "$US/koreader"
    : >"$US/koreader/reader.lua"
    : >"$SHIM_LOG"
}
ko_paused() { env KOREADER_DIR="$US/koreader" STOP_FRAMEWORK=no AWESOME_STOPPED=yes PILLOW_HARD_DISABLED=yes GRIMOIRE_WD_TICK=0.2 "$@"; }
# KOReader's Grimoire plugin in KOReader's plugins folder is the one the app carries.
PLUGIN_REL=koreader/plugins/grimoire.koplugin
plugin_is_payload() {
    [ -f "$US/$PLUGIN_REL/main.lua" ] || return 1
    for f in "$APP"/koreader/grimoire.koplugin/*; do
        cmp -s "$f" "$US/$PLUGIN_REL/${f##*/}" || return 1
    done
}
start_doc() {  # a start from the Library, waited for, with grimoire.log emptied first
    : >"$LOGF"
    GRIMOIRE_WD_TICK=0.2 sh "$DOC"
    wait_until 150 launcher_done
}

suite() {
    s="[$SH_NAME]"

    # The device's own watchdog timings (no GRIMOIRE_WD_* set): a Grimoire that takes 3 s to its
    # first frame and then beats every 2 s is left alone. That takes about 9 s, so it runs in the
    # background while the other cases go on, and is checked at the end.
    tree timings
    sh "$DOC" --unpack-only
    fake
    : >"$SHIM_LOG"
    FAKE_MODE=slow sh "$APP/grimoire.sh" &
    timings_pid=$!

    # First run on a fresh Kindle: unpacks, then starts app/grimoire.sh detached. The setsid
    # stand-in holds the launch until the fake binary is in place.
    tree first
    SHIM_HOLD=$T/go GRIMOIRE_WD_TICK=0.2 sh "$DOC"
    rc=$?
    check "$s first run returns 0 ($rc)" is "$rc" 0
    check "$s first run unpacks the app" sh -c '[ -f "$1/grimoire" ] && [ -f "$1/grimoire.sh" ] && [ -f "$1/CAPABILITIES" ]' sh "$APP"
    check "$s first run unpacks KOReader's Grimoire plugin" sh -c '[ -s "$1/main.lua" ] && [ -s "$1/_meta.lua" ]' sh \
        "$APP/koreader/grimoire.koplugin"
    check "$s first run records build $BUILD_ID" is "$(content "$APP/BUILD_ID")" "$BUILD_ID"
    check "$s first run logs the unpack" has "$LOGF" "Grimoire.sh: unpacked build $BUILD_ID"
    check "$s app/CAPABILITIES" is "$(content "$APP/CAPABILITIES")" "guard unpack-only hosted"
    fake
    : >"$T/go"
    check "$s first run: the launcher finishes" wait_until 150 launcher_done
    check "$s first run starts app/grimoire.sh detached" has "$SHIM_LOG" "setsid sh $APP/grimoire.sh"
    check "$s first run starts the binary" has "$SHIM_LOG" "$STARTED$US/grimoire awesome_stopped=yes"
    check "$s pillow off, then back on" in_order "$PILLOW_OFF" "$STARTED" "$PILLOW_ON"
    check "$s awesome stopped, then continued" in_order "killall -STOP awesome" "$STARTED" "killall -CONT awesome"
    check "$s status bar job stopped, then started" in_order "stop statusbar" "$STARTED" "start statusbar"
    check "$s preventScreenSaver 1, then 0" in_order "lipc-set-prop com.lab126.powerd preventScreenSaver 1" "$STARTED" \
        "lipc-set-prop com.lab126.powerd preventScreenSaver 0"
    check "$s firewall hole opened, then closed" in_order "iptables -A INPUT -i wlan0 -p tcp --dport 8765 -j ACCEPT" \
        "$STARTED" "iptables -D INPUT -i wlan0 -p tcp --dport 8765 -j ACCEPT"
    check "$s volumd stopped, then continued" in_order "killall -STOP volumd" "$STARTED" "killall -CONT volumd"
    check "$s the home booklet started after pillow" in_order "$PILLOW_ON" "$HOME_UP"
    check "$s the saved screen is put back" is "$(content "$T/sys/dev/fb0")" KINDLE
    check "$s the saved screen is dropped after" [ ! -e "$T/sys/var/tmp/grimoire-fb.dump" ]
    check "$s the heartbeat file is cleaned up" [ ! -e "$GRIMOIRE_TMP/grimoire.alive" ]
    check "$s log: locale" has "$LOGF" "grimoire.sh: locale: LANG=es_ES.UTF-8"
    check "$s log: passcode" has "$LOGF" "grimoire.sh: device passcode: off"
    check "$s no passcode: nothing to tell the binary" has "$SHIM_LOG" "grimoire passcode note= flag=off"
    check "$s no passcode: no marker" [ ! -e "$US/grimoire/.passcode-paused" ]
    check "$s log: power state" has "$LOGF" "grimoire.sh: power state: active"
    check "$s log: blanket modules" has "$LOGF" "grimoire.sh: blanket modules: screensaver splash"
    check "$s log: KOReader" has "$LOGF" "grimoire.sh: KOReader: not installed"
    check "$s log: user id" has "$LOGF" "grimoire.sh: user id: $(id -u)"
    check "$s log: power state at exit" has "$LOGF" "grimoire.sh: power state at exit: active"
    check "$s log: exit status" has "$LOGF" "grimoire.sh: Grimoire exited with status 0"

    # --unpack-only: unpacks as a normal run would, starts nothing; again with the same build it
    # does nothing at all; a different build unpacks again.
    tree unpack
    sh "$DOC" --unpack-only
    rc=$?
    check "$s --unpack-only returns 0 ($rc)" is "$rc" 0
    check "$s --unpack-only unpacks the app" sh -c '[ -f "$1/grimoire" ] && [ -f "$1/grimoire.sh" ]' sh "$APP"
    check "$s --unpack-only records the build" is "$(content "$APP/BUILD_ID")" "$BUILD_ID"
    check "$s --unpack-only starts nothing" lacks "$SHIM_LOG" "setsid"
    check "$s --unpack-only: no launcher run" lacks "$LOGF" "grimoire.sh:"
    echo changed >>"$APP/CAPABILITIES"
    lines=$(lines_in "$LOGF")
    : >"$SHIM_LOG"
    sh "$DOC" --unpack-only
    rc=$?
    check "$s same build again: returns 0 ($rc)" is "$rc" 0
    check "$s same build again: nothing unpacked" has "$APP/CAPABILITIES" changed
    check "$s same build again: nothing logged" is "$(lines_in "$LOGF")" "$lines"
    check "$s same build again: no tool called" [ ! -s "$SHIM_LOG" ]
    echo 000000000000 >"$APP/BUILD_ID"
    sh "$DOC" --unpack-only
    check "$s another build unpacks again" sh -c '! grep -q changed "$1" && [ "$(cat "$2")" = "$3" ]' sh \
        "$APP/CAPABILITIES" "$APP/BUILD_ID" "$BUILD_ID"

    # Inside KOReader (it exports KOREADER_DIR to what it runs): refuse with 3, unpack-only allowed.
    tree koreader
    KOREADER_DIR=$US/koreader sh "$DOC" 2>"$T/stderr"
    rc=$?
    check "$s inside KOReader: exit 3 ($rc)" is "$rc" 3
    check "$s inside KOReader: nothing unpacked or created" [ ! -e "$US/grimoire" ]
    check "$s inside KOReader: the reason on stderr" has "$T/stderr" "not starting inside KOReader"
    KOREADER_DIR=$US/koreader sh "$DOC" --unpack-only
    rc=$?
    check "$s inside KOReader: --unpack-only allowed ($rc)" sh -c '[ "$1" = 0 ] && [ -f "$2" ]' sh "$rc" "$APP/grimoire"
    fake
    : >"$SHIM_LOG"
    KOREADER_DIR=$US/koreader sh "$DOC" 2>/dev/null
    rc=$?
    check "$s inside KOReader: exit 3 once unpacked ($rc)" is "$rc" 3
    check "$s inside KOReader: logged" has "$LOGF" "Grimoire.sh: not starting inside KOReader"
    check "$s inside KOReader: nothing started" lacks "$SHIM_LOG" "setsid"
    KOREADER_DIR=$US/koreader sh "$DOC" --hosted 2>/dev/null
    rc=$?
    check "$s inside KOReader: --hosted naming no host, refused by app/grimoire.sh ($rc)" is "$rc" 3
    check "$s inside KOReader: --hosted naming no host, logged" has "$LOGF" "grimoire.sh: not starting: --hosted without a name"
    KOREADER_DIR=$US/koreader sh "$APP/grimoire.sh" 2>/dev/null
    rc=$?
    check "$s app/grimoire.sh inside KOReader: exit 3 ($rc)" is "$rc" 3
    check "$s app/grimoire.sh inside KOReader: logged" has "$LOGF" "grimoire.sh: not starting inside KOReader"
    check "$s app/grimoire.sh inside KOReader: Kindle UI untouched" sh -c '! grep -qE "lipc-set-prop|killall|^grimoire " "$1"' sh "$SHIM_LOG"
    sleep 0.5
    check "$s inside KOReader: no eips message (KOReader shows the status)" lacks "$SHIM_LOG" "eips"

    # A reader.lua still running, KOREADER_DIR unset (a Library tap with KOReader left behind).
    tree pidof
    sh "$DOC" --unpack-only
    fake
    echo 4242 >"$SHIM_STATE/pidof/reader.lua"
    : >"$SHIM_LOG"
    sh "$DOC" 2>/dev/null
    rc=$?
    check "$s KOReader still running: exit 3 ($rc)" is "$rc" 3
    check "$s KOReader still running: logged" has "$LOGF" "Grimoire.sh: not starting: KOReader is still running (reader.lua, pid 4242)"
    check "$s KOReader still running: nothing started" lacks "$SHIM_LOG" "setsid"
    check "$s KOReader still running: said on screen" wait_until 50 has "$SHIM_LOG" \
        "eips 1 3 Grimoire did not start: KOReader is running."
    sh "$APP/grimoire.sh" 2>/dev/null
    rc=$?
    check "$s app/grimoire.sh with KOReader running: exit 3 ($rc)" is "$rc" 3
    check "$s app/grimoire.sh with KOReader running: binary not started" lacks "$SHIM_LOG" "$STARTED"

    # --reset-calibration passes the guard, edits settings.ini, starts nothing.
    tree reset
    sh "$DOC" --unpack-only
    fake
    calibrated
    : >"$SHIM_LOG"
    KOREADER_DIR=$US/koreader sh "$DOC" --reset-calibration
    rc=$?
    check "$s --reset-calibration inside KOReader: returns 0 ($rc)" is "$rc" 0
    check "$s --reset-calibration: calibration gone, the rest kept" settings_reset
    check "$s --reset-calibration: logged" has "$LOGF" "grimoire.sh: pen and touch calibration reset"
    check "$s --reset-calibration: nothing started" sh -c '! grep -qE "setsid|^grimoire |lipc-set-prop" "$1"' sh "$SHIM_LOG"
    calibrated
    echo 4242 >"$SHIM_STATE/pidof/reader.lua"
    sh "$APP/grimoire.sh" --reset-calibration
    rc=$?
    check "$s app/grimoire.sh --reset-calibration with KOReader running ($rc)" sh -c '[ "$1" = 0 ]' sh "$rc"
    check "$s app/grimoire.sh --reset-calibration: calibration gone" settings_reset
    calibrated
    out=$(sh "$RESET")
    check "$s reset scriptlet: says so ($out)" is "$out" "Grimoire: pen and touch calibration reset."
    check "$s reset scriptlet: calibration gone, the rest kept" settings_reset
    rm -f "$US/grimoire/settings.ini"
    out=$(sh "$RESET")
    check "$s reset scriptlet without settings ($out)" is "$out" "Grimoire: nothing to reset (no settings yet)."

    # Locked when Grimoire exits (the power service says screenSaver, readyToSuspend or
    # suspended): the saved screen stays off the lock screen, the rest is restored. Checked once,
    # on the way: the port from settings.ini, renice, and the diagnostics of a passcode, Special
    # Offers and an installed (not running) KOReader.
    for state in screenSaver readyToSuspend suspended; do
        tree "locked-$state"
        sh "$DOC" --unpack-only
        fake
        printf 'http_port=18763\n' >"$US/grimoire/settings.ini"
        : >"$T/sys/var/local/system/userpasswdenabled"
        mkdir -p "$US/koreader"
        : >"$US/koreader/reader.lua"
        echo v2026.07.1 >"$US/koreader/git-rev"
        echo "screensaver ad_screensaver splash" >"$SHIM_STATE/lipc/com.lab126.blanket.load"
        : >"$SHIM_LOG"
        FAKE_EXIT_STATE=$state SHIM_NICE=5 GRIMOIRE_WD_TICK=0.2 sh "$APP/grimoire.sh"
        rc=$?
        l="$s locked at exit ($state)"
        check "$l: returns 0 ($rc)" is "$rc" 0
        check "$l: the binary ran" has "$SHIM_LOG" "grimoire exits"
        check "$l: the saved screen is not put back" is "$(content "$T/sys/dev/fb0")" GRIMOIRE
        check "$l: the saved screen is dropped" [ ! -e "$T/sys/var/tmp/grimoire-fb.dump" ]
        check "$l: logged" has "$LOGF" "grimoire.sh: power state at exit: $state, so the saved screen is not put back"
        check "$l: jobs, awesome, pillow and home still restored" in_order "$STARTED" "start statusbar" \
            "killall -CONT awesome" "$PILLOW_ON" "$HOME_UP"
        [ "$state" = screenSaver ] || continue
        check "$s firewall hole on the port from settings.ini" in_order "iptables -A INPUT -i wlan0 -p tcp --dport 18763 -j ACCEPT" \
            "$STARTED" "iptables -D INPUT -i wlan0 -p tcp --dport 18763 -j ACCEPT"
        check "$s started niced: back to normal priority" has "$SHIM_LOG" "renice -n -5"
        check "$s log: passcode on" has "$LOGF" "grimoire.sh: device passcode: on"
        check "$s log: Special Offers" has "$LOGF" "grimoire.sh: blanket modules: screensaver ad_screensaver splash (Special Offers)"
        check "$s log: KOReader installed, not running" has "$LOGF" "grimoire.sh: KOReader: installed, version v2026.07.1"
    done

    # A Device Passcode: paused while Grimoire runs (the binary is told), back on once the Kindle UI
    # is back, a marker meanwhile.
    tree passcode
    sh "$DOC" --unpack-only
    fake
    : >"$T/$PASSCODE_REL"
    : >"$SHIM_LOG"
    GRIMOIRE_WD_TICK=0.2 sh "$APP/grimoire.sh"
    check "$s passcode: paused while Grimoire runs, the binary told" has "$SHIM_LOG" "grimoire passcode note=paused flag=off"
    check "$s passcode: back on after" [ -e "$T/$PASSCODE_REL" ]
    check "$s passcode: no marker left" [ ! -e "$US/grimoire/.passcode-paused" ]
    check "$s passcode: paused before the start, back on after the restore" log_order \
        "grimoire.sh: device passcode: paused while Grimoire runs" "grimoire.sh: Grimoire running as pid" \
        "grimoire.sh: restoring the Kindle UI" "grimoire.sh: device passcode: back on"
    # The home booklet starts with the passcode still paused: it comes back last (as KOReader does).
    check "$s passcode: still paused when the home booklet starts" has "$SHIM_LOG" "$HOME_UP (passcode off)"
    # A session that never ended (the Kindle restarted under Grimoire: the marker stayed, no
    # passcode): it is not turned back on, and the binary is told, at this start and the next two
    # while the passcode stays off, then no more. The pause marker goes at once.
    tree passcode-left-off
    sh "$DOC" --unpack-only
    fake
    : >"$US/grimoire/.passcode-paused"
    : >"$SHIM_LOG"
    GRIMOIRE_WD_TICK=0.2 sh "$APP/grimoire.sh"
    check "$s passcode left off: the binary told" has "$SHIM_LOG" "grimoire passcode note=left-off flag=off"
    check "$s passcode left off: not turned back on" [ ! -e "$T/$PASSCODE_REL" ]
    check "$s passcode left off: the marker goes" [ ! -e "$US/grimoire/.passcode-paused" ]
    check "$s passcode left off: logged" has "$LOGF" "grimoire.sh: device passcode: left off by a session that did not end"
    check "$s passcode left off: told once so far" is "$(content "$US/grimoire/.passcode-left-off")" 1
    for k in 2 3 4; do
        : >"$SHIM_LOG"
        GRIMOIRE_WD_TICK=0.2 sh "$APP/grimoire.sh"
        if [ "$k" -le 3 ]; then
            check "$s passcode left off: told again at start $k" has "$SHIM_LOG" "grimoire passcode note=left-off flag=off"
            check "$s passcode left off: logged at start $k" has "$LOGF" "telling the user ($k of 3)"
        else
            check "$s passcode left off: not told a fourth time" has "$SHIM_LOG" "grimoire passcode note= flag=off"
            check "$s passcode left off: nothing left to count" [ ! -e "$US/grimoire/.passcode-left-off" ]
        fi
    done
    # Turned back on by the user after such a start: no more telling, an ordinary pause.
    tree passcode-left-off-then-on
    sh "$DOC" --unpack-only
    fake
    : >"$US/grimoire/.passcode-paused"
    GRIMOIRE_WD_TICK=0.2 sh "$APP/grimoire.sh"
    : >"$T/$PASSCODE_REL"
    : >"$SHIM_LOG"
    GRIMOIRE_WD_TICK=0.2 sh "$APP/grimoire.sh"
    check "$s passcode back on after a left-off start: an ordinary pause" has "$SHIM_LOG" "grimoire passcode note=paused flag=off"
    check "$s passcode back on after a left-off start: nothing left to count" [ ! -e "$US/grimoire/.passcode-left-off" ]
    check "$s passcode back on after a left-off start: back on after" [ -e "$T/$PASSCODE_REL" ]
    # The marker cannot be written (here a directory is in its way; on the Kindle, a full or
    # read-only userstore): no marker, no pause, since a restart could not tell it was paused.
    tree passcode-no-marker
    sh "$DOC" --unpack-only
    fake
    mkdir -p "$US/grimoire/.passcode-paused"
    : >"$T/$PASSCODE_REL"
    : >"$SHIM_LOG"
    GRIMOIRE_WD_TICK=0.2 sh "$APP/grimoire.sh" 2>"$T/stderr"
    check "$s passcode, no marker: not paused" has "$SHIM_LOG" "grimoire passcode note= flag=on"
    check "$s passcode, no marker: the passcode stays" [ -e "$T/$PASSCODE_REL" ]
    check "$s passcode, no marker: logged" has "$LOGF" "grimoire.sh: device passcode: not paused (could not write"
    check "$s passcode, no marker: Grimoire still runs" has "$SHIM_LOG" "grimoire exits"
    check "$s passcode, no marker: nothing on stderr" [ ! -s "$T/stderr" ]
    # Turned back on since by the user (marker and passcode both there): an ordinary pause.
    tree passcode-turned-on-again
    sh "$DOC" --unpack-only
    fake
    : >"$US/grimoire/.passcode-paused"
    : >"$T/$PASSCODE_REL"
    : >"$SHIM_LOG"
    GRIMOIRE_WD_TICK=0.2 sh "$APP/grimoire.sh"
    check "$s passcode turned on again: an ordinary pause" has "$SHIM_LOG" "grimoire passcode note=paused flag=off"
    check "$s passcode turned on again: back on after" [ -e "$T/$PASSCODE_REL" ]
    check "$s passcode turned on again: no left-off line" lacks "$LOGF" "left off by a session"

    # Stop-framework mode: lab126_gui stopped and started; locked at exit only shows in the log.
    tree stopfw
    sh "$DOC" --unpack-only
    fake
    : >"$US/grimoire/stop-framework"
    : >"$T/$PASSCODE_REL"
    : >"$SHIM_LOG"
    FAKE_EXIT_STATE=readyToSuspend GRIMOIRE_WD_TICK=0.2 sh "$APP/grimoire.sh"
    check "$s stop-framework: lab126_gui stopped, then started" in_order "stop lab126_gui" "$STARTED" "start lab126_gui"
    check "$s stop-framework: the passcode paused meanwhile" has "$SHIM_LOG" "grimoire passcode note=paused flag=off"
    check "$s stop-framework: the passcode back before the framework starts" has "$SHIM_LOG" "start lab126_gui (passcode on)"
    check "$s stop-framework: pillow and awesome left alone" sh -c '! grep -qE "disableEnablePillow|killall -(STOP|CONT) awesome" "$1"' sh "$SHIM_LOG"
    check "$s stop-framework: power state at exit logged" has "$LOGF" "grimoire.sh: power state at exit: readyToSuspend"

    # The watchdog: no first frame, or a heartbeat that stops. A stuck Grimoire's TERM handler
    # only sets its quit flag, so these fakes note the TERM and run on: the watchdog has to send
    # TERM, then KILL (exit status 137), and the Kindle UI comes back after. The launcher runs in
    # the background, bounded, so a watchdog that never kills fails in seconds.
    for mode in hang stale; do
        tree "watchdog-$mode"
        sh "$DOC" --unpack-only
        fake
        : >"$SHIM_LOG"
        : >"$T/$PASSCODE_REL"
        if [ "$mode" = hang ]; then
            why="no first frame after 1s, killing Grimoire"
            FAKE_MODE=hang GRIMOIRE_WD_TICK=0.2 GRIMOIRE_WD_FIRST=1 sh "$APP/grimoire.sh" 2>"$T/stderr" &
        else
            why="no heartbeat for"
            FAKE_MODE=stale GRIMOIRE_WD_TICK=0.2 GRIMOIRE_WD_STALE=1 sh "$APP/grimoire.sh" 2>"$T/stderr" &
        fi
        l="$s watchdog ($mode)"
        check "$l: the launcher finishes" finish $! 150
        check "$l: returns 0 ($rc)" is "$rc" 0
        check "$l: logged" has "$LOGF" "grimoire.sh: watchdog: $why"
        check "$l: TERM first" has "$SHIM_LOG" "grimoire caught TERM, runs on"
        check "$l: then KILL" has "$LOGF" "grimoire.sh: Grimoire exited with status 137"
        check "$l: Grimoire is gone" fake_gone
        check "$l: the Kindle UI restored after" in_order "$STARTED" "$PILLOW_ON" "$HOME_UP"
        check "$l: the passcode back on" [ -e "$T/$PASSCODE_REL" ]
    done

    # From Quire: the data moves once, never under a refused start or while the old app runs.
    tree quire
    mkdir -p "$US/quire/notebooks/n1"
    echo "name=Old" >"$US/quire/notebooks/n1/notebook.ini"
    echo "old log line" >"$US/quire/quire.log"
    printf '#!/bin/sh\nsh /mnt/us/quire/app/quire.sh\n' >"$US/documents/Quire.sh"
    KOREADER_DIR=$US/koreader sh "$DOC" 2>/dev/null
    rc=$?
    check "$s Quire: a refused start moves nothing ($rc)" sh -c '[ "$1" = 3 ] && [ -d "$2/quire" ] && [ ! -e "$2/grimoire" ]' sh "$rc" "$US"
    echo $$ >"$T/tmp/quire.launcher.pid"
    sh "$DOC" --unpack-only 2>"$T/stderr"
    rc=$?
    check "$s Quire: --unpack-only while the old app runs: 2, nothing done ($rc)" \
        sh -c '[ "$1" = 2 ] && [ -d "$2/quire" ] && [ ! -e "$2/grimoire" ]' sh "$rc" "$US"
    check "$s Quire: --unpack-only says why on stderr" has "$T/stderr" "the old Quire app is still running"
    sh "$DOC" --reset-calibration 2>/dev/null
    rc=$?
    check "$s Quire: --reset-calibration while the old app runs: 2, nothing done ($rc)" \
        sh -c '[ "$1" = 2 ] && [ -d "$2/quire" ] && [ ! -e "$2/grimoire" ]' sh "$rc" "$US"
    sh "$DOC"
    rc=$?
    check "$s Quire: a start while the old app runs does nothing ($rc)" \
        sh -c '[ "$1" = 0 ] && [ -d "$2/quire" ] && [ ! -e "$2/grimoire" ]' sh "$rc" "$US"
    check "$s Quire: and launches nothing" lacks "$SHIM_LOG" "setsid"
    rm -f "$T/tmp/quire.launcher.pid"
    sh "$DOC" --unpack-only
    check "$s Quire: the notebooks move over" sh -c '[ -f "$1/grimoire/notebooks/n1/notebook.ini" ] && [ ! -e "$1/quire" ]' sh "$US"
    check "$s Quire: its log goes on in grimoire.log" is "$(head -n 1 "$LOGF" 2>/dev/null)" "old log line"
    check "$s Quire: the old launcher is retired" [ ! -e "$US/documents/Quire.sh" ]
    check "$s Quire: then the app is unpacked" [ -f "$APP/grimoire" ]

    # Hosted inside KOReader (--hosted koreader, from KOReader's Grimoire plugin, in KOReader's exit
    # path): koreader.sh paused the Kindle UI and the passcode and puts them back itself, so only
    # Grimoire's own share happens, in the foreground. Left here to show they stay untouched: a
    # passcode flag (KOReader would have taken it away), a pause marker from an earlier session,
    # and Grimoire's own stop-framework switch.
    hosted_tree hosted
    : >"$T/$PASSCODE_REL"
    : >"$US/grimoire/.passcode-paused"
    : >"$US/grimoire/stop-framework"
    : >"$T/sys/etc/upstart/chromebar.conf"
    mkdir -p "$SHIM_STATE/not-running" && : >"$SHIM_STATE/not-running/chromebar"
    echo 1 >"$SHIM_STATE/lipc/com.lab126.powerd.preventScreenSaver"
    # KOReader still has ordinary files open (its log, a pipe), not the screen or the input.
    ko_paused FAKE_KO_HOLDS="/mnt/us/koreader/crash.log pipe:[4021]" fake-koreader \
        sh "$APP/grimoire.sh" --hosted koreader 2>"$T/stderr"
    rc=$?
    l="$s hosted"
    check "$l: returns 0 ($rc)" is "$rc" 0
    check "$l: in the foreground (Grimoire ended before it returned)" has "$SHIM_LOG" "grimoire exits"
    check "$l: logged as hosted" has "$LOGF" "grimoire.sh: starting Grimoire hosted by KOReader (reader.lua pid"
    check "$l: Grimoire told its host (GRIMOIRE_HOST)" has "$SHIM_LOG" "grimoire host=koreader"
    check "$l: Grimoire never wakes awesome" has "$SHIM_LOG" "$STARTED$US/grimoire awesome_stopped=no"
    check "$l: awesome stopped again (AWESOME_STOPPED=yes)" in_order "killall -STOP awesome" "$STARTED"
    check "$l: awesome never continued" lacks "$SHIM_LOG" "killall -CONT awesome"
    check "$l: pillow left alone" lacks "$SHIM_LOG" "disableEnablePillow"
    check "$l: no home booklet" lacks "$SHIM_LOG" "com.lab126.appmgrd"
    check "$l: volumd left alone" lacks "$SHIM_LOG" "volumd"
    check "$l: the framework left alone" lacks "$SHIM_LOG" "lab126_gui"
    check "$l: no screen dump, none put back" sh -c '[ "$(cat "$1")" = GRIMOIRE ] && [ ! -e "$2" ]' sh \
        "$T/sys/dev/fb0" "$T/sys/var/tmp/grimoire-fb.dump"
    check "$l: the passcode flag left alone" [ -e "$T/$PASSCODE_REL" ]
    check "$l: the passcode marker left alone" [ -e "$US/grimoire/.passcode-paused" ]
    check "$l: no passcode note for Grimoire" has "$SHIM_LOG" "grimoire passcode note= flag=on"
    check "$l: the passcode left to KOReader (logged)" has "$LOGF" "grimoire.sh: device passcode: left to KOReader"
    check "$l: the status bar job stopped, then started" in_order "stop statusbar" "$STARTED" "start statusbar"
    check "$l: a status job that was not running is not started" sh -c \
        'grep -qxF "stop chromebar" "$1" && ! grep -qF "start chromebar" "$1"' sh "$SHIM_LOG"
    check "$l: firewall hole opened, then closed" in_order "iptables -A INPUT -i wlan0 -p tcp --dport 8765 -j ACCEPT" \
        "$STARTED" "iptables -D INPUT -i wlan0 -p tcp --dport 8765 -j ACCEPT"
    check "$l: preventScreenSaver read, then on" in_order "lipc-get-prop -i com.lab126.powerd preventScreenSaver" \
        "lipc-set-prop com.lab126.powerd preventScreenSaver 1" "$STARTED"
    check "$l: KOReader's preventScreenSaver (1) put back" after_start "lipc-set-prop com.lab126.powerd preventScreenSaver 1"
    check "$l: preventScreenSaver never turned off" lacks "$SHIM_LOG" "preventScreenSaver 0"
    check "$l: no settle wait" lacks "$SHIM_LOG" "usleep 900000"
    check "$l: Grimoire's stop-framework switch ignored (logged)" has "$LOGF" \
        "grimoire.sh: the stop-framework flag does not apply inside KOReader"
    check "$l: hands the screen back (logged)" has "$LOGF" "grimoire.sh: handing the screen back to KOReader"
    check "$l: exit status logged" has "$LOGF" "grimoire.sh: Grimoire exited with status 0"
    check "$l: the lock released" [ ! -e "$GRIMOIRE_TMP/grimoire.launcher.pid" ]
    check "$l: nothing on stderr" [ ! -s "$T/stderr" ]
    check "$l: KOReader's plugin not installed by a hosted run" [ ! -e "$US/$PLUGIN_REL" ]
    # KOReader had preventScreenSaver off, or it cannot be read: off again after.
    for prevent in 0 unreadable; do
        hosted_tree "hosted-prevent-$prevent"
        [ "$prevent" = 0 ] && echo 0 >"$SHIM_STATE/lipc/com.lab126.powerd.preventScreenSaver"
        ko_paused fake-koreader sh "$APP/grimoire.sh" --hosted koreader
        check "$s hosted, preventScreenSaver $prevent: off again after" after_start "lipc-set-prop com.lab126.powerd preventScreenSaver 0"
    done
    # KOReader left awesome running (only pillow off): it stays so.
    hosted_tree hosted-awesome-running
    KOREADER_DIR=$US/koreader STOP_FRAMEWORK=no AWESOME_STOPPED=no PILLOW_HARD_DISABLED=yes GRIMOIRE_WD_TICK=0.2 \
        fake-koreader sh "$APP/grimoire.sh" --hosted koreader
    rc=$?
    check "$s hosted, awesome running: Grimoire ran ($rc)" sh -c '[ "$1" = 0 ] && grep -qF "grimoire exits" "$2"' sh "$rc" "$SHIM_LOG"
    check "$s hosted, awesome running: left alone" lacks "$SHIM_LOG" "killall"
    # KOReader stopped the framework (KUAL's no-framework start): fine, with a note in the log.
    hosted_tree hosted-framework-stopped
    KOREADER_DIR=$US/koreader STOP_FRAMEWORK=yes AWESOME_STOPPED=no GRIMOIRE_WD_TICK=0.2 \
        fake-koreader sh "$APP/grimoire.sh" --hosted koreader
    rc=$?
    check "$s hosted, framework stopped: Grimoire ran ($rc)" sh -c '[ "$1" = 0 ] && grep -qF "grimoire exits" "$2"' sh "$rc" "$SHIM_LOG"
    check "$s hosted, framework stopped: lab126_gui left alone" lacks "$SHIM_LOG" "lab126_gui"
    check "$s hosted, framework stopped: the power button note" has "$LOGF" \
        "grimoire.sh: KOReader stopped the Kindle framework: the power button may not put the Kindle to sleep in Grimoire"
    # Refusals (exit 3, logged, on stderr, nothing touched): no KOREADER_DIR; reader.lua started by
    # something else than koreader.sh; no reader.lua among the parents; a reader.lua that still
    # holds an input device (run from KOReader's terminal, say, not its exit path); a KOReader that
    # paused nothing (a KPVBooklet start); no host, or one Grimoire does not know.
    hosted_tree hosted-refused
    for why in no-koreader-dir wrong-parent no-reader-lua holds-input not-paused no-host other-host; do
        : >"$SHIM_LOG"
        : >"$T/stderr"
        case $why in
        no-koreader-dir)
            STOP_FRAMEWORK=no AWESOME_STOPPED=yes PILLOW_HARD_DISABLED=yes fake-koreader \
                sh "$APP/grimoire.sh" --hosted koreader 2>"$T/stderr"
            rc=$?
            text="not starting: --hosted koreader runs only inside KOReader, from its Grimoire plugin."
            ;;
        wrong-parent)
            ko_paused FAKE_KO_PARENT="/bin/sh /mnt/us/extensions/other/run.sh" fake-koreader \
                sh "$APP/grimoire.sh" --hosted koreader 2>"$T/stderr"
            rc=$?
            text="not starting hosted: KOReader's reader.lua (pid"
            ;;
        no-reader-lua)
            KOREADER_DIR=$US/koreader AWESOME_STOPPED=yes PILLOW_HARD_DISABLED=yes \
                sh "$APP/grimoire.sh" --hosted koreader 2>"$T/stderr"
            rc=$?
            text="not starting hosted: KOReader's reader.lua is not among this launcher's parents."
            ;;
        holds-input)
            ko_paused FAKE_KO_HOLDS="/dev/fb0 /dev/input/event1" fake-koreader \
                sh "$APP/grimoire.sh" --hosted koreader 2>"$T/stderr"
            rc=$?
            text="not starting hosted: KOReader still has /dev/input/event1 open (this is not its exit path), so Grimoire would get no pen or touch."
            ;;
        not-paused)
            KOREADER_DIR=$US/koreader STOP_FRAMEWORK=no AWESOME_STOPPED=no fake-koreader \
                sh "$APP/grimoire.sh" --hosted koreader 2>"$T/stderr"
            rc=$?
            text="not starting hosted: KOReader left the Kindle UI running (koreader.sh paused nothing)."
            ;;
        no-host)
            ko_paused fake-koreader sh "$APP/grimoire.sh" --hosted 2>"$T/stderr"
            rc=$?
            text="not starting: --hosted without a name is no host Grimoire knows"
            ;;
        other-host)
            ko_paused fake-koreader sh "$APP/grimoire.sh" --hosted kobo 2>"$T/stderr"
            rc=$?
            text="not starting: --hosted kobo is no host Grimoire knows"
            ;;
        esac
        l="$s hosted refused ($why)"
        check "$l: exit 3 ($rc)" is "$rc" 3
        check "$l: logged" has "$LOGF" "grimoire.sh: $text"
        check "$l: on stderr" has "$T/stderr" "Grimoire: $text"
        check "$l: nothing started or touched" sh -c '! grep -qE "^grimoire |lipc-|killall|iptables|^stop |^start " "$1"' sh "$SHIM_LOG"
        check "$l: no lock taken" [ ! -e "$GRIMOIRE_TMP/grimoire.launcher.pid" ]
    done
    check "$s hosted refused (wrong-parent): names reader.lua's pid" has "$LOGF" "was not started by koreader.sh"
    # documents/Grimoire.sh --hosted: inside KOReader it runs app/grimoire.sh --hosted in the
    # foreground (never detached, no plugin install); outside KOReader it refuses.
    hosted_tree doc-hosted
    ko_paused fake-koreader sh "$DOC" --hosted koreader 2>"$T/stderr"
    rc=$?
    l="$s Grimoire.sh --hosted inside KOReader"
    check "$l: returns 0 ($rc)" is "$rc" 0
    check "$l: Grimoire ran hosted" has "$SHIM_LOG" "grimoire host=koreader"
    check "$l: in the foreground (Grimoire ended before it returned)" has "$SHIM_LOG" "grimoire exits"
    check "$l: not detached" lacks "$SHIM_LOG" "setsid"
    check "$l: KOReader's plugin not installed" [ ! -e "$US/$PLUGIN_REL" ]
    : >"$SHIM_LOG"
    sh "$DOC" --hosted koreader 2>"$T/stderr"
    rc=$?
    l="$s Grimoire.sh --hosted outside KOReader"
    check "$l: exit 3 ($rc)" is "$rc" 3
    check "$l: logged" has "$LOGF" "Grimoire.sh: not starting: --hosted is for KOReader's Grimoire plugin, inside KOReader."
    check "$l: on stderr" has "$T/stderr" "Grimoire: not starting: --hosted is for KOReader's Grimoire plugin"
    check "$l: nothing started" sh -c '! grep -qE "setsid|^grimoire |lipc-set-prop" "$1"' sh "$SHIM_LOG"

    # KOReader's Grimoire plugin: a start from the Library installs it into KOReader once, keeps it
    # up to date, and never brings back one the user deleted.
    tree koplugin
    sh "$DOC" --unpack-only
    fake
    MARK=$US/grimoire/.koplugin-installed
    start_doc
    check "$s plugin, no KOReader: nothing installed" sh -c '[ ! -e "$1/koreader" ] && [ ! -e "$2" ]' sh "$US" "$MARK"
    mkdir -p "$US/koreader/plugins"
    : >"$US/koreader/reader.lua"
    sh "$DOC" --unpack-only
    check "$s plugin: --unpack-only installs nothing" [ ! -e "$US/$PLUGIN_REL" ]
    start_doc
    check "$s plugin: installed at a start from the Library" plugin_is_payload
    check "$s plugin: the marker holds the build" is "$(content "$MARK")" "$BUILD_ID"
    check "$s plugin: installed (logged)" has "$LOGF" "Grimoire.sh: installed KOReader's Grimoire plugin into $US/$PLUGIN_REL"
    check "$s plugin: no copy left half-done" sh -c '! ls -A "$1" | grep -q "tmp$"' sh "$US/$PLUGIN_REL"
    check "$s plugin: Grimoire still starts" has "$SHIM_LOG" "grimoire exits"
    start_doc
    check "$s plugin, the same build: nothing to do (no log line)" lacks "$LOGF" "Grimoire plugin"
    echo "-- an older plugin" >"$US/$PLUGIN_REL/main.lua"
    echo 000000000000 >"$MARK"
    start_doc
    check "$s plugin, a newer build: updated in place" plugin_is_payload
    check "$s plugin, a newer build: updated (logged)" has "$LOGF" "Grimoire.sh: updated KOReader's Grimoire plugin in $US/$PLUGIN_REL"
    check "$s plugin, a newer build: the marker holds it" is "$(content "$MARK")" "$BUILD_ID"
    rm -rf "${US:?}/$PLUGIN_REL"
    echo 000000000000 >"$MARK"
    start_doc
    check "$s plugin removed by the user: not brought back" [ ! -e "$US/$PLUGIN_REL" ]
    check "$s plugin removed: logged" has "$LOGF" \
        "Grimoire.sh: KOReader's Grimoire plugin was removed from $US/$PLUGIN_REL: not installing it again"
    check "$s plugin removed: the marker says so" is "$(content "$MARK")" removed
    start_doc
    check "$s plugin removed: logged once only" lacks "$LOGF" "Grimoire plugin"
    check "$s plugin removed: still not brought back" [ ! -e "$US/$PLUGIN_REL" ]
    mkdir -p "$US/$PLUGIN_REL"
    echo "-- an older plugin" >"$US/$PLUGIN_REL/main.lua"
    start_doc
    check "$s plugin copied back by hand: kept up to date again" plugin_is_payload
    check "$s plugin copied back by hand: the marker holds the build" is "$(content "$MARK")" "$BUILD_ID"
    # Copied in by hand before Grimoire ever started (no marker yet): brought up to date.
    tree koplugin-by-hand
    sh "$DOC" --unpack-only
    fake
    mkdir -p "$US/$PLUGIN_REL"
    : >"$US/koreader/reader.lua"
    echo "-- an older plugin" >"$US/$PLUGIN_REL/main.lua"
    start_doc
    check "$s plugin there before the first start: brought up to date" plugin_is_payload
    check "$s plugin there before the first start: the marker holds the build" is \
        "$(content "$US/grimoire/.koplugin-installed")" "$BUILD_ID"

    # The run with the device's timings, started first.
    enter timings
    check "$s device timings: the launcher finishes" finish "$timings_pid" 250
    check "$s device timings: returns 0 ($rc)" is "$rc" 0
    check "$s device timings: a first frame after 3 s is not killed" lacks "$LOGF" "watchdog:"
    check "$s device timings: Grimoire runs to its end" has "$SHIM_LOG" "grimoire exits"
    check "$s device timings: exit status 0" has "$LOGF" "grimoire.sh: Grimoire exited with status 0"
}

# Each shell gets its own scratch Kindles and runs in the background; its report is printed once
# it is done.
ORIG_PATH=$PATH
runs=""
for SH_NAME in sh dash busybox; do
    SH_BIN=$(command -v "$SH_NAME" 2>/dev/null)
    if [ -z "$SH_BIN" ]; then
        echo "skip the launchers under $SH_NAME (not installed)"
        continue
    fi
    # `sh` inside the launchers is this shell (busybox runs as sh when called so).
    mkdir -p "$W/$SH_NAME/bin"
    ln -s "$SH_BIN" "$W/$SH_NAME/bin/sh"
    (
        PATH=$W/$SH_NAME/bin:$SHIMS:$ORIG_PATH
        export PATH
        suite
        exit $fail
    ) >"$W/$SH_NAME.out" 2>&1 &
    runs="$runs $!:$SH_NAME"
done
for r in $runs; do
    wait "${r%%:*}" || fail=1
    cat "$W/${r#*:}.out"
done

# The scratch Kindles take room (an app in each): kept only when something failed.
if [ $fail = 0 ]; then
    rm -rf "$W"
    echo "LAUNCHER CHECK PASSED"
else
    echo "LAUNCHER CHECK FAILED (see $W)"
    exit 1
fi
