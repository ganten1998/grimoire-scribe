#!/bin/sh
# Runs the Kindle launchers from the zip (make package) on this computer: documents/Grimoire.sh
# and the app/grimoire.sh it unpacks, in a scratch device tree, with the Kindle's tools replaced
# by stand-ins that record every call (tests/shims) and the binary by a shell script
# (tests/shims/fake-grimoire). Checks unpacking, --unpack-only, the KOReader guard,
# --reset-calibration, the order of the pause and the restore, the restore on a locked Kindle and
# the watchdog, once with each POSIX shell found (sh, dash, busybox sh).
# usage: tests/launcher_check.sh [build/dist/Grimoire.zip]
set -u
cd "$(dirname "$0")/.." || exit 1
REPO=$(pwd)
ZIP=${1:-build/dist/Grimoire.zip}
case "$ZIP" in /*) ;; *) ZIP=$REPO/$ZIP ;; esac
W=$(cd "$(dirname "$ZIP")/.." && pwd)/launcher-check
SHIMS=$REPO/tests/shims
export LC_ALL=C  # Grimoire.sh ends in a binary archive
unset KOREADER_DIR FAKE_MODE FAKE_EXIT_STATE SHIM_HOLD SHIM_NICE GRIMOIRE_WD_TICK GRIMOIRE_WD_FIRST GRIMOIRE_WD_STALE
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
    tests/launcher_check.sh tests/package_check.sh tests/shims/*; do
    check "sh -n $f" sh -n "$f"
    if command -v dash >/dev/null 2>&1; then check "dash -n $f" dash -n "$f"; fi
done
if command -v shellcheck >/dev/null 2>&1; then
    check "shellcheck" shellcheck -s sh -S warning kindle/Grimoire.sh kindle/app/grimoire.sh \
        "kindle/Grimoire - reset pen calibration.sh" tests/launcher_check.sh tests/package_check.sh tests/shims/*
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

# A fresh scratch Kindle: $T/us is /mnt/us, $T/tmp is /tmp, $T/sys holds /etc, /var and
# /dev/fb0 (a plain file), $T/state the stand-ins' answers, $T/calls.log every call they got.
tree() {
    T=$W/$SH_NAME/$1
    rm -rf "$T"
    mkdir -p "$T/us/documents" "$T/tmp" "$T/sys/etc/upstart" "$T/sys/var/tmp" "$T/sys/var/local/system" \
        "$T/sys/dev" "$T/state/lipc" "$T/state/pidof"
    US=$T/us
    APP=$US/grimoire/app
    DOC=$US/documents/Grimoire.sh
    RESET="$US/documents/Grimoire - reset pen calibration.sh"
    LOGF=$US/grimoire/grimoire.log
    ln "$W/Grimoire.sh" "$DOC" 2>/dev/null || cp "$W/Grimoire.sh" "$DOC"  # never written to
    cp "$W/reset.sh" "$RESET"
    : >"$T/sys/etc/upstart/volumd.conf"
    : >"$T/sys/etc/upstart/statusbar.conf"
    echo "Kindle 5.19.6 (test)" >"$T/sys/etc/prettyversion.txt"
    printf 'LANG=es_ES.UTF-8\nLC_ALL=es_ES.UTF-8\n' >"$T/sys/var/local/system/locale"
    printf 'KINDLE' >"$T/sys/dev/fb0"
    echo active >"$T/state/lipc/com.lab126.powerd.state"
    echo "screensaver splash" >"$T/state/lipc/com.lab126.blanket.load"
    GRIMOIRE_US=$US GRIMOIRE_TMP=$T/tmp GRIMOIRE_SYSROOT=$T/sys SHIM_LOG=$T/calls.log SHIM_STATE=$T/state
    export GRIMOIRE_US GRIMOIRE_TMP GRIMOIRE_SYSROOT SHIM_LOG SHIM_STATE
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
lines_in() { wc -l <"$1" | tr -d ' '; }
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

suite() {
    s="[$SH_NAME]"

    # First run on a fresh Kindle: unpacks, then starts app/grimoire.sh detached. The setsid
    # stand-in holds the launch until the fake binary is in place.
    tree first
    SHIM_HOLD=$T/go GRIMOIRE_WD_TICK=0.2 sh "$DOC"
    rc=$?
    check "$s first run returns 0 ($rc)" is "$rc" 0
    check "$s first run unpacks the app" sh -c '[ -f "$1/grimoire" ] && [ -f "$1/grimoire.sh" ] && [ -f "$1/CAPABILITIES" ]' sh "$APP"
    check "$s first run records build $BUILD_ID" is "$(content "$APP/BUILD_ID")" "$BUILD_ID"
    check "$s first run logs the unpack" has "$LOGF" "Grimoire.sh: unpacked build $BUILD_ID"
    check "$s app/CAPABILITIES" is "$(content "$APP/CAPABILITIES")" "guard unpack-only"
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
    check "$s inside KOReader: --hosted refused, no hosted mode yet ($rc)" is "$rc" 3
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

    # Locked when Grimoire exits: the saved screen stays off the lock screen, the rest is restored.
    # Also: the port from settings.ini, renice, and the diagnostics of a passcode, Special Offers
    # and an installed (not running) KOReader.
    tree locked
    sh "$DOC" --unpack-only
    fake
    printf 'http_port=18763\n' >"$US/grimoire/settings.ini"
    : >"$T/sys/var/local/system/userpasswdenabled"
    mkdir -p "$US/koreader"
    : >"$US/koreader/reader.lua"
    echo v2026.07.1 >"$US/koreader/git-rev"
    echo "screensaver ad_screensaver splash" >"$SHIM_STATE/lipc/com.lab126.blanket.load"
    : >"$SHIM_LOG"
    FAKE_EXIT_STATE=screenSaver SHIM_NICE=5 GRIMOIRE_WD_TICK=0.2 sh "$APP/grimoire.sh"
    rc=$?
    check "$s locked at exit: returns 0 ($rc)" is "$rc" 0
    check "$s locked at exit: the binary ran" has "$SHIM_LOG" "grimoire exits"
    check "$s locked at exit: the saved screen is not put back" is "$(content "$T/sys/dev/fb0")" GRIMOIRE
    check "$s locked at exit: the saved screen is dropped" [ ! -e "$T/sys/var/tmp/grimoire-fb.dump" ]
    check "$s locked at exit: logged" has "$LOGF" "grimoire.sh: power state at exit: screenSaver, so the saved screen is not put back"
    check "$s locked at exit: jobs, awesome, pillow and home still restored" in_order "$STARTED" "start statusbar" \
        "killall -CONT awesome" "$PILLOW_ON" "$HOME_UP"
    check "$s firewall hole on the port from settings.ini" in_order "iptables -A INPUT -i wlan0 -p tcp --dport 18763 -j ACCEPT" \
        "$STARTED" "iptables -D INPUT -i wlan0 -p tcp --dport 18763 -j ACCEPT"
    check "$s started niced: back to normal priority" has "$SHIM_LOG" "renice -n -5"
    check "$s log: passcode on" has "$LOGF" "grimoire.sh: device passcode: on"
    check "$s log: Special Offers" has "$LOGF" "grimoire.sh: blanket modules: screensaver ad_screensaver splash (Special Offers)"
    check "$s log: KOReader installed, not running" has "$LOGF" "grimoire.sh: KOReader: installed, version v2026.07.1"

    # Stop-framework mode: lab126_gui stopped and started; locked at exit only shows in the log.
    tree stopfw
    sh "$DOC" --unpack-only
    fake
    : >"$US/grimoire/stop-framework"
    : >"$SHIM_LOG"
    FAKE_EXIT_STATE=readyToSuspend GRIMOIRE_WD_TICK=0.2 sh "$APP/grimoire.sh"
    check "$s stop-framework: lab126_gui stopped, then started" in_order "stop lab126_gui" "$STARTED" "start lab126_gui"
    check "$s stop-framework: pillow and awesome left alone" sh -c '! grep -qE "disableEnablePillow|killall -(STOP|CONT) awesome" "$1"' sh "$SHIM_LOG"
    check "$s stop-framework: power state at exit logged" has "$LOGF" "grimoire.sh: power state at exit: readyToSuspend"

    # The watchdog: no first frame, then a heartbeat that stops; both killed, the UI restored.
    tree watchdog
    sh "$DOC" --unpack-only
    fake
    : >"$SHIM_LOG"
    FAKE_MODE=hang GRIMOIRE_WD_TICK=0.2 GRIMOIRE_WD_FIRST=1 sh "$APP/grimoire.sh"
    rc=$?
    check "$s watchdog: no first frame, killed" has "$LOGF" "grimoire.sh: watchdog: no first frame after 1s, killing Grimoire"
    check "$s watchdog: the Kindle UI restored after ($rc)" in_order "$STARTED" "$PILLOW_ON" "$HOME_UP"
    : >"$SHIM_LOG"
    FAKE_MODE=stale GRIMOIRE_WD_TICK=0.2 GRIMOIRE_WD_STALE=1 sh "$APP/grimoire.sh"
    check "$s watchdog: heartbeat stopped, killed" has "$LOGF" "grimoire.sh: watchdog: no heartbeat for"
    check "$s watchdog: the Kindle UI restored after that too" in_order "$STARTED" "$PILLOW_ON" "$HOME_UP"

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
    sh "$DOC" --unpack-only
    rc=$?
    check "$s Quire: left alone while the old app runs ($rc)" sh -c '[ "$1" = 0 ] && [ -d "$2/quire" ] && [ ! -e "$2/grimoire" ]' sh "$rc" "$US"
    rm -f "$T/tmp/quire.launcher.pid"
    sh "$DOC" --unpack-only
    check "$s Quire: the notebooks move over" sh -c '[ -f "$1/grimoire/notebooks/n1/notebook.ini" ] && [ ! -e "$1/quire" ]' sh "$US"
    check "$s Quire: its log goes on in grimoire.log" is "$(head -n 1 "$LOGF" 2>/dev/null)" "old log line"
    check "$s Quire: the old launcher is retired" [ ! -e "$US/documents/Quire.sh" ]
    check "$s Quire: then the app is unpacked" [ -f "$APP/grimoire" ]
}

ORIG_PATH=$PATH
for SH_NAME in sh dash busybox; do
    SH_BIN=$(command -v "$SH_NAME" 2>/dev/null)
    if [ -z "$SH_BIN" ]; then
        echo "skip the launchers under $SH_NAME (not installed)"
        continue
    fi
    # `sh` inside the launchers is this shell (busybox runs as sh when called so).
    mkdir -p "$W/$SH_NAME/bin"
    ln -s "$SH_BIN" "$W/$SH_NAME/bin/sh"
    PATH=$W/$SH_NAME/bin:$SHIMS:$ORIG_PATH
    export PATH
    suite
    PATH=$ORIG_PATH
done

# The scratch Kindles take room (an app in each): kept only when something failed.
if [ $fail = 0 ]; then
    rm -rf "$W"
    echo "LAUNCHER CHECK PASSED"
else
    echo "LAUNCHER CHECK FAILED (see $W)"
    exit 1
fi
