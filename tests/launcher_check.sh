#!/bin/sh
# Runs the Kindle launchers from the zip (make package) on this computer: documents/Grimoire.sh
# and the app/grimoire.sh it unpacks, in a scratch device tree, with the Kindle's tools replaced
# by stand-ins that record every call (tests/shims) and the binary by a shell script
# (tests/shims/fake-grimoire). Checks unpacking, --unpack-only, the KOReader guard,
# --reset-calibration, the order of the pause and the restore, the restore on a locked Kindle,
# the watchdog and its device timings, the move from Quire, the hosted run inside KOReader
# (--hosted koreader, under tests/shims/fake-koreader) and its refusals, the lock another Grimoire
# holds, and the install of KOReader's Grimoire plugin, once with each POSIX shell found (sh, dash,
# busybox sh); the shells run side by side.
# usage: tests/launcher_check.sh [build/dist/Grimoire.zip]
set -u
cd "$(dirname "$0")/.." || exit 1
REPO=$(pwd)
ZIP=${1:-build/dist/Grimoire.zip}
case "$ZIP" in /*) ;; *) ZIP=$REPO/$ZIP ;; esac
W=$(cd "$(dirname "$ZIP")/.." && pwd)/launcher-check
SHIMS=$REPO/tests/shims
export LC_ALL=C  # Grimoire.sh ends in a binary archive
unset KOREADER_DIR FAKE_MODE FAKE_EXIT_STATE SHIM_HOLD SHIM_NICE GRIMOIRE_WD_TICK GRIMOIRE_WD_FIRST GRIMOIRE_WD_STALE GRIMOIRE_KPM \
    STOP_FRAMEWORK AWESOME_STOPPED PILLOW_HARD_DISABLED FAKE_KO_PARENT FAKE_KO_HOLDS GRIMOIRE_ANNOTATE
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
for f in kindle/Grimoire.sh kindle/app/grimoire.sh kindle/app/bluetooth.sh kindle/app/wifi.sh kindle/app/update.sh "kindle/Grimoire - reset pen calibration.sh" \
    kindle/kpm/*.sh tests/launcher_check.sh tests/package_check.sh tests/koplugin_check.sh tests/shims/*; do
    check "sh -n $f" sh -n "$f"
    if command -v dash >/dev/null 2>&1; then check "dash -n $f" dash -n "$f"; fi
done
if command -v shellcheck >/dev/null 2>&1; then
    check "shellcheck" shellcheck -s sh -S warning kindle/Grimoire.sh kindle/app/grimoire.sh kindle/app/bluetooth.sh kindle/app/wifi.sh kindle/app/update.sh \
        "kindle/Grimoire - reset pen calibration.sh" kindle/kpm/*.sh tests/launcher_check.sh tests/package_check.sh \
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
# a reader.lua that koreader.sh started, running its own command the way os.execute does, and
# that pidof finds meanwhile, as on the Kindle.
hosted_tree() {  # hosted_tree NAME
    tree "$1"
    sh "$DOC" --unpack-only
    fake
    mkdir -p "$US/koreader"
    : >"$US/koreader/reader.lua"
    : >"$SHIM_LOG"
}
ko_paused() {
    env KOREADER_DIR="$US/koreader" STOP_FRAMEWORK=no AWESOME_STOPPED=yes PILLOW_HARD_DISABLED=yes LC_ALL=en_US.UTF-8 \
        GRIMOIRE_WD_TICK=0.2 "$@"
}
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

    # An existing RC1 cache retains files absent from the new tar. Prune only the known retired
    # script, including when the current build is already unpacked; keep unknown local edits.
    tree retired-dpi
    sh "$DOC" --unpack-only
    retired=$APP/koreader/patches/1-grimoire-scribe-dpi.lua
    mkdir -p "$(dirname "$retired")"
    cp "$REPO/tests/fixtures/rc1-dpi-cache.txt" "$retired"
    sh "$DOC" --unpack-only
    check "$s retired DPI cache: exact RC1 file removed" [ ! -e "$retired" ]
    echo '-- local modification' > "$retired"
    sh "$DOC" --unpack-only
    check "$s retired DPI cache: local edit kept" has "$retired" '-- local modification'

    # Settings > Updates' Update now: Grimoire asks (the request file the launcher named) and leaves.
    # The launcher restores the Kindle UI, then hands the update to a copy of update.sh, which waits
    # for it to be gone (KPM's hooks refuse while it runs), installs with KPM, and starts Grimoire again.
    tree update
    sh "$DOC" --unpack-only
    fake
    : >"$SHIM_LOG"
    FAKE_MODE=update GRIMOIRE_KPM="$SHIMS/fake-kpm" GRIMOIRE_WD_TICK=0.2 sh "$APP/grimoire.sh" &
    check "$s update: the launcher finishes" finish $! 150
    check "$s update: Grimoire was told where update.sh is" has "$SHIM_LOG" "grimoire update script=$APP/update.sh"
    check "$s update: the launcher hands it on" has "$LOGF" "an update was asked for"
    # (Polled with has: in_order and wait_until share their counter.)
    check "$s update: KPM is done" wait_until 100 has "$SHIM_LOG" "kpm launch grimoire"
    check "$s update: KPM installs once the launcher is gone, then starts Grimoire" \
        in_order "kpm --fbink -y install grimoire (launcher gone)" "kpm launch grimoire (launcher gone)"
    check "$s update: the request is used up" [ ! -e "$GRIMOIRE_TMP/grimoire-update" ]
    check "$s update: nothing failed" [ ! -e "$US/grimoire/.update-failed" ]
    # A start with no request starts no update, even with an old request left behind.
    : >"$SHIM_LOG"
    echo stable >"$GRIMOIRE_TMP/grimoire-update"
    GRIMOIRE_KPM="$SHIMS/fake-kpm" GRIMOIRE_WD_TICK=0.2 sh "$APP/grimoire.sh" &
    check "$s update: a plain run finishes" finish $! 150
    sleep 0.5
    check "$s update: an old request starts nothing" lacks "$SHIM_LOG" "kpm "

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
    check "$s first run passes the locale on as it is" has "$SHIM_LOG" "grimoire lc_all=C "
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

    # Updating a scriptlet while Grimoire runs must not unpack over the live app or reset its
    # settings. This check precedes the inner launcher's single-instance lock.
    tree live-update
    sh "$DOC" --unpack-only
    fake
    cp "$APP/grimoire" "$T/old-binary"
    echo old-build >"$APP/BUILD_ID"
    printf 'pen.valid=1\ntouch.valid=1\ntheme=gothic\n' >"$US/grimoire/settings.ini"
    cp "$US/grimoire/settings.ini" "$T/old-settings"
    echo $$ >"$GRIMOIRE_TMP/grimoire.launcher.pid"
    for flag in '' --unpack-only --reset-calibration --hosted; do
        want=2; [ -z "$flag" ] && want=0
        # Hosted needs its ordinary KOReader environment before the live-app guard.
        if [ "$flag" = --hosted ]; then
            KOREADER_DIR=$US/koreader sh "$DOC" --hosted koreader 2>"$T/stderr"
        elif [ -z "$flag" ]; then
            KOREADER_DIR='' sh "$DOC" 2>"$T/stderr"
        else
            KOREADER_DIR='' sh "$DOC" "$flag" 2>"$T/stderr"
        fi
        rc=$?
        check "$s live update ($flag): refused without unpacking ($rc)" is "$rc" "$want"
        check "$s live update ($flag): binary kept" cmp -s "$APP/grimoire" "$T/old-binary"
        check "$s live update ($flag): old build kept" is "$(content "$APP/BUILD_ID")" old-build
    done
    for launcher in "$APP/grimoire.sh" "$RESET"; do
        sh "$launcher" --reset-calibration >"$T/stdout" 2>"$T/stderr"
        rc=$?
        check "$s live calibration reset: refused ($rc)" is "$rc" 2
    done
    check "$s live update/reset: settings kept" cmp -s "$US/grimoire/settings.ini" "$T/old-settings"
    check "$s live update/reset: lock kept" is "$(content "$GRIMOIRE_TMP/grimoire.launcher.pid")" "$$"
    echo invalid >"$GRIMOIRE_TMP/grimoire.launcher.pid"
    sh "$DOC" --unpack-only
    check "$s invalid old lock: update allowed" is "$(content "$APP/BUILD_ID")" "$BUILD_ID"

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

    # PIN remains enabled for the entire session. Older beta crash notices still migrate safely.
    tree passcode
    sh "$DOC" --unpack-only
    fake
    : >"$T/$PASSCODE_REL"
    : >"$SHIM_LOG"
    GRIMOIRE_WD_TICK=0.2 sh "$APP/grimoire.sh"
    check "$s passcode: native unlock requested, flag stays on" has "$SHIM_LOG" "grimoire passcode note=enabled flag=on"
    check "$s passcode: still on after" [ -e "$T/$PASSCODE_REL" ]
    check "$s passcode: no pause marker" [ ! -e "$US/grimoire/.passcode-paused" ]
    check "$s passcode: on when home resumes" has "$SHIM_LOG" "$HOME_UP (passcode on)"
    check "$s passcode: never paused" lacks "$LOGF" "device passcode: paused"
    : >"$US/grimoire/stop-framework"
    : >"$SHIM_LOG"
    GRIMOIRE_WD_TICK=0.2 sh "$APP/grimoire.sh"
    check "$s PIN plus stop-framework: framework stays available" lacks "$SHIM_LOG" "stop lab126_gui"
    check "$s PIN plus stop-framework: PIN never disabled" has "$SHIM_LOG" "grimoire passcode note=enabled flag=on"
    tree passcode-left-off
    sh "$DOC" --unpack-only
    fake
    : >"$US/grimoire/.passcode-paused"
    GRIMOIRE_WD_TICK=0.2 sh "$APP/grimoire.sh"
    check "$s old beta crash: left-off notice" has "$SHIM_LOG" "grimoire passcode note=left-off flag=off"
    check "$s old beta crash: no unknown PIN flag created" [ ! -e "$T/$PASSCODE_REL" ]
    check "$s old beta crash: marker consumed" [ ! -e "$US/grimoire/.passcode-paused" ]
    tree passcode-locked-start
    sh "$DOC" --unpack-only
    fake
    : >"$T/$PASSCODE_REL"
    echo screenSaver >"$SHIM_STATE/lipc/com.lab126.powerd.state"
    GRIMOIRE_WD_TICK=0.2 sh "$APP/grimoire.sh"
    rc=$?
    check "$s locked start: refuses until native unlock ($rc)" is "$rc" 3
    check "$s locked start: never opens the app" lacks "$SHIM_LOG" "$STARTED"
    check "$s locked start: PIN stays enabled" [ -e "$T/$PASSCODE_REL" ]

    # Stop-framework mode: lab126_gui stopped and started; locked at exit only shows in the log.
    tree stopfw
    sh "$DOC" --unpack-only
    fake
    : >"$US/grimoire/stop-framework"
    : >"$SHIM_LOG"
    FAKE_EXIT_STATE=readyToSuspend GRIMOIRE_WD_TICK=0.2 sh "$APP/grimoire.sh"
    check "$s stop-framework: lab126_gui stopped, then started" in_order "stop lab126_gui" "$STARTED" "start lab126_gui"
    check "$s stop-framework: no PIN present" has "$SHIM_LOG" "grimoire passcode note= flag=off"
    check "$s stop-framework: the passcode back before the framework starts" has "$SHIM_LOG" "start lab126_gui (passcode off)"
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
        check "$l: reports the failure ($rc)" is "$rc" 137
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
    # KOReader still has ordinary files open (its log, a pipe), not the screen or the input. LANG
    # comes down from the Kindle UI, as it does to a start from the Library.
    ko_paused LANG=es_ES.UTF-8 FAKE_KO_HOLDS="/mnt/us/koreader/crash.log pipe:[4021]" fake-koreader \
        sh "$APP/grimoire.sh" --hosted koreader 2>"$T/stderr"
    rc=$?
    l="$s hosted"
    check "$l: returns 0 ($rc)" is "$rc" 0
    check "$l: in the foreground (Grimoire ended before it returned)" has "$SHIM_LOG" "grimoire exits"
    check "$l: logged as hosted" has "$LOGF" "grimoire.sh: starting Grimoire hosted by KOReader (reader.lua pid"
    check "$l: Grimoire told its host (GRIMOIRE_HOST)" has "$SHIM_LOG" "grimoire host=koreader"
    check "$l: KOReader's LC_ALL kept from Grimoire, LANG carried on" has "$SHIM_LOG" \
        "grimoire lc_all=(unset) lang=es_ES.UTF-8"
    check "$l: native PIN can resume the host UI" has "$SHIM_LOG" "$STARTED$US/grimoire awesome_stopped=yes"
    check "$l: awesome stopped again (AWESOME_STOPPED=yes)" in_order "killall -STOP awesome" "$STARTED"
    check "$l: awesome stopped again first, before the lipc calls" in_order "killall -STOP awesome" \
        "lipc-get-prop -s com.lab126.powerd state"
    check "$l: awesome stopped again (logged)" has "$LOGF" "grimoire.sh: awesome stopped again, as KOReader had it"
    check "$l: awesome never continued" lacks "$SHIM_LOG" "killall -CONT awesome"
    check "$l: pillow returns to the host pause" has "$SHIM_LOG" "disableEnablePillow disable"
    check "$l: no home booklet" lacks "$SHIM_LOG" "com.lab126.appmgrd"
    check "$l: volumd left alone" lacks "$SHIM_LOG" "volumd"
    check "$l: the framework left alone" lacks "$SHIM_LOG" "lab126_gui"
    check "$l: no screen dump, none put back" sh -c '[ "$(cat "$1")" = GRIMOIRE ] && [ ! -e "$2" ]' sh \
        "$T/sys/dev/fb0" "$T/sys/var/tmp/grimoire-fb.dump"
    check "$l: the passcode flag left alone" [ -e "$T/$PASSCODE_REL" ]
    check "$l: the passcode marker left alone" [ -e "$US/grimoire/.passcode-paused" ]
    check "$l: no passcode note for Grimoire" has "$SHIM_LOG" "grimoire passcode note=enabled flag=on"
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
    check "$l: no page to annotate passed on (Open Grimoire)" sh -c 'grep -qxF "grimoire annotate=" "$1"' sh "$SHIM_LOG"
    # KOReader's Annotate this page in Grimoire: --annotate after --hosted koreader, and the binary
    # is asked to open the page (GRIMOIRE_ANNOTATE=1), logged; app/grimoire.sh says it can.
    check "$s app/grimoire.sh's Capabilities include annotate" sh -c \
        'head -n 10 "$1" | sed -n "s/^# Capabilities: //p" | grep -qw annotate' sh "$APP/grimoire.sh"
    hosted_tree hosted-annotate
    ko_paused fake-koreader sh "$APP/grimoire.sh" --hosted koreader --annotate 2>"$T/stderr"
    rc=$?
    l="$s hosted, a page to annotate"
    check "$l: returns 0 ($rc)" is "$rc" 0
    check "$l: Grimoire ran hosted" has "$SHIM_LOG" "grimoire host=koreader"
    check "$l: asked to open the page (GRIMOIRE_ANNOTATE=1)" sh -c 'grep -qxF "grimoire annotate=1" "$1"' sh "$SHIM_LOG"
    check "$l: logged" has "$LOGF" "grimoire.sh: KOReader sent a page to annotate (grimoire/import/.annotate)"
    check "$l: nothing on stderr" [ ! -s "$T/stderr" ]
    check "$l: no book passed on" sh -c 'grep -qxF "grimoire book=" "$1"' sh "$SHIM_LOG"
    # Open this book in Grimoire: --book after --hosted koreader, GRIMOIRE_BOOK=1, logged.
    check "$s app/grimoire.sh's Capabilities include book" sh -c \
        'head -n 10 "$1" | sed -n "s/^# Capabilities: //p" | grep -qw book' sh "$APP/grimoire.sh"
    hosted_tree hosted-book
    ko_paused fake-koreader sh "$APP/grimoire.sh" --hosted koreader --book 2>"$T/stderr"
    rc=$?
    l="$s hosted, a whole book"
    check "$l: returns 0 ($rc)" is "$rc" 0
    check "$l: asked to open the book (GRIMOIRE_BOOK=1)" sh -c 'grep -qxF "grimoire book=1" "$1"' sh "$SHIM_LOG"
    check "$l: no page to annotate passed on" sh -c 'grep -qxF "grimoire annotate=" "$1"' sh "$SHIM_LOG"
    check "$l: logged" has "$LOGF" "grimoire.sh: KOReader sent a whole book (grimoire/import/.book)"
    check "$l: nothing on stderr" [ ! -s "$T/stderr" ]
    hosted_tree hosted-unpaused
    KOREADER_DIR=$US/koreader STOP_FRAMEWORK=no AWESOME_STOPPED=no PILLOW_HARD_DISABLED=no GRIMOIRE_WD_TICK=0.2 \
        fake-koreader sh "$APP/grimoire.sh" --hosted koreader
    rc=$?
    check "$s unpaused KOReader launcher: works ($rc)" is "$rc" 0
    check "$s unpaused KOReader launcher: own UI pause and restore" in_order "$PILLOW_OFF" "$STARTED" "$PILLOW_ON"
    check "$s unpaused KOReader launcher: no home screen before KOReader" lacks "$SHIM_LOG" "$HOME_UP"

    # Only that start: a GRIMOIRE_ANNOTATE the launcher inherits never reaches the binary, hosted
    # without --annotate or started from the Library (where --annotate means nothing either).
    hosted_tree hosted-annotate-inherited
    ko_paused GRIMOIRE_ANNOTATE=1 fake-koreader sh "$APP/grimoire.sh" --hosted koreader 2>/dev/null
    check "$s hosted without --annotate, GRIMOIRE_ANNOTATE=1 inherited: not passed on" sh -c \
        'grep -qxF "grimoire annotate=" "$1"' sh "$SHIM_LOG"
    hosted_tree hosted-book-inherited
    ko_paused GRIMOIRE_BOOK=1 fake-koreader sh "$APP/grimoire.sh" --hosted koreader 2>/dev/null
    check "$s hosted without --book, GRIMOIRE_BOOK=1 inherited: not passed on" sh -c \
        'grep -qxF "grimoire book=" "$1"' sh "$SHIM_LOG"
    hosted_tree library-annotate
    GRIMOIRE_ANNOTATE=1 GRIMOIRE_WD_TICK=0.2 sh "$APP/grimoire.sh" --annotate 2>/dev/null
    rc=$?
    l="$s from the Library with --annotate"
    check "$l: an ordinary start ($rc)" sh -c '[ "$1" = 0 ] && grep -qxF "grimoire host=" "$2" && grep -qF "$3" "$2"' sh \
        "$rc" "$SHIM_LOG" "$PILLOW_OFF"
    check "$l: no page to annotate passed on" sh -c 'grep -qxF "grimoire annotate=" "$1"' sh "$SHIM_LOG"
    check "$l: nothing logged of one" lacks "$LOGF" "sent a page to annotate"
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
    # Post-update setups can retain a virtual keyboard after releasing pen/touch. Its zero
    # absolute-axis bitmap permits the handover, but must not hide a later pen/touch fd.
    for mode in keys keys-spaces keys-pen keys-touch missing empty whitespace malformed; do
        hosted_tree "hosted-input-$mode"
        caps=$T/sys/sys/class/input
        mkdir -p "$caps/event5/device/capabilities" "$caps/event6/device/capabilities"
        echo 0 >"$caps/event5/device/capabilities/abs"
        holds=/dev/input/event5
        expected=3
        case $mode in
        keys) expected=0 ;;
        keys-spaces) echo '0 0' >"$caps/event5/device/capabilities/abs"; expected=0 ;;
        keys-pen) echo f000003 >"$caps/event6/device/capabilities/abs"; holds="$holds /dev/input/event6" ;;
        keys-touch) echo 'ee18000 0' >"$caps/event6/device/capabilities/abs"; holds="$holds /dev/input/event6" ;;
        missing) rm "$caps/event5/device/capabilities/abs" ;;
        empty) : >"$caps/event5/device/capabilities/abs" ;;
        whitespace) echo '   ' >"$caps/event5/device/capabilities/abs" ;;
        malformed) echo unknown >"$caps/event5/device/capabilities/abs" ;;
        esac
        ko_paused FAKE_KO_HOLDS="$holds" fake-koreader sh "$APP/grimoire.sh" --hosted koreader 2>"$T/stderr"
        rc=$?
        check "$s hosted $mode: status $expected ($rc)" is "$rc" "$expected"
        if [ "$expected" = 0 ]; then
            check "$s hosted $mode: Grimoire ran" has "$SHIM_LOG" "grimoire exits"
        else
            check "$s hosted $mode: no UI or input takeover" sh -c '! grep -qE "^grimoire |lipc-|killall|iptables|^stop |^start " "$1"' sh "$SHIM_LOG"
            case $mode in
            keys-pen|keys-touch) check "$s hosted $mode: retained digitizer identified" has "$LOGF" "still has /dev/input/event6 open" ;;
            esac
        fi
        check "$s hosted $mode: no lock left" [ ! -e "$GRIMOIRE_TMP/grimoire.launcher.pid" ]
    done

    # Refusals (exit 3, logged, on stderr, nothing touched): no KOREADER_DIR; reader.lua started by
    # something else than koreader.sh; no reader.lua among the parents; a reader.lua that still
    # holds an input device (run from KOReader's terminal, say, not its exit path); a KOReader that
    # paused nothing (a KPVBooklet start); no host, or one Grimoire does not know.
    hosted_tree hosted-refused
    for why in no-koreader-dir wrong-parent no-reader-lua holds-input no-host other-host; do
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
    # Another Grimoire's launcher holds the lock (alive, and grimoire.sh by its scratch /proc
    # cmdline): hosted, that is exit 1, so KOReader's plugin keeps its launch marker and says
    # Grimoire could not start; from the Library it is 0, as nothing happens. Either way Grimoire
    # does not start and the lock stays the other's. awesome, which KOReader's exit woke, is stopped
    # again even so: before the lock wait, and never continued.
    hosted_tree hosted-locked
    sleep 60 &
    holder=$!
    mkdir -p "$T/sys/proc/$holder"
    printf 'sh\000%s\000' "$APP/grimoire.sh" >"$T/sys/proc/$holder/cmdline"
    echo "$holder" >"$GRIMOIRE_TMP/grimoire.launcher.pid"
    ko_paused fake-koreader sh "$APP/grimoire.sh" --hosted koreader 2>"$T/stderr"
    rc=$?
    l="$s hosted, another Grimoire running"
    check "$l: exit 1 ($rc)" is "$rc" 1
    check "$l: logged" has "$LOGF" "grimoire.sh: already running (launcher pid $holder), not starting another"
    check "$l: Grimoire not started" lacks "$SHIM_LOG" "$STARTED"
    check "$l: nothing else touched" sh -c '! grep -qE "lipc-|iptables|^stop |^start |killall -CONT" "$1"' sh "$SHIM_LOG"
    check "$l: awesome stopped again before the lock wait" has "$SHIM_LOG" "killall -STOP awesome"
    check "$l: the lock stays the other's" is "$(content "$GRIMOIRE_TMP/grimoire.launcher.pid")" "$holder"
    : >"$SHIM_LOG"
    : >"$LOGF"
    GRIMOIRE_WD_TICK=0.2 sh "$APP/grimoire.sh" 2>/dev/null
    rc=$?
    l="$s from the Library, another Grimoire running"
    check "$l: returns 0 ($rc)" is "$rc" 0
    check "$l: logged" has "$LOGF" "grimoire.sh: already running (launcher pid $holder), not starting another"
    check "$l: Grimoire not started" lacks "$SHIM_LOG" "$STARTED"
    check "$l: the lock stays the other's" is "$(content "$GRIMOIRE_TMP/grimoire.launcher.pid")" "$holder"
    kill "$holder" 2>/dev/null
    wait "$holder" 2>/dev/null
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
    ko_paused fake-koreader sh "$DOC" --hosted koreader --annotate 2>/dev/null
    check "$l --annotate: passed on to app/grimoire.sh, the page to Grimoire" sh -c \
        'grep -qxF "grimoire host=koreader" "$1" && grep -qxF "grimoire annotate=1" "$1"' sh "$SHIM_LOG"
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
    check "$s plugin: fresh install leaves KOReader alone" [ ! -e "$US/$PLUGIN_REL" ]
    check "$s plugin: fresh install has no consent marker" [ ! -e "$MARK" ]
    mkdir -p "$US/$PLUGIN_REL"
    cp "$APP/koreader/grimoire.koplugin/main.lua" "$US/$PLUGIN_REL/main.lua"
    start_doc
    check "$s plugin: explicitly copied plugin is maintained" plugin_is_payload
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
    echo keep >"$US/$PLUGIN_REL/README.user"
    start_doc
    check "$s plugin removed, extra file remains: not brought back" [ ! -e "$US/$PLUGIN_REL/main.lua" ]
    check "$s plugin removed: extra user file kept" has "$US/$PLUGIN_REL/README.user" keep
    echo "-- an older plugin" >"$US/$PLUGIN_REL/main.lua"
    start_doc
    check "$s plugin copied back by hand: kept up to date again" plugin_is_payload
    check "$s plugin copied back by hand: the marker holds the build" is "$(content "$MARK")" "$BUILD_ID"
    # KOReader uninstalled (KPM, say), then installed again: a start in between forgets the marker,
    # so the new KOReader gets the plugin, even though the old one's plugin was removed by hand.
    rm -rf "${US:?}/$PLUGIN_REL"
    start_doc
    check "$s plugin removed, then KOReader: the plugin marked removed first" is "$(content "$MARK")" removed
    rm -rf "${US:?}/koreader"
    : >"$SHIM_LOG"
    start_doc
    check "$s KOReader uninstalled: the marker forgotten" [ ! -e "$MARK" ]
    check "$s KOReader uninstalled: logged" has "$LOGF" \
        "Grimoire.sh: KOReader is no longer installed: plugin consent forgotten"
    check "$s KOReader uninstalled: Grimoire still starts" has "$SHIM_LOG" "grimoire exits"
    start_doc
    check "$s KOReader uninstalled: logged once only" lacks "$LOGF" "KOReader is no longer installed"
    mkdir -p "$US/koreader/plugins"
    : >"$US/koreader/reader.lua"
    start_doc
    check "$s KOReader installed again: needs consent again" [ ! -e "$US/$PLUGIN_REL/main.lua" ]
    check "$s KOReader installed again: no marker" [ ! -e "$MARK" ]
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
