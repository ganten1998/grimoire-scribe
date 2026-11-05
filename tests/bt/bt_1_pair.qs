# settings: shortcuts_enabled=1
# settings: controller=1
# Pairing a Bluetooth controller through the helper's API: tests/bt_check.sh stands a helper in
# (tests/fake_bthelper.py) and runs this (a helper is needed, so it is not in tests/scenarios). The helper is
# used as it is (not Grimoire's: it is never stopped), Settings > Pair > Pair a controller... finds two
# controllers (a name that is not printable shown safely), the second will not pair, the first does, then is
# forgotten; its Done goes back to Pair; Bluetooth turned off and on again there, and the helper going and
# coming back with shortcuts (Settings > Shortcuts' Enable shortcuts).
wait 2000
expect library.bluetooth is external
tap 1535 95
wait 200
tap library pair-menu
wait 200
expect library.sheet is pair-menu
tap library pair
wait 300
expect library.sheet is pair
wait 3000
expect library.pair is idle 2 0
shot 01-found
tap 930 1055
wait 2500
expect toast is Couldn't pair controller
expect library.pair is idle 2 0
tap 930 895
wait 300
expect library.pair is pairing 2 0
wait 2500
expect toast is Paired.
expect library.pair is idle 2 1
shot 02-paired
tap 930 1160
wait 1500
expect toast is Forgotten.
expect library.pair is idle 2 0
tap 930 1375
wait 200
expect library.sheet is pair-menu
tap library bluetooth
wait 200
expect settings.controller is 0
expect library.bluetooth is stopped
shot 03-bluetooth-off
# On again; then shortcuts off take it away, and back on bring it back
tap library bluetooth
wait 200
expect library.bluetooth is external
tap library done
wait 200
tap 1535 95
wait 200
tap library shortcuts
wait 200
tap library shortcuts.enable
wait 200
expect settings.shortcuts_enabled is 0
expect library.bluetooth is stopped
tap library shortcuts.enable
wait 200
expect library.bluetooth is external
quit
